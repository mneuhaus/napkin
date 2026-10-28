//! Exchange-rate download and on-disk cache for Napkin.
//!
//! Fiat rates come from <https://open.er-api.com> (daily, no key), crypto rates from the Coinbase
//! exchange-rates endpoint (no key), restricted to the tickers the engine knows. The merged table
//! is cached as JSON so the app and the CLI start offline with the last good rates.
//!
//! ```no_run
//! use napkin_rates::{RateCache, DEFAULT_MAX_AGE};
//!
//! # fn main() -> Result<(), napkin_rates::RefreshError> {
//! let cache = RateCache::new("rates.json");
//! let rates = if cache.needs_refresh(DEFAULT_MAX_AGE) { cache.refresh()? } else { cache.load_or_bundled() };
//! assert_eq!(rates.base, "USD");
//! # Ok(())
//! # }
//! ```

mod download;
mod sources;

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use napkin_core::ExchangeRates;

/// How old cached rates may get before [`RateCache::needs_refresh`] asks for a download.
pub const DEFAULT_MAX_AGE: Duration = Duration::from_secs(60 * 60);

/// A JSON file holding the last downloaded [`ExchangeRates`].
///
/// The file is the serialized [`ExchangeRates`] (`base`, `updated`, `rates`), the same shape as
/// the snapshot bundled with the engine.
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

    /// The cached rates, or `None` when there is no readable cache file or it does not hold a
    /// USD-based table of finite, positive rates.
    pub fn load(&self) -> Option<ExchangeRates> {
        let json = fs::read_to_string(&self.path).ok()?;
        let rates: ExchangeRates = serde_json::from_str(&json).ok()?;
        sources::is_valid_table(&rates).then_some(rates)
    }

    /// The cached rates, falling back to the snapshot bundled with the engine.
    pub fn load_or_bundled(&self) -> ExchangeRates {
        self.load().unwrap_or_else(ExchangeRates::bundled)
    }

    /// Whether the cache is missing, unusable, or older than `max_age` (by file modification time).
    pub fn needs_refresh(&self, max_age: Duration) -> bool {
        let Ok(modified) = fs::metadata(&self.path).and_then(|metadata| metadata.modified()) else {
            return true;
        };
        // A modification time in the future (clock changes) counts as fresh.
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::ZERO);
        age > max_age || self.load().is_none()
    }

    /// Downloads fresh rates (blocking), stores them in the cache and returns them.
    ///
    /// Both sources are fetched in parallel with a timeout of about ten seconds each. When one
    /// source fails, the other one's fresh rates are kept and the rest comes from the cache (or
    /// the bundled snapshot), so a flaky crypto API never discards a good fiat download.
    ///
    /// # Errors
    ///
    /// Returns [`RefreshError`] when both sources fail or the cache cannot be written.
    pub fn refresh(&self) -> Result<ExchangeRates, RefreshError> {
        let (fiat, crypto) = download::fetch_both(sources::FIAT_URL, sources::CRYPTO_URL);
        self.refresh_from(fiat, crypto)
    }

    /// Writes `rates` to the cache atomically: a temporary file in the cache folder is written,
    /// flushed and renamed over the old cache, so readers never see a half-written file. Creates
    /// the folder when needed.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the folder or the file cannot be written.
    pub fn store(&self, rates: &ExchangeRates) -> io::Result<()> {
        /// Distinguishes concurrent writers within one process; the process id covers the rest.
        static WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

        let file_name = self.path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} has no file name", self.path.display()),
            )
        })?;
        let dir = self
            .path
            .parent()
            .filter(|dir| !dir.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(dir)?;
        let temp = dir.join(format!(
            ".{}.{}-{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            WRITE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let json = serde_json::to_vec_pretty(rates).map_err(io::Error::other)?;
        let written = write_then_rename(&temp, &self.path, &json);
        if written.is_err() {
            // Best effort: the temporary file may not even exist.
            let _ = fs::remove_file(&temp);
        }
        written
    }

    /// The download-independent part of [`RateCache::refresh`]: parses the two response bodies
    /// (or download errors), merges them over the cache and stores the result.
    fn refresh_from(
        &self,
        fiat_body: Result<String, String>,
        crypto_body: Result<String, String>,
    ) -> Result<ExchangeRates, RefreshError> {
        let fiat = fiat_body.and_then(|body| sources::parse_fiat(&body));
        let crypto = crypto_body.and_then(|body| sources::parse_crypto(&body));
        if let (Err(fiat_error), Err(crypto_error)) = (&fiat, &crypto) {
            return Err(RefreshError::Unavailable(format!(
                "open.er-api.com: {fiat_error}; api.coinbase.com: {crypto_error}"
            )));
        }
        let rates = sources::merge(self.load_or_bundled(), fiat.ok(), crypto.ok());
        self.store(&rates)?;
        Ok(rates)
    }
}

fn write_then_rename(temp: &Path, target: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temp, target)
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use chrono::DateTime;
    use tempfile::TempDir;

    const FIAT_FIXTURE: &str = include_str!("../tests/fixtures/open-er-api-usd.json");
    const CRYPTO_FIXTURE: &str = include_str!("../tests/fixtures/coinbase-usd.json");

    fn cache_in(dir: &TempDir) -> RateCache {
        RateCache::new(dir.path().join("Napkin").join("rates.json"))
    }

    fn sample_rates() -> ExchangeRates {
        ExchangeRates {
            base: "USD".to_owned(),
            updated: DateTime::from_timestamp(1_700_000_000, 0),
            rates: BTreeMap::from([
                ("USD".to_owned(), 1.0),
                ("EUR".to_owned(), 0.5),
                ("BTC".to_owned(), 0.123),
            ]),
        }
    }

    fn set_age(path: &Path, age: Duration) {
        let file = File::options().write(true).open(path).expect("cache file exists");
        file.set_modified(SystemTime::now() - age)
            .expect("mtime can be set");
    }

    #[test]
    fn store_and_load_round_trip_without_leftovers() {
        let dir = TempDir::new().expect("temp dir");
        let cache = cache_in(&dir);
        cache.store(&sample_rates()).expect("store succeeds");
        cache.store(&sample_rates()).expect("overwriting succeeds");
        assert_eq!(cache.load(), Some(sample_rates()));
        let files: Vec<_> = fs::read_dir(cache.path().parent().expect("has a parent"))
            .expect("folder exists")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(files, vec!["rates.json"], "no temporary files are left behind");
    }

    #[test]
    fn unusable_cache_files_load_as_none() {
        let dir = TempDir::new().expect("temp dir");
        let cache = cache_in(&dir);
        assert_eq!(cache.load(), None, "missing");
        fs::create_dir_all(cache.path().parent().expect("has a parent")).expect("mkdir");
        for content in [
            "",
            "not json",
            r#"{"base":"USD","rates":{}}"#,
            r#"{"base":"EUR","rates":{"USD":1}}"#,
        ] {
            fs::write(cache.path(), content).expect("write");
            assert_eq!(cache.load(), None, "content {content:?}");
            assert_eq!(
                cache.load_or_bundled(),
                ExchangeRates::bundled(),
                "content {content:?}"
            );
        }
    }

    #[test]
    fn needs_refresh_follows_the_file_age() {
        let dir = TempDir::new().expect("temp dir");
        let cache = cache_in(&dir);
        assert!(cache.needs_refresh(DEFAULT_MAX_AGE), "missing cache");
        cache.store(&sample_rates()).expect("store");
        assert!(!cache.needs_refresh(DEFAULT_MAX_AGE), "fresh cache");
        set_age(cache.path(), Duration::from_secs(2 * 60 * 60));
        assert!(cache.needs_refresh(DEFAULT_MAX_AGE), "two hours old");
        assert!(
            !cache.needs_refresh(Duration::from_secs(3 * 60 * 60)),
            "younger than a custom max age"
        );
        fs::write(cache.path(), "garbage").expect("write");
        assert!(cache.needs_refresh(DEFAULT_MAX_AGE), "fresh but unusable");
    }

    #[test]
    fn refresh_merges_both_sources_and_writes_the_cache() {
        let dir = TempDir::new().expect("temp dir");
        let cache = cache_in(&dir);
        let rates = cache
            .refresh_from(Ok(FIAT_FIXTURE.to_owned()), Ok(CRYPTO_FIXTURE.to_owned()))
            .expect("both sources work");
        assert!(rates.rates.contains_key("EUR") && rates.rates.contains_key("BTC"));
        assert_ne!(
            rates.rates["BTC"],
            ExchangeRates::bundled().rates["BTC"],
            "BTC comes from the fixture"
        );
        assert_eq!(cache.load(), Some(rates));
    }

    #[test]
    fn refresh_fills_a_failed_source_from_the_cache() {
        let dir = TempDir::new().expect("temp dir");
        let cache = cache_in(&dir);
        cache.store(&sample_rates()).expect("store");

        let rates = cache
            .refresh_from(Ok(FIAT_FIXTURE.to_owned()), Err("timed out".to_owned()))
            .expect("fiat alone is enough");
        assert_eq!(rates.rates["BTC"], 0.123, "crypto from the cache");
        assert!((rates.rates["EUR"] - 0.5).abs() > 0.01, "fiat is fresh");

        cache.store(&sample_rates()).expect("store");
        let rates = cache
            .refresh_from(
                Ok("<html>maintenance</html>".to_owned()),
                Ok(CRYPTO_FIXTURE.to_owned()),
            )
            .expect("crypto alone is enough");
        assert_eq!(rates.rates["EUR"], 0.5, "fiat from the cache");
        assert!((rates.rates["BTC"] - 0.123).abs() > 0.01, "crypto is fresh");
        assert_eq!(cache.load(), Some(rates));
    }

    #[test]
    fn refresh_fills_from_the_bundled_snapshot_without_a_cache() {
        let dir = TempDir::new().expect("temp dir");
        let cache = cache_in(&dir);
        let rates = cache
            .refresh_from(Err("offline".to_owned()), Ok(CRYPTO_FIXTURE.to_owned()))
            .expect("crypto works");
        assert_eq!(rates.rates["EUR"], ExchangeRates::bundled().rates["EUR"]);
    }

    #[test]
    fn refresh_fails_and_keeps_the_cache_when_both_sources_fail() {
        let dir = TempDir::new().expect("temp dir");
        let cache = cache_in(&dir);
        cache.store(&sample_rates()).expect("store");
        let error = cache
            .refresh_from(Err("dns error".to_owned()), Ok("{}".to_owned()))
            .expect_err("nothing usable");
        assert!(matches!(error, RefreshError::Unavailable(_)), "{error:?}");
        let message = error.to_string();
        assert!(
            message.contains("dns error") && message.contains("api.coinbase.com"),
            "{message}"
        );
        assert_eq!(cache.load(), Some(sample_rates()));
    }

    #[test]
    fn refresh_reports_an_unwritable_cache() {
        let dir = TempDir::new().expect("temp dir");
        let blocker = dir.path().join("file");
        fs::write(&blocker, "").expect("write");
        let cache = RateCache::new(blocker.join("rates.json"));
        let error = cache
            .refresh_from(Ok(FIAT_FIXTURE.to_owned()), Ok(CRYPTO_FIXTURE.to_owned()))
            .expect_err("a file blocks the folder");
        assert!(matches!(error, RefreshError::Io(_)), "{error:?}");
    }

    #[test]
    #[ignore = "downloads from open.er-api.com and api.coinbase.com"]
    fn live_refresh() {
        let dir = TempDir::new().expect("temp dir");
        let cache = cache_in(&dir);
        let rates = cache.refresh().expect("both sources answer");
        assert!(rates.updated.is_some());
        assert!(rates.rates["EUR"] > 0.0 && rates.rates["BTC"] > 0.0);
        assert!(!cache.needs_refresh(DEFAULT_MAX_AGE));
    }
}
