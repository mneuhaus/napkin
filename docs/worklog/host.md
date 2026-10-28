# Worklog: host crates (napkin-rates, napkin-extensions, napkin-cli, Alfred)

Branch `host`, worktree `.worktrees/host`. Engine (`napkin-core`) is a stub on this branch.

## Done

- **napkin-rates**: parallel blocking download (ureq 3, rustls, 10 s global timeout) of
  open.er-api.com (fiat) and Coinbase (crypto), merge over the last cache or the bundled snapshot,
  atomic cache writes (temp file + fsync + rename), `needs_refresh` by mtime (also true for an
  unusable file). 18 unit tests on recorded fixtures (`tests/fixtures/`), 1 ignored live test (passes).
- **napkin-extensions**: QuickJS (rquickjs 0.14, default features) on a dedicated worker thread;
  `numi.addUnit/addFunction/setVariable`, `log`, `console.log`; one context per file; limits
  (2 s load, 500 ms per call, 64 MB heap, 1 MB JS stack); public `load_source`, `load_file`,
  `Extension { units, functions, variables, log }`, `Extension::register_into`, `LoadError`.
  Tests: all 17 Numi fixtures (sample.js + 16 community extensions, MIT notice in
  `tests/fixtures/numi/NUMI-LICENSE.txt`) plus 15 host behaviour tests, 3 unit tests, 1 doc test.
- **napkin-cli** (`napkin`): one-shot (`napkin "20 inches in cm"`, exit 1 + `napkin: msg` on
  stderr), document from stdin or `-f` (`line  → result`, `line  → error: msg`, bare line for
  none), `--results-only`, `--json`, `--refresh-rates`, `--no-extensions`, `--precision 0-10`,
  plus `--extensions-dir DIR` (repeatable), `--rates-cache FILE`, `-v/--verbose`; `serve
  [--port 15055]` on 127.0.0.1 via tiny_http. 6 unit + 16 integration tests (binary run with an
  isolated `$HOME`, incl. a real HTTP round trip and a closed stdout pipe).
- **integrations/alfred**: `info.plist` (script filters `=` without space and `napkin` with space,
  clipboard output), `script-filter.js` (JXA, runs `napkin --json -- <query>`), README with
  install steps. `plutil -lint` OK; script filter exercised directly with `osascript`.

## Gate (final)

`cargo fmt --check` OK; `cargo clippy -p napkin-rates -p napkin-extensions -p napkin-cli
--all-targets --no-deps -- -D warnings` OK; `cargo test` for the three crates: 77 passed,
1 ignored (live rates, passes with `--ignored`); `cargo doc --no-deps` for both libs, no warnings.
Without `--no-deps`, clippy stops in the `napkin-core` stub (see below).

## Next / known gaps

- Alfred itself is not installed on this machine: the workflow was not imported into Alfred.
- `log()` output from inside custom functions (at evaluation time) is dropped; only load-time
  logs reach `LoadReport::log`.
- `napkin serve` loads rates once at start (no background refresh; `--refresh-rates` at start).
- Engine-side: accept `base_unit_id == id` for base-less units; real engine replaces the stub, then
  rerun the CLI tests (they only rely on plain numbers and non-numeric words).

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
- **rquickjs Send/Sync**: `Persistent<T>` holds a raw `*mut JSRuntime` and is never `Send`, even
  with the `parallel` feature, so `parallel` alone cannot make `ExtFunction` closures
  `Send + Sync` without `unsafe impl Send`. Instead one worker thread owns the runtime, all
  contexts and saved functions; `ExtFunction` captures only an `mpsc::Sender<Job>` (Send + Sync)
  and a function index, sends `Call` jobs and waits on a reply channel (10 s watchdog). No
  `unsafe` anywhere. The thread ends when the last handle is dropped; `Host` field order drops
  persistent functions and contexts before the runtime (QuickJS aborts otherwise).
- One `Context` per file: Numi community extensions redeclare top-level `let ratio`, `function
  fact` etc.; a shared global scope would make the second file fail or clobber the first.
- Scripts run as sloppy-mode global scripts (rquickjs defaults to strict).
- A file whose script throws registers nothing (atomic per file) and lands in `failed`; its
  `log` lines are still reported. Engine `RegisterError`s (duplicate id, unknown base) do not fail
  the file: they go to `LoadReport::log` as `file.js: warning: ...` (kept `LoadReport`'s fields
  unchanged instead of adding a `warnings` field, which could break struct literals in the app).
- Structurally invalid `numi.*` calls (missing id/phrases, non-function) throw a `TypeError`.
- `addUnit` defaults: `baseUnitId` missing -> the unit's own id (a new dimension; needed by
  Electrical-conversion.js where `watt` has no base and `kilowatt` is based on `watt`),
  `format` missing -> first phrase, `ratio` missing -> 1. `phrases` may also be an array; numeric
  strings are accepted for `ratio`/`double`. **Engine needs to accept `base_unit_id == id`.**
- Values passed to JS: `{double, unitId}` with `unitId` omitted for plain numbers. Function
  results must be a finite number or `{double, unitId?}`; `undefined`/`null`/NaN/strings/Promises
  become `Err`.
- `numi.*` calls after loading (from inside a custom function) throw; `log` calls then are dropped
  (no channel to the user at evaluation time).
- Folders deduplicated, visited in order, files sorted by name, only top-level `*.js`
  (case-insensitive), BOM stripped, missing folders skipped silently, unreadable folders reported.
- CLI: no `args_conflicts_with_subcommands`, so global flags work before `serve`
  (`napkin --no-extensions serve`) and `napkin 5 serve` is still an expression;
  `allow_negative_numbers` makes `napkin -5` an expression. Document mode always exits 0 (errors
  are part of the output); `--results-only` prints a blank row for errors, like the app. JSON:
  `{"input","result":{"text","number","unit"}|null,"error":str|null}` per line, an array per
  document. A final newline does not create an extra empty line. Broken pipe exits 0. No
  expression and stdin is a terminal: help on stderr, exit 2. Extension failures are always
  warned on stderr, logs and rejected registrations only with `--verbose`.
- HTTP: 200 + result text, 422 + error message, 400 without `q`, 405 for non-GET/HEAD; any path;
  `+` stays a plus sign (an operator; Numi's workflow encodes spaces as `%20`).
- `cargo clippy -p <crate>` also lints the path dependency `napkin-core`, whose stub fails two
  pedantic lints (`unused_self`, `missing_panics_doc`) on this branch. My gate uses `--no-deps`;
  the real engine replaces that stub.
