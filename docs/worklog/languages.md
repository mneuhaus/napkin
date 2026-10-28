# Languages worklog (multilingual input)

Branch `main`. Goal: Napkin understands Numi's 28 input languages on top of English, shows
word units and dates in the selected language, and accepts every phrase Numi's own vocabulary
accepts (compatibility test against the Numi 3.34 app bundle in `tmp/numi-dmg/engine-strings`,
reference only, never committed).

## Status

- [x] core: `InputLanguage` (28 languages, tags, native names, `from_locale`, serde as tag),
      `Settings::input_language` (serde default English), `InputLanguage::sample_note()`
- [x] core: data format + loader (`src/vocab.rs`, `lang/<tag>.txt`), classifier layering,
      CJK segmentation, word assignments, o'clock, local zone, localized units and dates
- [x] German (de, de-AT, de-CH) and English extras, except currency names
- [x] currency names for every language, all other languages (delegated, see below)
- [x] test harness: `tests/languages/` (tables + sample notes), `tests/numi_compat.rs`
- [x] app picker, CLI `--language` (+ `--number-style`), spec "Languages" section, README
- [ ] gate, screenshots, final commit

## Layout

- `crates/napkin-core/src/language.rs`: the public `InputLanguage` enum.
- `crates/napkin-core/src/vocab.rs`: data file format (module docs), loader, `Vocab` (lookups,
  plural rules, display forms, date patterns), `Lexicon` (selected language over English extras).
- `crates/napkin-core/lang/<tag>.txt`: vocabulary per language; `lang/<tag>.numi`: sample note.
  `lang/en.txt` holds English phrases beyond the built-in grammar (always active).
- `crates/napkin-core/tests/languages/`: per-language acceptance tables + sample-note test.
- `crates/napkin-core/tests/numi_compat.rs`: Numi compatibility; explained misses per language in
  `tests/numi_compat/<tag>.txt`. `NAPKIN_COMPAT_LANG=de cargo test -p napkin-core --test
  numi_compat -- --nocapture` checks one language; the table lands in `tmp/numi-compat-coverage.md`.

## Decisions

- **Own data format, not Numi's**: `key = phrase, phrase` lines with `{a,b}` alternation, keys
  named after Napkin's concepts (`times`, `percent.of`, `unit.<id>`, `show.<id>`, `currency.<CODE>`,
  `zone.<IANA>`, `month.<n>`), prefix words (`prefix.c = Zenti`) and templates (`square =
  Quadrat*`) generate compounds. Numi's `.strings` are only read by the compatibility test.
- **English is always on**; the selected language is consulted first, then English's extras
  (`lang/en.txt`), then the built-in English grammar.
- **Precedence** (documented in `classify.rs` module docs):
  - longest phrase wins; a user variable shadows every language word;
  - short English unit symbols keep their meaning: a language word spelt like a ≤2-letter
    English symbol of a base unit or a common prefix (k, M, G, c, m, µ, n) cannot be a scale,
    unit or currency (`m` stays metre, so German/French/Romanian `m` for million/thousand is an
    explained miss; `kt` stays knot). Rare-prefix symbols give way (`Em` exametre → CSS em,
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
- **German specifics**: `14 Uhr`, `14:30 Uhr`, `14.30 Uhr` are times of day (`Uhr` alone = now),
  `12. März 2026`, `übermorgen`/`vorgestern`, exonyms (`Wien`, `Moskau`, `Peking`, ...),
  `Zentner` only in de-AT/de-CH (100 kg there, 50 kg in Germany, so de leaves it unknown),
  `Deka` = 10 g in de-AT, `Fuss` display and `Fr.`/`Stutz` in de-CH, `Pfund` follows `pound`.
- **Output**: word units use the language's `show.<id>` forms with CLDR plural categories
  (one/few/many/other per language) and fall back to English words; squared/cubed word units
  use `show.square`/`show.cubic` (`Quadratfuß`), else symbols. Dates use the language's
  weekday/month names and pattern (`Mi., 28. Okt. 2026`). English output is unchanged.
- **New built-in unit**: `ping` (坪, 400/121 m²) because Numi has it in several languages.
- `local` is a zone only in zone position (`in local`, `2pm local`, `local time` after a
  conversion); on its own it stays prose so English lines with "local" do not change.
- **English extras** from Numi: `multiply`, `mult`, `square root`, `cube root`, `percents`,
  `from` (percent of), `as a % decrease/increase of`, `milliard`, `local`, `exponent`, `w`,
  `y`, `mon`, `″`, dotted abbreviations, `cc`, `ccm`, `CI`, `lbm`, `quintal`. Excluded on purpose:
  `a` (are; article), `th`/`th.` (thousand; ordinals), `t.` (teaspoon; tonne).

## Delegation (2026-10-28)

Vocabulary authoring ran in six parallel subagents that only touched their own files
(`lang/<tag>.txt`, `lang/<tag>.numi`, `tests/languages/<module>.rs`,
`tests/numi_compat/<tag>.txt`) and reported engine gaps (`ENGINE:` misses) instead of editing
`src/`: a = en/de currencies, b = fr/it/es, c = pt-BR/pt-PT/ro, d = nl/nl-BE/af/da/no/sv/fi,
e = ru/uk/pl/cs/bs-BA/tr/he, f = zh-Hans/zh-Hant/ja/ko. Their worklogs are in `tmp/lang-work/`.

## Engine features added from the agents' gap reports

- Uppercase currency codes beat a language's unit word (`5 SEK` vs Norwegian `sek` seconds,
  `3 CUP` vs German `Cup`, `TON` vs Dutch `ton`).
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
  A lone o'clock word is prose unless also listed under `now` (German `Uhr` is).
- Korean words the vocabulary does not know are split into known words when they consist of
  them entirely (`인치를` → `인치` + `를`, `5만원`), so particles and attached units separate;
  other words (`초과`) stay whole.
- A now-word that is also a unit is not an operand after an operator (`60 km por hora` with
  Spanish `hora` = now alone).
- Conversion words of one or two letters (`à`, `a`, `zu`, `в`, `w`) convert only when a unit,
  currency, zone or format follows.

## Known gaps (not done)

- Finnish case endings as conversion (`20 dollaria euroiksi`); works after `muunnettuna`.
- Grammatical gender in squared/cubed display (Spanish `millas cuadradas` vs `pies cuadrados`):
  es/it/pt/ro show symbols (`2 mi²`) instead.
- French `14h30`, `1er mai`; Dutch dashed dates `24-12-2026`; Italian `3 giorni fa`.
- Japanese `1000円の20%引き` (postfix discount), reversed percent questions (`10是多少的20%`),
  rate words before the number (`时速60公里`), format words starting with digits (`16进制`),
  Chinese numerals (`二十`), full-width digits (`５`).
- Korean topic particles on variable names (`가격은 5000원`).
- A hyphen inside a word is minus (`Rendez-vous à 14:00`), as in English (`x-y`).
- Numbers in phrases like `3 Viertel` (German for 3/4) read as quarts (Numi's choice kept).

## Coverage

(filled in from the compatibility test at the end)
