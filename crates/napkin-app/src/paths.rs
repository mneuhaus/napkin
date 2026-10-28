//! Where Napkin keeps its files. `NAPKIN_HOME=<dir>` moves everything (notes, settings, window
//! state, rate cache, extensions, login item) into one folder, for tests and screenshots.

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use napkin_rates::RateCache;

/// Environment variable that relocates every file Napkin reads or writes.
pub(crate) const HOME_VARIABLE: &str = "NAPKIN_HOME";

/// The folders and files the app uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppPaths {
    /// `~/Library/Application Support/Napkin`, or `$NAPKIN_HOME`.
    support: PathBuf,
    /// The rate cache file.
    rates: PathBuf,
    /// Folder for the launch-at-login agent plist.
    launch_agents: PathBuf,
    /// Whether `NAPKIN_HOME` is in effect (nothing outside it is touched).
    sandboxed: bool,
}

impl AppPaths {
    /// Resolves the paths from `NAPKIN_HOME` or the platform folders.
    pub(crate) fn resolve() -> anyhow::Result<Self> {
        if let Some(home) = std::env::var_os(HOME_VARIABLE).filter(|value| !value.is_empty()) {
            let home = absolute(PathBuf::from(home))?;
            return Ok(Self::in_folder(&home));
        }
        let support = dirs::data_dir()
            .context("no application support folder")?
            .join("Napkin");
        let rates = RateCache::default_location()
            .map_or_else(|| support.join("rates.json"), |cache| cache.path().to_path_buf());
        let launch_agents = dirs::home_dir()
            .context("no home folder")?
            .join("Library")
            .join("LaunchAgents");
        Ok(Self {
            support,
            rates,
            launch_agents,
            sandboxed: false,
        })
    }

    /// Paths rooted in `home`, as with `NAPKIN_HOME`.
    pub(crate) fn in_folder(home: &Path) -> Self {
        Self {
            support: home.to_path_buf(),
            rates: home.join("cache").join("rates.json"),
            launch_agents: home.join("LaunchAgents"),
            sandboxed: true,
        }
    }

    /// The default notes folder (settings can point elsewhere).
    pub(crate) fn default_notes_dir(&self) -> PathBuf {
        self.support.join("notes")
    }

    pub(crate) fn settings_file(&self) -> PathBuf {
        self.support.join("settings.json")
    }

    /// Window position and the last open note.
    pub(crate) fn state_file(&self) -> PathBuf {
        self.support.join("state.json")
    }

    pub(crate) fn rate_cache(&self) -> RateCache {
        RateCache::new(&self.rates)
    }

    /// Napkin's own extension folder.
    pub(crate) fn extension_dir(&self) -> PathBuf {
        self.support.join("extensions")
    }

    /// Napkin's extension folder plus Numi's, unless sandboxed by `NAPKIN_HOME`.
    pub(crate) fn extension_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![self.extension_dir()];
        if self.sandboxed {
            return dirs;
        }
        for dir in napkin_extensions::default_extension_dirs() {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
        dirs
    }

    pub(crate) fn launch_agents_dir(&self) -> &Path {
        &self.launch_agents
    }
}

fn absolute(path: PathBuf) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }
    Ok(std::env::current_dir().context("no current folder")?.join(path))
}
