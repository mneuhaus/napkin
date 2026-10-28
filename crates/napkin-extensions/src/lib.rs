//! Numi-compatible JavaScript extensions for Napkin.
//!
//! Extensions are `.js` files that call `numi.addUnit`, `numi.addFunction`, `numi.setVariable`
//! and `log`, exactly like Numi's
//! [plugin API](https://github.com/nikolaeu/numi/wiki#plugins), so existing Numi extensions run
//! unchanged.
//!
//! STUB: the signatures below are the contract the app and the CLI use; the bodies get replaced.

use std::path::{Path, PathBuf};

use napkin_core::Engine;

/// Napkin's own extension folder (`~/Library/Application Support/Napkin/extensions` on macOS).
pub fn napkin_extension_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("Napkin").join("extensions"))
}

/// Napkin's folder plus Numi's extension folders (direct download and Setapp) that exist.
pub fn default_extension_dirs() -> Vec<PathBuf> {
    napkin_extension_dir().into_iter().collect()
}

/// Loads every `*.js` file in `dirs` (sorted by file name, missing folders skipped) into `engine`.
/// One broken extension never stops the others.
pub fn load_into(engine: &mut Engine, dirs: &[PathBuf]) -> LoadReport {
    let _ = (engine, dirs);
    LoadReport::default()
}

/// What happened while loading extensions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadReport {
    /// Files that ran without error.
    pub loaded: Vec<PathBuf>,
    /// Files that failed, with the reason.
    pub failed: Vec<(PathBuf, String)>,
    /// Lines passed to `log(...)`, prefixed with the file name.
    pub log: Vec<String>,
}

impl LoadReport {
    /// Whether `path` loaded successfully.
    pub fn is_loaded(&self, path: &Path) -> bool {
        self.loaded.iter().any(|loaded| loaded == path)
    }
}
