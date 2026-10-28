# Engine worklog (napkin-core)

Branch `engine`, worktree `.worktrees/engine`. Scope: `crates/napkin-core/` only.

## Layout

Modules in `crates/napkin-core/src/`:

- `units.rs`: built-in unit table, SI/IEC prefixes, dimensions, compound units (`Unit` = product
  of `UnitRef^power`).
- `currency.rs`: ISO codes, names, signs, crypto tickers, case rules.
- `zones.rs`: time zone lookup (IANA names, cities, aliases, abbreviations), `Zone` (named or
  system local).
- `lexer.rs`: raw tokens of one line (numbers per number style, dates/times, words, symbols).
- `classify.rs`: context-sensitive classification of words into semantic tokens + span kinds.
- `parser.rs`: Pratt parser (implicit multiplication, unit attachment, conversions, percent
  phrases) producing `Expr`.
- `value.rs`: `Quantity`, `Moment`, `Units` (dimension/factor/conversion), display + answers.
- `eval.rs`: evaluator (arithmetic, percent rules, date math, doc refs, function calls).
- `functions.rs`: built-in functions.
- `format.rs`: number, scientific, radix, currency formatting.
- `document.rs`: line semantics (headers, comments, labels, assignments, `em`/`ppi`, prev/sum/
  avg, prose-tolerant parsing).
- `custom.rs`: extension registry (units, functions) with validation.
- `num.rs`: checked f64/integer conversions (the only `as` casts, behind `#[expect]`).

Tests: `tests/acceptance.rs` (spec table), `tests/semantics.rs` (lexer/parser/document edge
cases), `tests/extensions.rs` (hooks with closures), `tests/performance.rs` (ignored; run
`cargo test -p napkin-core --release --test performance -- --ignored`; ~1.3 ms for 500 lines).

## Status

- [x] units, currency, zones
- [x] lexer, classifier
- [x] parser, evaluator, formatter
- [x] document semantics
- [x] extension hooks
- [x] tests (acceptance 40 rows / 63 lines all passing, semantics, extensions, perf)
- [x] gate (fmt, clippy -D warnings, test, doc)

## Decisions

- **Prose tolerance**: unknown words become `Text` tokens and are dropped. If the remaining
  tokens do not parse, dangling operators/keywords are trimmed and the longest parsable prefix
  is used. A line without a numeric token (number, variable, constant, prev/sum/avg, date/time
  word or literal) is `None`, never an error. `=` in a non-assignment line ends the
  expression (`1 meter 20 cm = 120 cm` evaluates the left side).
- **Context decides word meaning** (classifier): unit words only count as units right after a
  value (`5 cm`, `(1+2) m`, `x cm`) or after `in/to/as/into`, `/`, `per`, `sq`/`cubic`; else
  they are prose. Zone names only count next to `time`/`now`, after a time literal or after a
  conversion keyword. `of`/`on`/`off` are percentage keywords only after `%`, a variable,
  `prev`/`sum`, `)` or `percent` (so `2 cups of flour and 3 eggs` = 5 cups).
- **Case rules**: uppercase words that are currency codes win over case-insensitive unit
  names (`CUP` currency, `cup` volume, `TON` crypto, `ton` tonne). Unit symbols are
  case-sensitive, unit names are not. Scales `k`/`M` only directly after a number.
- **`pound(s)`** is weight unless the line contains another currency (sign, code or name), then
  GBP (`5 pounds in EUR`, `£5 + 3 pounds`).
- **Display form**: word-style units keep the abbreviation the user typed (`1 hour 30 min` →
  `90 min`, spec), otherwise words pluralised by value. Squared/cubed word units display as
  `square feet`, `cubic feet`.
- **Functions return plain numbers**, except `round`/`ceil`/`floor`/`abs`, which keep currency
  and percent units (`round($3.456)` → `$3`). Spec row `round(1 month in days)` → `30` requires
  dropping physical units.
- **Unary minus binds tighter than `^`** as the spec's precedence list says (`-2^2` = 4).
- **Percent arithmetic**: `x * 5%` multiplies by 0.05 (spec); `5% * 2` stays a percentage
  (10%). `X + P%` / `X - P%` apply the percentage to X.
- **Small numbers**: values that round to zero at the precision keep `precision` significant
  digits (`0.002 s`); below 1e-6 and from 1e15 scientific notation.
- **Trig noise**: |sin/cos/tan| < 1e-12 shows as 0 (`sin 180` → 0).
- **Date math**: whole days/weeks/months/years follow the calendar in the moment's zone
  (`today + 1 month` → 28 Nov); other durations add seconds. Moment − moment is the wall-clock
  difference (so `New York time - London time` → `-4 hours`), in days for two dates or ≥ 2 days,
  hours from 1 hour, else minutes.
- **Time literals with a zone** (`2:30 pm HKT`) are anchored there (today's date in that zone);
  a zone after any other value converts.
- **Extensions**: custom unit phrases beat built-in units (Numi's sample uses `hrs`); a custom
  unit whose `base_unit_id` equals its own id starts a new dimension (Numi's ScreenUnits sample
  does this); `base_unit_id` may also be another custom id, a currency code or a built-in unit
  id. Custom function phrases are case-insensitive and beat built-in functions. Global
  variables resolve their `unitId` when used (unknown id → error on use).
- **Variable names** may not be units (incl. one-letter symbols such as `b`, `g`, `h`),
  currencies, keywords, functions or constants; `m = 5` and `hours = 5` are errors.
- `Clock::System` uses chrono's `Local` for the local zone (no extra dependency).

## Spec rows

No spec row needed changing. All 63 checked lines of the acceptance table pass.

## Known gaps / ideas

- No derived SI units beyond the spec (newton, watt, joule, pascal); extensions can add them.
- No data-rate units (`Mbps`); extensions can add them.
- Custom function phrases are single words.
- English-style `1,5` reads as `1 5` (= 5), as there is no decimal comma in English style.
