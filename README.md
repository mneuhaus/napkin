# Napkin

Back-of-the-napkin math for your desktop. Napkin is an open-source calculator notepad that
speaks [Numi](https://numi.app)'s language: type calculations in plain words on the left, read
the answers on the right of the same line.

**Website:** <https://mneuhaus.github.io/napkin/> · **Download:** [latest release](https://github.com/mneuhaus/napkin/releases/latest)

```
# Trip to Rome
Hotel: 3 nights * €89            €267
Train: $120 in EUR               €107.69
Food: 4 days * 35 EUR            €140
Total: sum                       €514.69

20 inches in cm                  50.8 cm
5 feet 10 inches in cm           177.8 cm
2:30 pm HKT in Berlin            07:30
today + 2 weeks                  Wed, 11 Nov 2026
price = $1,200                   $1,200
price - 15% discount             $1,020
60 km / 2 h                      30 km/h
```

Built in Rust with [GPUI](https://www.gpui.rs), following the
[BeautifulRust](https://github.com/morganlinton/BeautifulRust) guidelines.

## What it understands

Everything in Numi's documentation, plus a little more:

- **Arithmetic** with symbols or words (`8 times 9`, `$50 without 10%`), implicit
  multiplication (`6 (3)`), `mod`, bitwise operators, factorials, hex/binary/octal input and
  output (`0o1435343 in hex`), scientific notation (`5 300 in sci`), scales (`$2k`, `2M eur`).
- **Units** with every SI prefix: length, area (`20 sq cm`), volume (`20 cu cm`, cups, gallons),
  weight, time, temperature, angles, data (`1 GiB in MB`), CSS (`px`, `pt`, `em`, `ppi`), and
  compound units from dimension algebra (`km/h`, `m²`). Mixed units add up (`1 meter 20 cm`).
- **Currencies** by code, name or sign, live rates for ~160 fiat currencies and ~40 crypto
  tickers (`$30 CAD + 5 USD - 7EUR`, `0.25 BTC in EUR`), cached for offline use.
- **Percentages** in all of Numi's forms: `20% of $10`, `5% on $30`, `6% off 40 EUR`,
  `$50 as a % of $100`, `5% of what is 6 EUR`, …
- **Dates, times and time zones**: `now`, `today + 2 weeks`, `New York time`,
  `2:30 pm HKT in Berlin`, `fromunix(1446587186)`, `2026-12-24 - today`.
- **Functions**: `sqrt`, `cbrt`, `root 2 (8)`, `log 2 (10)`, `ln`, `round`, trig in degrees or
  radians, and more.
- **Notes as documents**: `# headers`, `// comments`, `"quoted comments"`, `Labels:`, variables,
  `prev`, `sum`/`total`, `avg`/`average`. Prose between the numbers is ignored.
- **28 input languages**, Numi's set, on top of English: `5 Zoll in cm`, `20 Prozent von 50`,
  `heute + 2 Wochen`, `3 fois 4`, `14h30 à New York`, `5厘米加3毫米`, `三百五十元`,
  `2 часа в минутах`. Word units and dates come out in the chosen language (`2 Stunden`,
  `Mi., 28. Okt. 2026`).

## Install

**App (macOS 11+, Apple Silicon and Intel):** download `Napkin-<version>.dmg` from the
[latest release](https://github.com/mneuhaus/napkin/releases/latest), open it and drag Napkin
into Applications.

> The current builds are not notarized by Apple yet. The first launch is blocked once: open
> **System Settings → Privacy & Security**, scroll to the message about Napkin and click
> **Open Anyway**. (Or run `xattr -dr com.apple.quarantine /Applications/Napkin.app`.)

**Command line:** download `napkin-cli-<version>-macos-universal.tar.gz` from the release and put
`napkin` on your `PATH`, or build it with Cargo:

```sh
cargo install --git https://github.com/mneuhaus/napkin napkin-cli
```

**Alfred:** open `Napkin.alfredworkflow` from the release (needs the command line tool).

## The app

- Results right-aligned on each line, syntax colouring, click a result to copy it.
- Multiple notes saved as plain `.numi` files (point the notes folder at iCloud Drive or Dropbox
  to sync), a searchable notes list (`⌘L`), import/export/print.
- Numi's shortcuts (`⇧⌘C` copy result, `⌥⌘C` copy all, `⇧⌘0` parenthesise, …), six themes
  including Numi Classic and Solarized, settings for input language, precision, number
  style, angle unit and clock format.
- Menu bar icon, global hotkey (`⌥⌘N`), launch at login, optional Dock icon.
- Numi's Alfred endpoint on `http://localhost:15055/?q=…`, so Numi's Alfred workflow works too.

## Command line

```sh
cargo install --path crates/napkin-cli
napkin "20 inches in cm"          # 50.8 cm
napkin -f trip.numi               # every line with its result
napkin --json "1 BTC in EUR"
napkin --language de "5 Zoll in cm"   # 12.7 cm (default: the system language)
napkin serve                      # Alfred-compatible HTTP endpoint without the app
```

An Alfred workflow lives in [`integrations/alfred`](integrations/alfred).

## Extensions

Napkin runs Numi's JavaScript extensions unchanged (`numi.addUnit`, `numi.addFunction`,
`numi.setVariable`, `log`). It loads them from `~/Library/Application Support/Napkin/extensions`
and from Numi's own extension folders when they exist:

```js
numi.addUnit({
  id: "horse", phrases: "horse, horses, hrs",
  baseUnitId: "m", format: "hrs", ratio: 2.4,
});
numi.addFunction({ id: "zum", phrases: "zum" }, (values) => ({
  double: values[0].double + values[1].double,
}));
```

## Building from source

Rust (stable) and Xcode (for the Metal shader compiler GPUI needs):

```sh
cargo test --workspace
cargo run -p napkin-app            # the app
./scripts/bundle-macos.sh         # dist/Napkin.app
./scripts/release-macos.sh        # dist/release: universal DMG, CLI tarball, Alfred workflow
```

The code follows the BeautifulRust guidelines (`.claude/skills/rust-best-practices`);
`cargo clippy --workspace --all-targets -- -D warnings` must stay clean. After cloning, enable
the secret-scanning hooks with `git config core.hooksPath .githooks`.

## Layout

| Crate | What |
|---|---|
| `napkin-core` | The calculation engine (no I/O). |
| `napkin-rates` | Exchange-rate download (open.er-api.com, Coinbase) and cache. |
| `napkin-extensions` | QuickJS host for Numi-compatible extensions. |
| `napkin-cli` | The `napkin` command. |
| `napkin-app` | The GPUI desktop app. |

`docs/spec.md` is the behaviour contract, with an acceptance table the engine tests run against.

## License

MIT. Napkin is an independent project, not affiliated with Numi.
