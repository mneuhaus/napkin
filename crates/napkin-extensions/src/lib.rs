//! Numi-compatible JavaScript extensions for Napkin.
//!
//! Extensions are `.js` files that call `numi.addUnit`, `numi.addFunction`, `numi.setVariable`
//! and `log`, exactly like Numi's
//! [plugin API](https://github.com/nikolaeu/numi/wiki#plugins), so existing Numi extensions run
//! unchanged.
//!
//! Scripts run in an embedded [QuickJS](https://bellard.org/quickjs/) engine without file system
//! or network access, each file in a context of its own (so two files may both declare
//! `let ratio`). The runtime lives on a dedicated thread; the [`CustomFunction`]s handed to the
//! engine are `Send + Sync` handles that call into it, with a time limit per call.
//!
//! ```
//! use napkin_core::ExtValue;
//!
//! # fn main() -> Result<(), napkin_extensions::LoadError> {
//! let extension = napkin_extensions::load_source(
//!     "zum.js",
//!     r#"numi.addFunction({ "id": "zum", "phrases": "zum" }, function(values) {
//!            return { "double": values[0].double + values[1].double };
//!        });"#,
//! )?;
//! let zum = &extension.functions[0];
//! let args = [ExtValue { double: 2.0, unit_id: None }, ExtValue { double: 3.0, unit_id: None }];
//! assert_eq!((zum.call)(&args), Ok(ExtValue { double: 5.0, unit_id: None }));
//! # Ok(())
//! # }
//! ```

mod numi_api;
mod worker;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use napkin_core::{CustomFunction, CustomUnit, Engine, ExtValue};

use crate::worker::{LoadOutcome, Loaded, Worker};

/// Numi's application-support folder names: the direct download and the Setapp edition.
const NUMI_APP_FOLDERS: [&str; 2] = ["com.dmitrynikolaev.numi", "com.nikolaeu.numi-setapp"];

/// Napkin's own extension folder (`~/Library/Application Support/Napkin/extensions` on macOS).
pub fn napkin_extension_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("Napkin").join("extensions"))
}

/// Napkin's folder plus Numi's extension folders (direct download and Setapp) that exist.
pub fn default_extension_dirs() -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = napkin_extension_dir().into_iter().collect();
    if let Some(data_dir) = dirs::data_dir() {
        folders.extend(
            NUMI_APP_FOLDERS
                .iter()
                .map(|app| data_dir.join(app).join("extensions"))
                .filter(|dir| dir.is_dir()),
        );
    }
    folders
}

/// Loads every `*.js` file in `dirs` (sorted by file name, missing folders skipped) into `engine`.
/// One broken extension never stops the others.
///
/// Folders are visited in the given order, files within a folder by file name. A file whose
/// script throws (or does not compile) is listed in [`LoadReport::failed`] and registers nothing.
/// Registrations the engine rejects (a duplicate id, an unknown base unit) do not fail the file;
/// they are reported in [`LoadReport::log`] as warnings.
pub fn load_into(engine: &mut Engine, dirs: &[PathBuf]) -> LoadReport {
    let mut report = LoadReport::default();
    let mut files = Vec::new();
    for (index, dir) in dirs.iter().enumerate() {
        if dirs[..index].contains(dir) {
            continue;
        }
        match extension_files(dir) {
            Ok(found) => files.extend(found),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => report
                .failed
                .push((dir.clone(), format!("could not list the folder: {error}"))),
        }
    }
    if files.is_empty() {
        return report;
    }
    let worker = match Worker::spawn() {
        Ok(worker) => worker,
        Err(error) => {
            report
                .failed
                .extend(files.into_iter().map(|file| (file, error.clone())));
            return report;
        }
    };
    for file in files {
        let name = display_name(&file);
        let (result, log) = load_with(&worker, &file);
        report
            .log
            .extend(log.into_iter().map(|line| format!("{name}: {line}")));
        match result {
            Ok(extension) => {
                let rejected = extension.register_into(engine);
                report.log.extend(
                    rejected
                        .into_iter()
                        .map(|warning| format!("{name}: warning: {warning}")),
                );
                report.loaded.push(file);
            }
            Err(error) => report.failed.push((file, error.to_string())),
        }
    }
    report
}

/// Runs one extension file and returns what it declared, without registering anything.
///
/// # Errors
///
/// Returns [`LoadError`] when the file cannot be read or its script fails.
pub fn load_file(path: &Path) -> Result<Extension, LoadError> {
    let source = read_source(path)?;
    load_source(&display_name(path), &source)
}

/// Runs extension source code and returns what it declared, without registering anything.
/// `name` shows up in error locations (`sample.js:3:5`).
///
/// # Errors
///
/// Returns [`LoadError`] when the script throws, does not compile, calls the `numi` API with
/// invalid arguments, or the JavaScript runtime cannot start.
pub fn load_source(name: &str, source: &str) -> Result<Extension, LoadError> {
    let worker = Worker::spawn().map_err(LoadError::Runtime)?;
    let LoadOutcome { result, log } = worker.load(name, source.to_owned());
    result
        .map(|loaded| Extension::from_loaded(&worker, loaded, log))
        .map_err(LoadError::Script)
}

/// What one extension declared: units, functions and global variables, plus its `log` output.
///
/// The functions stay callable for as long as any of them (or a clone) is alive, also after
/// being registered with an [`Engine`].
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Extension {
    /// Units from `numi.addUnit`, in call order.
    pub units: Vec<CustomUnit>,
    /// Functions from `numi.addFunction`, in call order.
    pub functions: Vec<CustomFunction>,
    /// Variables from `numi.setVariable`, in call order.
    pub variables: Vec<(String, ExtValue)>,
    /// Lines passed to `log(...)` (or `console.log`) while the script loaded.
    pub log: Vec<String>,
}

impl Extension {
    /// Registers everything with `engine`: units first, then functions, then variables. Returns
    /// one message per registration the engine rejected; the others still take effect.
    pub fn register_into(self, engine: &mut Engine) -> Vec<String> {
        let mut rejected = Vec::new();
        for unit in self.units {
            let id = unit.id.clone();
            if let Err(error) = engine.register_unit(unit) {
                rejected.push(format!("unit {id:?} not registered: {error}"));
            }
        }
        for function in self.functions {
            let id = function.id.clone();
            if let Err(error) = engine.register_function(function) {
                rejected.push(format!("function {id:?} not registered: {error}"));
            }
        }
        for (name, value) in self.variables {
            if let Err(error) = engine.set_global_variable(&name, value) {
                rejected.push(format!("variable {name:?} not set: {error}"));
            }
        }
        rejected
    }

    fn from_loaded(worker: &Worker, loaded: Loaded, log: Vec<String>) -> Self {
        let functions = loaded
            .functions
            .into_iter()
            .map(|function| {
                let worker = worker.clone();
                let handle = function.handle;
                CustomFunction {
                    id: function.id,
                    phrases: function.phrases,
                    call: Arc::new(move |args: &[ExtValue]| worker.call(handle, args)),
                }
            })
            .collect();
        Self {
            units: loaded.units,
            functions,
            variables: loaded.variables,
            log,
        }
    }
}

/// Why an extension could not be loaded.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LoadError {
    /// The file could not be read or is not UTF-8.
    #[error("could not read the file: {0}")]
    Read(#[from] io::Error),
    /// The script threw, did not compile, ran too long, or misused the `numi` API.
    #[error("{0}")]
    Script(String),
    /// The JavaScript runtime could not be started or stopped unexpectedly.
    #[error("{0}")]
    Runtime(String),
}

/// What happened while loading extensions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadReport {
    /// Files that ran without error.
    pub loaded: Vec<PathBuf>,
    /// Files that failed, with the reason (also unreadable folders).
    pub failed: Vec<(PathBuf, String)>,
    /// Lines passed to `log(...)`, prefixed with the file name, and warnings about registrations
    /// the engine rejected (`file.js: warning: unit "x" not registered: ...`).
    pub log: Vec<String>,
}

impl LoadReport {
    /// Whether `path` loaded successfully.
    pub fn is_loaded(&self, path: &Path) -> bool {
        self.loaded.iter().any(|loaded| loaded == path)
    }
}

/// Loads one file on a shared worker. The log comes back even when the script failed.
fn load_with(worker: &Worker, path: &Path) -> (Result<Extension, LoadError>, Vec<String>) {
    let source = match read_source(path) {
        Ok(source) => source,
        Err(error) => return (Err(error), Vec::new()),
    };
    let LoadOutcome { result, log } = worker.load(&display_name(path), source);
    match result {
        Ok(loaded) => (Ok(Extension::from_loaded(worker, loaded, Vec::new())), log),
        Err(error) => (Err(LoadError::Script(error)), log),
    }
}

/// The `*.js` files directly inside `dir`, sorted by file name.
fn extension_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let is_script = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("js"));
        if is_script && path.is_file() {
            files.push(path);
        }
    }
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(files)
}

fn read_source(path: &Path) -> Result<String, LoadError> {
    let bytes = fs::read(path)?;
    let source =
        String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(source
        .strip_prefix('\u{feff}')
        .map(str::to_owned)
        .unwrap_or(source))
}

fn display_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_handles_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Extension>();
        assert_send_sync::<LoadError>();
    }

    #[test]
    fn default_dirs_start_with_napkins_folder_and_list_only_existing_numi_folders() {
        let dirs = default_extension_dirs();
        assert_eq!(dirs.first(), napkin_extension_dir().as_ref());
        assert!(dirs.iter().skip(1).all(|dir| dir.is_dir()), "{dirs:?}");
    }

    #[test]
    fn strips_a_byte_order_mark() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().join("bom.js");
        fs::write(&path, "\u{feff}log(1)").expect("write");
        assert_eq!(read_source(&path).expect("readable"), "log(1)");
    }
}
