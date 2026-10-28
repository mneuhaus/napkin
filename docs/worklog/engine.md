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
- [x] adversarial review round (independent reviewer probed the spec and Numi semantics;
      fixes below, pinned in `tests/semantics.rs::review_regressions`)

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
- **Prose gaps**: when ignored words separate two operands, the parser sees a `Gap` token and
  does not multiply across it (`3 apples 4 pears` → 3, `$275 for the Model 227` → $275);
  operators across prose still work (`3 apples + 4 apples` → 7).
- **Malformed numbers** (`3:4`, `1.2.3`, `31.02.2026`) are prose, not arithmetic. A grouping
  separator not followed by exactly three digits reads as a decimal separator in every style
  (`3,25` → 3.25 in English style; `1.5` in German style). No-break spaces group digits.
- **Robustness limits**: at most 300 significant tokens per line and 100 levels of nesting;
  beyond that the line is an error (checked on a 512 KB thread).
- **Floating-point noise**: `a ± b` smaller than 1e-12 of its operands is 0; same for
  temperature conversions (`32 °F in °C` → `0 °C`). `tan` at 90° is an error.
- **Rounding** is half away from zero everywhere (`0.125` → `0.13`), precision is clamped to
  10, and precision 0 shows whole numbers only. Crypto shows up to 8 fixed decimals.
- **Units after a number absorb `/unit`** (`60 km/h` is one unit), so `30 km / 60 km/h` = 0.5 h.
- **Every SI prefix** on metre, gram, second, litre, hertz, kelvin, radian, including
  quetta/ronna/ronto/quecto; `qm` stays the German m². `Pm` is petametre (am/pm must be all
  lower or all upper case). Aliases `mcg`, `fortnight`, `decade`, `century`, `ºC`.
- **A date next to a zoned time uses the time's zone** (`2027-01-15 9:00 EST in Berlin`).
- **Times more than six days away** show the full date instead of a weekday prefix.
- **Zones**: city lookup ignores diacritics (`São Paulo`) and dots (`St. Petersburg`);
  `UTC+2`, `GMT-5` and `Etc/GMT+5` work. Zone names beat same-named variables in zone context.
- **Reserved names** (variables, globals, extension function phrases): keywords, scales (`k`,
  `M`, `thousand`), formats (`hex`), `what`/`is`, units, currencies, zone abbreviations.
  Extension functions may replace built-in function names (Numi's community
  `LogarithmTotalCalculator` registers `log`). Custom unit ids may not be Numi base ids or
  currency codes; unit and function phrases may not collide across extensions; custom unit
  phrases may contain symbols (`N/m2`, `Pound-force per square inch`); function phrases may be
  several words. Globals must be finite with a resolvable unit id. Unit-id lookup prefers
  built-in ids over custom ids.
- `prev` with nothing above is prose (`None`); `sum`/`average` of an empty block are 0.
- `ppi` needs a plain number; `2abc = 4` is an invalid assignment (error).

## Spec rows

No spec row needed changing. All 63 checked lines of the acceptance table pass.

## Known gaps / ideas

- No derived SI units beyond the spec (newton, watt, joule, pascal); extensions can add them.
- No data-rate units (`Mbps`); extensions can add them.
- Unknown words before a conversion are dropped, so `1 furlongz in m` reads as `1 in m` →
  `1 m` (a plain number takes the target unit). Same for unknown currency codes.
- Functions drop physical units (spec row `round(1 month in days)` → `30`), keeping only
  currency and percent for `round`/`ceil`/`floor`/`abs`.
