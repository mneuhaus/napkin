//! Exchange-rate download and on-disk cache for Napkin.
//!
//! Fiat rates come from <https://open.er-api.com> (daily, no key), crypto rates from the Coinbase
//! exchange-rates endpoint (no key), restricted to the tickers the engine knows. The merged table
//! is cached as JSON so the app and the CLI start offline with the last good rates.
//!
//! STUB: the signatures below are the contract the app and the CLI use; the bodies get replaced.

use std::path::{Path, PathBuf};
use std::time::Duration;

use napkin_core::ExchangeRates;

/// How old cached rates may get before [`RateCache::needs_refresh`] asks for a download.
pub const DEFAULT_MAX_AGE: Duration = Duration::from_secs(60 * 60);

/// A JSON file holding the last downloaded [`ExchangeRates`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateCache {
    path: PathBuf,
}

impl RateCache {
    /// A cache stored at `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The platform cache location (`~/Library/Caches/Napkin/rates.json` on macOS).
    pub fn default_location() -> Option<Self> {
        dirs::cache_dir().map(|dir| Self::new(dir.join("Napkin").join("rates.json")))
    }

    /// Where the cache file lives.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The cached rates, or `None` when there is no readable cache file.
    pub fn load(&self) -> Option<ExchangeRates> {
        None
    }

    /// The cached rates, falling back to the snapshot bundled with the engine.
    pub fn load_or_bundled(&self) -> ExchangeRates {
        self.load().unwrap_or_else(ExchangeRates::bundled)
    }

    /// Whether the cache is missing or older than `max_age` (by file modification time).
    pub fn needs_refresh(&self, max_age: Duration) -> bool {
        let _ = max_age;
        true
    }

    /// Downloads fresh rates (blocking), stores them in the cache and returns them.
    ///
    /// # Errors
    ///
    /// Returns [`RefreshError`] when both sources fail or the cache cannot be written.
    pub fn refresh(&self) -> Result<ExchangeRates, RefreshError> {
        Err(RefreshError::Unavailable("not implemented yet".to_owned()))
    }
}

/// Why downloading or caching rates failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RefreshError {
    /// No source answered with usable data.
    #[error("exchange rates unavailable: {0}")]
    Unavailable(String),
    /// The cache file could not be written.
    #[error("could not write the rate cache: {0}")]
    Io(#[from] std::io::Error),
}
