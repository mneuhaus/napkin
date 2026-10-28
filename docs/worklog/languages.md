# Languages worklog (multilingual input)

Branch `main`. Goal: Napkin understands Numi's 28 input languages on top of English and shows
word units and dates in the selected language.

## Status

- [x] core: `InputLanguage` (28 languages, tags, native names, `from_locale`, serde as tag),
      `Settings::input_language` (serde default English), `InputLanguage::sample_note()`
- [x] core: data format + loader (`src/vocab.rs`, `lang/<tag>.txt`), classifier layering,
      CJK segmentation, word assignments, o'clock, local zone, localized units and dates
- [x] German (de, de-AT, de-CH) and English extras, except currency names
- [x] currency names for every language, all other languages (delegated, see below)
- [x] test harness: `tests/languages/` (tables + sample notes), `tests/fuzz_languages.rs`
- [x] app picker, CLI `--language` (+ `--number-style`), spec "Languages" section, README
- [x] gate (fmt, clippy -D warnings, tests, docs), CLI demo, screenshots
      (`tmp/shots/languages-*.png`), final commit

## Layout

- `crates/napkin-core/src/language.rs`: the public `InputLanguage` enum.
- `crates/napkin-core/src/vocab.rs`: data file format (module docs), loader, `Vocab` (lookups,
  plural rules, display forms, date patterns), `Lexicon` (selected language over English extras).
- `crates/napkin-core/lang/<tag>.txt`: vocabulary per language; `lang/<tag>.numi`: sample note.
  `lang/en.txt` holds English phrases beyond the built-in grammar (always active).
- `crates/napkin-core/tests/languages/`: per-language acceptance tables + sample-note test.
- `crates/napkin-core/tests/fuzz_languages.rs`: random soup of every language's words never
  panics (`NAPKIN_FUZZ_ROUNDS=<n>` for a deeper run).

## Decisions

- **Own data format, not Numi's**: `key = phrase, phrase` lines with `{a,b}` alternation, keys
  named after Napkin's concepts (`times`, `percent.of`, `unit.<id>`, `show.<id>`, `currency.<CODE>`,
  `zone.<IANA>`, `month.<n>`), prefix words (`prefix.c = Zenti`) and templates (`square =
  Quadrat*`) generate compounds.
- **English is always on**; the selected language is consulted first, then English's extras
  (`lang/en.txt`), then the built-in English grammar.
- **Precedence** (documented in `classify.rs` module docs):
  - longest phrase wins; a user variable shadows every language word;
  - short English unit symbols keep their meaning: a language word spelt like a ≤2-letter
    English symbol of a base unit or a common prefix (k, M, G, c, m, µ, n) cannot be a scale,
    unit or currency (`m` stays metre, never million or thousand; `kt` stays knot).
    Rare-prefix symbols give way (`Em` exametre → CSS em,
    `Tg` teragram → German Tag, `Grad` gigaradian → German degrees);
  - an operator word with another meaning (Italian `e` vs Euler's e, Dutch `min` vs minutes,
    Spanish `sin` vs sine, Chinese `和` plus vs sum) is an operator only between two operands
    (value before, operand after); conversion words that are also units (Portuguese `em`)
    convert only when a unit, currency, zone or format follows;
  - percentage words apply after a percentage before anything else;
  - weight-or-currency words (`Pfund`, like `pound`) are the currency on money lines, else
    the weight.
- **Language words are not reserved** as variable names (`y = 5` works in Spanish; the variable
  then shadows `y` = plus in that document). English reserved words stay reserved.
- **Assignment words** (`is`, `ist gleich`, `是`) assign only to a valid, unreserved name;
  otherwise the line evaluates as before (`what is 5 + 3` → 8). `=` keeps reporting bad names.
- **Case**: phrases of one or two characters match their exact case (`T` day vs `t` tonne),
  longer ones any case.
- **Scales follow the language**: German/French `Billion` = 10^12, `Milliarde` = 10^9; English
  `billion` = 10^9 stays for English spelling.
- **CJK**: Chinese characters and kana are one raw token each; the classifier joins the longest
  known phrase (`厘米`, `5千米`). Hangul words stay whole but break at digits (`5센티미터`).
  Words break at script changes (`5cm加3mm`). Dates `2026年3月12日`, `3月12号`, `2026년 3월 12일`.
  Full-width operators and `＃` headers work. Multi-character CJK variable names work.
- **German specifics**: `14 Uhr`, `14:30 Uhr`, `14.30 Uhr` are times of day,
  `12. März 2026`, `übermorgen`/`vorgestern`, exonyms (`Wien`, `Moskau`, `Peking`, ...),
  `Zentner` only in de-AT/de-CH (100 kg there, 50 kg in Germany, so de leaves it unknown),
  `Deka` = 10 g in de-AT, `Fuss` display and `Fr.`/`Stutz` in de-CH, `Pfund` follows `pound`.
- **Output**: word units use the language's `show.<id>` forms with CLDR plural categories
  (one/few/many/other per language) and fall back to English words; squared/cubed word units
  use `show.square`/`show.cubic` (`Quadratfuß`), else symbols. Dates use the language's
  weekday/month names and pattern (`Mi., 28. Okt. 2026`). English output is unchanged.
- **New built-in unit**: `ping` (坪, 400/121 m², the floor-area unit of Taiwan and Japan).
- `local` is a zone only in zone position (`in local`, `2pm local`, `local time` after a
  conversion); on its own it stays prose so English lines with "local" do not change.
- **English extras**: `square root`, `cube root`, `as a % decrease/increase of`, `local`,
  `exponent`, `w`, `y`, `″`, dotted abbreviations, `cc`, `ccm`, `CI`, `lbm`, `quintal`. Not
  words on purpose: `a` (the article, not the are), `th`/`th.` (ordinals, not thousand), `t.`
  (tonne, not teaspoon).

## Delegation (2026-10-28)

Vocabulary authoring ran in six parallel subagents that only touched their own files
(`lang/<tag>.txt`, `lang/<tag>.numi`, `tests/languages/<module>.rs`) and reported engine gaps
instead of editing
`src/`: a = en/de currencies, b = fr/it/es, c = pt-BR/pt-PT/ro, d = nl/nl-BE/af/da/no/sv/fi,
e = ru/uk/pl/cs/bs-BA/tr/he, f = zh-Hans/zh-Hant/ja/ko. Their worklogs are in `tmp/lang-work/`.

## Engine features added from the agents' gap reports

- Uppercase currency codes beat a language's unit word (`5 SEK` vs Norwegian `sek` seconds,
  `TON` vs Dutch `ton`).
- `percent.before` (`百分之20`, the number after it counts as a share: `百分之20的50` = 10).
- Chained scales (`2 mille miliardi`, `2 thousand million`, `5千万`).
- `per.<id>`: a unit in the denominator (`100 рублей в час`, `all'ora`, `de l'heure`).
- `filler`: a word between a number and its unit (Romanian `25 de kilograme`, French
  `3 millions de dollars`); `date-word`: joining words in date literals (`12 de março de 2026`).
- Plural forms in `show.square`/`show.cubic` (`2 pieds carrés`).
- A contested operator that is also a unit yields when the next operand carries a unit of the
  same dimension (Dutch `5 min 30 sec` = 330 s, `10 min 3` = 7).
- `scandinavian mile` built-in unit (10 km) for Swedish/Norwegian `mil`, which otherwise would
  silently become the English mil (thou).
- Danish plural rule (`1,5 time`); Romanian `few` is n % 100 in 1..19 (`101 de ore`).
- East Asian time forms: `am`/`pm` before the hour (`下午3点`, `午後3時`, `오후 3시`),
  `half-hour` and minutes after o'clock (`3点半`, `3点20分`, `3시 30분`), `unit-before.<id>`
  (`摄氏100度`, `大さじ3`), `convert-after` (Korean `유로로`, Japanese `に`), `號` as a day marker.
  A lone o'clock word is prose unless also listed under `now`.
- Korean words the vocabulary does not know are split into known words when they consist of
  them entirely (`인치를` → `인치` + `를`, `5만원`), so particles and attached units separate;
  other words (`초과`) stay whole.
- A now-word that is also a unit is not an operand after an operator (`60 km por hora` with
  Spanish `hora` = now alone).
- Conversion words of one or two letters (`à`, `a`, `zu`, `в`, `w`) convert only when a unit,
  currency, zone or format follows.
- Turkish `%20`: `percent.before = %` makes a percent sign before its number a percentage
  (`50 + %20` = 60) where the language lists it; English is unchanged.
- A `"` between two Hebrew letters is the gershayim of an abbreviation (`ש"ח`), not a quote.
- `dunam` built-in unit (1000 m²) for Turkish `dönüm` and Hebrew `דונם`.
- Vocabulary lookups borrow instead of allocating (English 5.2 ms → 2.8 ms per 500 lines).

## Known gaps (not done)

- Finnish case endings as conversion (`20 dollaria euroiksi`); works after `muunnettuna`.
- Grammatical gender in squared/cubed display (Spanish `millas cuadradas` vs `pies cuadrados`):
  es/it/pt/ro show symbols (`2 mi²`) instead.
- Italian `3 giorni fa`.
- Japanese `1000円の20%引き` (postfix discount), reversed percent questions (`10是多少的20%`),
  rate words before the number (`时速60公里`), format words starting with digits (`16进制`).
- Korean topic particles on variable names (`가격은 5000원`).
- Numerals inside everyday words beyond the `prose` lists read as numbers (`十分` "very" is 10
  minutes, `一点` "a little" is one o'clock); two Chinese digits in a row are a range (`三五个`,
  three or five) but end up multiplied (15). A lone Korean `십` stays prose (single syllables
  need a known word after them: `십분`).

## Input parsing fixes (2026-10-28)

Four gaps from the list below, closed in the engine with data-file keys where the rule is
per language:

- **French times and ordinal dates**: `hour-mark = h` (fr, pt-BR/pt-PT) and `u` (nl-BE) read
  `14h30`, `14 h 30`, `9h05`, `14u30` as times of day (two-digit minutes after the mark). The
  lexer keeps `h30` as one word, so the classifier splits mark and minutes. `14h`, `2 h` stay
  hours, and so does `2 h 30 min` (a unit after the minutes). `ordinal = er, re, ère` (fr),
  `st, nd, rd, th` (en, always on), `º, ª` (es, pt, it) are suffixes on the day of a date
  literal (`1er mai`, `March 1st`, `1º de mayo`); `30º` alone stays degrees. Flemish `14u30`
  was claimed in `nl-BE.txt` but did not work before (`u30` was one word).
- **Dashed dates**: `24-12-2026` (four-digit year last) is a date in the lexer, day first
  except English number style with English input, which is month first like `3/12/2026`; the
  other order is tried when the preferred one is no date. The slash rule is unchanged (style
  only). `10-5-3`, spaced dashes and impossible dates stay subtractions.
- **CJK numerals**: `numeral.<value>` keys (digits, 10/100/1000, group multipliers from 10^4,
  `point`) in zh-Hans, zh-Hant, ja and ko; grammar and examples in `src/numerals.rs`. Chinese
  and Japanese numerals are merged into number raws before classification (one raw per
  character), longest phrase first: a phrase or variable from the same character at least as
  long wins (`一共`, `十六进制`, `四舍五入`, `一昨日`), except a bare scale (`十万`). New `prose`
  key for everyday words with numerals (`万一`, `一半`, `一起`, `百度`, weekday names, Japanese
  `一緒`, `千葉`). A numeral takes its multipliers (`五千米` is 5,000 m, `5千米` stays 5 km).
  Korean numerals are split off in `segment` and need a multiplier: with known words after
  them (`오천원`, `만원`, `이십분`) or as a whole word of two syllables starting with a digit or
  `십` (`삼백`); `천사` (angel), `백조` (swan), `이사`, `사이` stay words.
- **Full-width forms** fold to ASCII per line in `Document::line` (spans mapped back), so they
  work in every language; the full-width comma stays (Chinese clause separator).
- **Hyphenated words**: the lexer joins letters around a hyphen into one word, so vocabulary
  phrases with hyphens (`après-demain`, `US-dollar`, `Groot-Brittanje`) and IANA cities
  (`Port-au-Prince`) match as one raw; `split_hyphens` turns it back into a subtraction only
  when every part is a value. Decision for a value joined to an unknown word (`e-mail`,
  `price-discount`): one prose word, never a subtraction of prose.

## Vocabulary cleanup (2026-10-28)

The vocabularies must stand on their own: every file now holds only the words and abbreviations
a native speaker writes. All 28 files were reviewed entry by entry. Kinds of entries removed:

- misspellings and typos (missing, extra or swapped letters, wrong diacritics), including the
  sections of "misspellings" in the German files;
- invented or nonstandard abbreviations, odd plurals and case forms, definite forms after a
  number, case-only duplicates;
- calques and mistranslations (everyday words standing in for imperial units, "weather"/"time"
  words as the current time, a currency word under the wrong currency), ungrammatical operator
  and percentage phrases (bare imperatives, missing prepositions, wrong agreement);
- currency names that are not the language's own: country-name compounds the language does not
  use (German keeps only the established ones like `Hongkong-Dollar`, `Kenia-Schilling`),
  genitive constructions, names in other languages or scripts, Traditional characters in the
  Simplified Chinese file, symbol drawings;
- English words in other languages' files where the always-on English already covers them;
- comments that described anything but the language itself.

Correct standard forms were added only where a misspelt or nonstandard variant had stood in
for them (80 phrases in all, mostly currency names). Removed phrases, counted after expanding `{a,b}` blocks:

| Language | Removed | Language | Removed | Language | Removed |
|---|---|---|---|---|---|
| en | 88 | fr | 449 | pl | 213 |
| af | 613 | he | 1 | pt-BR | 666 |
| bs-BA | 23 | it | 916 | pt-PT | 1 |
| cs | 21 | ja | 2 | ro | 91 |
| da | 22 | ko | 6 | ru | 194 |
| de | 1201 | nl | 856 | sv | 40 |
| de-AT | 7 | nl-BE | 1 | tr | 20 |
| de-CH | 6 | no | 26 | uk | 97 |
| es | 41 | fi | 4 | zh-Hans | 107 |
| | | | | zh-Hant | 23 |

All languages: 5735 removed. Acceptance tables (de, es, fr, it, nl, nordic, pl, pt, ru, zh) and
the Russian sample note now use standard phrases for the same features. Chinese has no natural
phrase for "as a % off", so that key is gone in zh-Hans and zh-Hant (the English phrase works).

## Performance

Release, 500 lines: English 2.8 ms, German 3.0 ms, Chinese 1.9 ms, Russian 1.8 ms
(`input_languages_stay_fast`, budget 10 ms). After the input parsing fixes, best of three runs
on a busy machine: English 3.0 ms, German 3.2 ms, Chinese 1.9 ms, Russian 1.8 ms (the old build
measured 2.7 / 3.0 / 1.9 / 1.8 ms in the same session).
