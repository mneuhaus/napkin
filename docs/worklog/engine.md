# Engine worklog (napkin-core)

Branch `engine`, worktree `.worktrees/engine`. Scope: `crates/napkin-core/` only.

## Plan

Modules in `crates/napkin-core/src/`:

- `units.rs`: built-in unit table, SI/IEC prefixes, dimensions, compound units.
- `currency.rs`: ISO codes, names, signs, crypto tickers, case rules.
- `zones.rs`: time zone lookup (IANA names, cities, aliases, abbreviations).
- `lexer.rs`: raw tokens of one line (numbers by number style, dates/times, words, symbols).
- `classify.rs`: context-sensitive classification of words into semantic tokens + spans.
- `ast.rs`, `parser.rs`: Pratt parser (implicit multiplication, conversions, percent forms).
- `value.rs`: quantities and moments, arithmetic.
- `eval.rs`, `functions.rs`: evaluator and built-in functions.
- `format.rs`: number, currency, unit, date/time formatting.
- `document.rs`: line semantics (headers, comments, labels, assignments, prev/sum/avg).
- `custom.rs`: extension registry (units, functions, globals) with validation.

## Status

- [ ] units, currency, zones
- [ ] lexer, classifier
- [ ] parser, evaluator, formatter
- [ ] document semantics
- [ ] extension hooks
- [ ] tests (acceptance, unit, extension, perf)
- [ ] gate (fmt, clippy, test, doc)

## Decisions

(see below, appended as work progresses)
