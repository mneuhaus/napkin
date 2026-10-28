# Worklog: host crates (napkin-rates, napkin-extensions, napkin-cli, Alfred)

Branch `host`, worktree `.worktrees/host`. Engine (`napkin-core`) is a stub on this branch.

## Done

- **napkin-rates**: parallel blocking download (ureq 3, rustls, 10 s global timeout) of
  open.er-api.com (fiat) and Coinbase (crypto), merge over the last cache or the bundled snapshot,
  atomic cache writes (temp file + fsync + rename), `needs_refresh` by mtime (also true for an
  unusable file). 18 unit tests on recorded fixtures (`tests/fixtures/`), 1 ignored live test (passes).

## Next

- napkin-extensions (QuickJS host).
- napkin-cli, Alfred workflow.

## Decisions

- Cache file = serialized `ExchangeRates` (same shape as the bundled snapshot), pretty JSON.
- Merge = fallback table (cache, else bundled) overlaid with fresh fiat, then with fresh crypto.
  Entries a fresh source stops listing keep their fallback value. `USD` pinned to 1.
  `updated` = fiat publication time when fiat is fresh, else the fallback's (age of the oldest part).
- Crypto tickers taken from Coinbase = bundled tickers not listed by the fresh fiat table. When
  the fiat source is down, a 38-entry `KNOWN_CRYPTO` list decides (a test keeps it equal to
  "bundled minus fiat fixture"), so Coinbase's fiat rates are never used.
- `serde_json/float_roundtrip` enabled so cached rates read back bit-for-bit (feature unifies
  workspace-wide; harmless).
- Added public `RateCache::store` (atomic write), useful for the app too.
- `cargo clippy -p <crate>` also lints the path dependency `napkin-core`, whose stub fails two
  pedantic lints (`unused_self`, `missing_panics_doc`) on this branch. My gate uses `--no-deps`;
  the real engine replaces that stub.
