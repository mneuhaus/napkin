# Napkin specification

Napkin is a 1:1 open-source alternative to [Numi](https://numi.app) (macOS calculator notepad,
v3.35): you type calculations in plain words on the left, results appear on the right of the
same line. Source of truth for Numi's behaviour: its wiki
(<https://github.com/nikolaeu/numi/wiki>), README, changelog and plugin samples
(<https://github.com/nikolaeu/numi>). Where Numi's docs are silent, this file decides.

Rust, [GPUI](https://www.gpui.rs) via `gpui-kit` 0.7.3 for the UI, code following
[BeautifulRust](https://github.com/morganlinton/BeautifulRust) (vendored as
`.claude/skills/rust-best-practices`).

## Crates

| Crate | Kind | Owns |
|---|---|---|
| `napkin-core` | lib | Lexer, parser, units, currencies, dates, time zones, formatting, document semantics, extension hooks. No I/O, no network. |
| `napkin-rates` | lib | Rate download (fiat: open.er-api.com, crypto: Coinbase), JSON cache. |
| `napkin-extensions` | lib | QuickJS host for Numi-compatible `.js` extensions. |
| `napkin-cli` | bin `napkin` | Command line, Alfred integration. |
| `napkin-app` | bin `napkin-app` | GPUI desktop app. |

The public API in `napkin-core/src/lib.rs`, `napkin-rates/src/lib.rs` and
`napkin-extensions/src/lib.rs` is the contract between crates. Implementations may add items;
they do not change or remove the existing signatures without updating every caller.

## Default settings

Precision 2 (max decimal places, trailing zeros trimmed), number style English (`1,234.56`),
trig functions read plain numbers as **degrees** (Numi 3.18 changelog), 24-hour clock.

## Engine semantics

### Lines

The document is evaluated top to bottom; one `LineOutput` per line.

- **Header**: a line starting with `#` (after optional whitespace). No result; whole line is a
  `Header` span. Numi: `# This is header`.
- **Comment**: `//` comments out the rest of the line (whole line when it starts with `//`).
  Text in double quotes is a comment: `$275 for the "Model 227"` → `$275`.
- **Label**: text up to the first `:` that is followed by whitespace or the end of the line and
  is not between two digits (so `2:30 pm` is a time, not a label) is a label and not evaluated:
  `Price: $11 + $34.45` → `$45.45`, `Line 1: $10` → `$10`.
- **Prose tolerance**: words the engine does not know are ignored (`Text` spans), as in Numi:
  `Discounted: prev - 5% discount`, `$275 for the "Model 227"`, `3 apples + 4 apples` → `7`.
  A line without any numeric content yields `LineResult::None`, never an error.
- **Errors**: a line that clearly tries to compute but cannot (incompatible units, unknown
  function arguments, division by zero) yields `LineResult::Error`. The app shows nothing.

### Numbers

- Decimal `1.5`, grouped `1,000,000` (English style), space-grouped `5 300` (groups of exactly
  three digits after the first group), scientific input `1e3`, `2.5E-4`.
- `0b110111011` → `443`, `0o1435343` → `408291`, `0x1F` → `31`.
- Scales after a number: `k`, `thousand` (×1e3), `M`, `million` (×1e6), `billion`, `bn` (×1e9),
  `trillion` (×1e12). One-letter scales are case-sensitive: `m` is metre, `K` is kelvin, `M`
  alone is million, but `MB` is megabytes. `$2k` → `$2,000`, `2M eur` → `€2,000,000`.
- In `German` and `French` style the decimal separator is `,` for input and output; in
  `Swiss`, `English` and `Plain` it is `.`.

### Operators

| Operation | Tokens |
|---|---|
| Addition | `+`, `plus`, `and`, `with` |
| Subtraction | `-`, `minus`, `subtract`, `without` |
| Multiplication | `*`, `×`, `times`, `multiplied by`, `mul` |
| Division | `/`, `÷`, `divide`, `divide by`, `divided by` |
| Exponent | `^`, `**` |
| Bitwise and / or / xor | `&`, `|`, `xor` |
| Shifts | `<<`, `>>` |
| Modulo | `mod` |
| Factorial | `5!`, `fact 5` |

Usual precedence: unary minus, `^` (right-assoc), implicit multiplication, `* / mod`, `+ -`,
shifts, `&`, `xor`, `|`, conversion (`in`/`into`/`as`/`to`) lowest.
Implicit multiplication between adjacent groups: `6 (3)` → `18`, `2(3+4)` → `14`, `2 pi` → `6.28`.
Adjacent quantities of the same dimension add up: `1 meter 20 cm` → `120 cm`,
`5 feet 10 inches` → `70 inches`, `1 hour 30 min` → `90 min`.

### Mixed units

`+`/`-` between compatible quantities converts to the **right** operand's unit
(`1 meter + 20 cm` → `120 cm`). `*`/`/` combine dimensions (`60 km / 2 h` → `30 km/h`,
`3 m * 4 m` → `12 m²`, `10 m² / 2 m` → `5 m`); dividing equal dimensions gives a plain number.
A plain number with a quantity takes the quantity's unit (`$5 + 3` → `$8`).

### Conversion

`in`, `into`, `as`, `to`: `20 inches in cm` → `50.8 cm`, `$30 in Euro`, `1.2 em in px`.
Format targets: `in hex` → `0x63AE3`, `in bin`/`binary` → `0b…`, `in oct`/`octal` → `0o…`,
`in sci`/`scientific` → `5.3e3`, `in dec`/`decimal`.
Since `in` is the conversion keyword, inches are `inch`/`inches` (also `"` is not inches: quotes
are comments).

### Variables and constants

- `name = expr` assigns and shows the value. Names: letters, digits, `_`, not starting with a
  digit, not a unit, keyword, function or constant (`m = 5` is an error), case-sensitive.
- `em` and `ppi` are assignable settings for the rest of the document: `em = 20px`, `ppi = 326`.
- Constants: `pi` / `π` = 3.1415926536…, `e` = 2.7182818285…
- Tokens: `prev` = result of the nearest line above that has a value. `sum` / `total` = sum of
  the values of the lines above, up to the previous empty line or header (lines without a value
  are skipped; mixed compatible units convert to the first value's unit). `average` / `avg`
  = their mean.

### Percentages

| Operation | Example | Result |
|---|---|---|
| Add/subtract percent | `$10 - 40%` / `$10 + 40%` | `$6` / `$14` |
| Percent of | `20% of $10` | `$2` |
| Percent on (add) | `5% on $30` | `$31.50` |
| Percent off (subtract) | `6% off 40 EUR` | `€37.60` |
| X as % of Y | `$50 as a % of $100` | `50%` |
| X as % on Y | `$70 as a % on $20` | `250%` |
| X as % off Y | `$20 as a % off $70` | `71.43%` |
| Percent of what | `5% of what is 6 EUR` | `€120` |
| Percent on what | `5% on what is 6 EUR` | `€5.71` |
| Percent off what | `5% off what is 6 EUR` | `€6.32` |

`x * 5%` multiplies by 0.05; `v times 7 - v2` with `v = $20`, `v2 = 5%` → `$133`.

### Units

All SI-based units accept every SI prefix, short and long, case-sensitive
(`mm`, `picometers`, `GB`, `MHz`). Plural and singular forms, common abbreviations.

- **Length**: meter, mil, point, line, inch, hand, foot, yard, rod, chain, furlong, mile,
  cable, nautical mile, league; CSS `px`, `pt`, `em` (see CSS).
- **Area**: hectare, are, acre, and squared lengths with `square`/`sq` prefix or `²`/`^2` suffix:
  `20 sq cm`, `30 square inches`, `11 sqm` → `11 m²`.
- **Volume**: pint, quart, gallon, teaspoon (`tsp`), tablespoon (`tbsp`), cup, liter (`l`, `L`,
  with SI prefixes), cubed lengths with `cubic`/`cu`/`cb` prefix or `³`: `20 cu cm`, `11 cbm`.
- **Weight**: gram (SI prefixes), tonne, carat, centner, pound, stone, ounce.
- **Time**: second, minute, hour, day, week, month (= 1/12 year), year (= 365 days), with SI
  prefixes on seconds (`ms`, `ns`). `round(1 month in days)` → `30`.
- **Temperature**: kelvin (`K`), celsius (`°C`, `C`, `celsius`), fahrenheit (`°F`, `F`).
  Offset conversion for absolute values: `100 °C in F` → `212 °F`, `0 K in C` → `-273.15 °C`.
- **Angle**: radian (`rad`), degree (`deg`, `°`).
- **Data**: bit (`b`), byte (`B`); SI prefixes ×1000 (`kB`, `MB`, `GB`, `TB`, `kb`, `Mb`), IEC
  ×1024 (`KiB`, `MiB`, `GiB`, `TiB`). `1 GiB in MB` → `1,073.74 MB`.
- **Speed and other compounds** fall out of dimension algebra: `km/h`, `mph`, `m/s`, `knot`.
- **Frequency**: hertz (`Hz`, SI prefixes).

Display: metric/SI, data and compound units use symbols (`cm`, `kg`, `GB`, `°C`, `km/h`, `m²`);
US/imperial and time units use words, pluralised by value (`1 inch`, `70 inches`, `3 feet`,
`2 hours`, `30.42 days`); angles `°`/`rad`; percent `%`.

### CSS

`px`, `pt`, `em`. `em` defaults to 16 px, `ppi` to 96. `12 pt in px` → `16 px`,
`1 inch in px` → `96 px`; after `ppi = 326`: `1 cm in px` → `128.35 px`;
after `em = 20px`: `1.2 em in px` → `24 px`.

### Currency

- ISO 4217 codes (`USD`, `EUR`, `CAD`…), names (`dollar(s)`, `euro(s)`, `pound(s)`, `yen`,
  `rouble(s)`/`ruble(s)`, `franc(s)`, `rupee(s)`, `yuan`, `won`…), signs (`$` = USD, `€`, `£`,
  `¥` = JPY, `₽`, `₹`, `₩`, `₺`, `₴`, `₪`, `฿`, `₿` = BTC). A sign can prefix or follow the number.
  `$30 CAD` means 30 CAD (an explicit code overrides the generic `$`).
- Three-letter codes are case-insensitive only for an allowlist of common currencies (USD, EUR,
  GBP, JPY, CHF, CAD, AUD, NZD, CNY, HKD, SGD, SEK, NOK, DKK, PLN, CZK, HUF, RUB, UAH, INR, KRW,
  BRL, MXN, ZAR, TRY, ILS, THB, AED, SAR); every other code (and every crypto ticker: BTC, ETH,
  SOL, DOGE, …) must be uppercase, so words like `cup`, `all`, `top`, `pen` keep their meaning.
- Conversion through `ExchangeRates` (units per one base currency). Unknown code → error.
- Display: `$`, `€`, `£`, `¥` as prefix (`$1,234.50`, `€37.60`), every other currency as code
  suffix (`27.53 CHF`). Fiat shows no decimals when the value is whole after rounding to cents,
  else exactly 2 (`$2,000`, `$31.50`). Crypto shows up to 8 decimals, trailing zeros trimmed.

### Dates, times and time zones

- `now`, `time` → current local time of day; `today`, `tomorrow`, `yesterday` → dates.
- Date literals: `2026-03-12`, `12 March 2026`, `March 12`, `12.03.2026`, `3/12/2026`
  (month first only in English style).
- Times of day: `14:30`, `2:30 pm`, `2pm`, `noon`, `midnight`.
- Arithmetic: `today + 2 weeks`, `now + 3 hours`, `3:30 pm + 2 hours` → `17:30`,
  `2026-12-24 - today` → duration in days, `fromunix(1446587186)`.
- Time zones by IANA name (`Europe/Berlin`), city (`Berlin`, `New York`, `San Francisco`,
  `Hong Kong`; every IANA zone's city plus a curated alias list of major cities that are not
  zone names), and abbreviation (`PST`/`PDT`/`PT` → America/Los_Angeles, `EST`/`EDT`/`ET`,
  `CST`, `MST`, `GMT`, `UTC`, `CET`/`CEST`, `BST`, `IST` (India), `JST`, `HKT`, `AEST`, `MSK`, …).
  Abbreviations map to their zone and follow its daylight-saving rules.
- `PST time`, `New York time`, `time in Madrid`, `now in Tokyo`, `Berlin time` → time there.
  `2:30 pm HKT in Berlin` → that moment in Berlin. `New York time - London time` → difference.
- Display: time of day `13:00` (or `1:00 pm` in 12-hour format); when the date is not today in
  the shown zone, prefix the weekday: `Thu 01:30`. Dates `Wed, 28 Oct 2026`; date-times
  `Wed, 28 Oct 2026 13:00`.

### Functions

`sqrt`, `cbrt`, `root n (x)`, `abs`, `log x` (base 10), `log b (x)`, `ln`, `fact`, `round`,
`ceil`, `floor`, `sin`, `cos`, `tan`, `arcsin`/`asin`, `arccos`/`acos`, `arctan`/`atan`,
`sinh`, `cosh`, `tanh`, `fromunix`. Called with or without parentheses (`sqrt 16`, `sqrt(16)`).
Multiple arguments are separated by `;`: `myFunction(1;5;4)`. Inverse trig functions return
degrees in degree mode. `sin 45°` works in either mode.

### Extensions (Numi JavaScript API)

`numi.addUnit({id, phrases, baseUnitId, format, ratio})`,
`numi.addFunction({id, phrases}, fn(values) -> value)`, `numi.setVariable(name, value)`,
`log(...)`. Values are `{double, unitId}` or plain numbers. Base unit ids: `m`, `second`,
`percent`, `m2`, `m3`, `bit`, `byte`, `radian`, `gram`, `USD`. Folders:
`~/Library/Application Support/Napkin/extensions`, plus Numi's
`~/Library/Application Support/com.dmitrynikolaev.numi/extensions` and
`~/Library/Application Support/com.nikolaeu.numi-setapp/extensions` when present.

### Acceptance examples

Default settings, `Clock::Fixed { now: 2026-10-28T12:00:00Z, time_zone: Europe/Berlin }`
(Berlin is on CET, UTC+1; New York on EDT until 2026-11-01), test rates
`USD 1, EUR 0.8, GBP 0.75, CAD 1.25, JPY 150, CHF 0.9, BTC 0.00001`. Lines in one block share a
document.

| Input | Result |
|---|---|
| `8 times 9` | `72` |
| `6 (3)` | `18` |
| `1 meter 20 cm` | `120 cm` |
| `20 inches in cm` | `50.8 cm` |
| `1 mile in km` | `1.61 km` |
| `0b110111011` | `443` |
| `0o1435343 in hex` | `0x63AE3` |
| `5 300 in sci` | `5.3e3` |
| `$30 in Euro` | `€24` |
| `$30 CAD + 5 USD - 7EUR` | `€16.20` |
| `$2k` | `$2,000` |
| `2M eur` | `€2,000,000` |
| `v = $20` / `v2 = 5%` / `v times 7 - v2` / `v + $10` | `$20` / `5%` / `$133` / `$30` |
| `12 pt in px` | `16 px` |
| `em = 20px` / `1.2 em in px` | `20 px` / `24 px` |
| `ppi = 326` / `1 cm in px` | `326` / `128.35 px` |
| `Line 1: $10` / `Line 2: $15` / `Result: sum` | `$10` / `$15` / `$25` |
| `Line 1: $10` / `Line 2: $20` / `Result: average` | … / … / `$15` |
| `Cost: $20 + 56 EUR` / `Discounted: prev - 5% discount` | `€72` / `€68.40` |
| `round(1 month in days)` | `30` |
| `1 month in days` | `30.42 days` |
| `sqrt 16` / `cbrt 8` / `root 2 (8)` | `4` / `2` / `2.83` |
| `abs(-4)` / `log 2 (10)` / `ln 3` / `fact 5` | `4` / `3.32` / `1.1` / `120` |
| `round 3.45` / `ceil 3.76` / `floor 2.56` | `3` / `4` / `2` |
| `sin 30` / `sin 45°` / `sinh 3` | `0.5` / `0.71` / `10.02` |
| `20 sq cm` / `11 sqm` / `20 cu cm` / `11 cbm` | `20 cm²` / `11 m²` / `20 cm³` / `11 m³` |
| `3 GB in MB` / `1 GiB in MB` | `3,000 MB` / `1,073.74 MB` |
| `100 °C in F` / `0 K in C` | `212 °F` / `-273.15 °C` |
| `60 km / 2 h` | `30 km/h` |
| `# This is header` / `// This is comment` | none / none |
| `$275 for the "Model 227"` | `$275` |
| `Price: $11 + $34.45` | `$45.45` |
| `now` / `time` | `13:00` |
| `today` / `today + 2 weeks` | `Wed, 28 Oct 2026` / `Wed, 11 Nov 2026` |
| `PST time` / `New York time` / `Time in Madrid` | `05:00` / `08:00` / `13:00` |
| `2:30 pm HKT in Berlin` | `07:30` |
| `fromunix(1446587186)` | `Tue, 3 Nov 2015 22:46` |
| `3:30 pm + 2 hours` | `17:30` |
| `1 BTC in USD` | `$100,000` |
| `$50 in BTC` | `0.0005 BTC` |

## App (napkin-app)

### Window and editor

- One window per open note set, default 560×440, resizable, hidden title bar with traffic lights;
  the note title shows in the title area. Remembers size and position.
- Editor fills the window. Each line's result is drawn right-aligned on the same row, in the
  theme's result colour; results never overlap the text (a result column sized to the widest
  visible result, the editor soft-wraps to the space left; the result aligns with the first
  visual row of its line). Errors show nothing.
- Syntax highlighting from `LineOutput::spans`, colour per `SpanKind` from the theme; headers
  bold; comments and labels dimmed; ignored prose in the normal text colour, slightly dimmed.
- Hovering a result highlights it; clicking copies `Answer::text` to the clipboard and shows a
  short "Copied" flash.
- Re-evaluate on every edit (synchronously; documents are small).

### Notes

- Multiple notes, autosaved as plain UTF-8 `.numi` files in the notes folder
  (`~/Library/Application Support/Napkin/notes`, changeable in settings, e.g. to an iCloud Drive
  or Dropbox folder as the open replacement for Numi's iCloud sync).
- `⌘N` new note, `⌘L` notes list (searchable, switch, rename, delete with confirmation),
  `⌘⇧]` / `⌘⇧[` next/previous note. Title = first non-empty line, unless renamed.
- Import `⌘O` (open a `.numi`/`.txt` file as a new note), export `⌘S` (save the current note as
  `.numi`), print `⌘P` (native print dialog with input and results), shortcuts as in Numi.

### Shortcuts (Numi)

| Action | Key |
|---|---|
| Surround selection (or the current line's expression) with parentheses | `⇧⌘0` |
| Copy result of the current line | `⇧⌘C` |
| Select all | `⌘A` |
| Delete all | `⌥⌘⌫` |
| Copy all (input with results, `input  = result` per line) | `⌥⌘C` |
| Import / Export / Print | `⌘O` / `⌘S` / `⌘P` |
| Settings | `⌘,` |
| Increase / decrease / reset font size | `⌘+` / `⌘-` / `⌘0` |

### Themes

Built-in: System (follows light/dark), Light, Dark, Numi Classic (Numi's dark grey with warm
yellow results), Solarized Light, Solarized Dark. Each theme defines background, text, result,
selection, caret, and one colour per `SpanKind`.

### Settings (`⌘,`)

Theme, font size, precision (0–10), number style, angle unit, time format, notes folder,
global hotkey (default `⌥⌘N`, can be disabled), show menu bar icon, show dock icon,
launch at login, Alfred/HTTP integration on/off. Persisted as JSON in
`~/Library/Application Support/Napkin/settings.json`.

### Menu bar and hotkey

- Menu bar icon (tray) toggles the window; option to run without a dock icon.
- Global hotkey shows/hides the window from anywhere.

### Alfred / HTTP

Like Numi: `http://localhost:15055/?q=<expression>` answers with the formatted result as plain
text (Numi's Alfred workflow works unchanged). Only binds to 127.0.0.1.

### Rates and extensions

On start: cached rates (or bundled), refresh in the background when older than an hour, then
re-evaluate. Load extensions from the extension folders on start; settings has "Reload
extensions" and "Open extensions folder".

## CLI (napkin-cli)

- `napkin "20 inches in cm"` → `50.8 cm` (exit 0; exit 1 with the message on stderr on error).
- `napkin` with stdin (or `-f file.numi`) evaluates a whole document and prints
  `line  → result` per line (`--results-only` prints just results, blank for none).
- `--json` prints machine-readable output; `--refresh-rates` downloads rates first;
  `--no-extensions`; `--precision N`.
- `napkin serve [--port 15055]` runs the Alfred-compatible HTTP endpoint headless.
- `integrations/alfred/`: an Alfred workflow (script filter calling `napkin`).
