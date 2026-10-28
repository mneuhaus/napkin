//! Compatibility with Numi's own input vocabulary: every phrase Numi accepts in a language must
//! mean the same in Napkin with that language selected.
//!
//! Numi is closed source, so its vocabulary is not part of this repository. The test reads the
//! `.strings` files extracted from a Numi app bundle (`tmp/numi-dmg/engine-strings`, or the
//! folder in `NAPKIN_NUMI_STRINGS`) and skips when they are missing. Each `*.variants` phrase is
//! probed against an English reference (`6 mal 3` against `6 * 3`, `3 Stunden` against `3 h`),
//! and `sample.welcome` must evaluate without errors.
//!
//! Phrases Napkin deliberately reads differently are listed with the reason in
//! `tests/numi_compat/<tag>.txt` (`key<TAB>phrase<TAB>reason`); the test fails on any other miss
//! and on listed misses that pass by now. `NAPKIN_COMPAT_LANG=de` checks one language; the
//! coverage table is printed (`-- --nocapture`) and written to `tmp/numi-compat-coverage.md`.

mod common;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use napkin_core::{Answer, Engine, InputLanguage, LineResult, NumberStyle};

/// The unit ids and English phrases Numi's unit keys stand for.
const UNITS: &[(&str, &str)] = &[
    ("second", "s"),
    ("minute", "min"),
    ("hour", "h"),
    ("day", "days"),
    ("week", "weeks"),
    ("month", "months"),
    ("year", "years"),
    ("meter", "m"),
    ("inch", "inches"),
    ("mile", "miles"),
    ("foot", "feet"),
    ("yard", "yards"),
    ("point", "pt"),
    ("cable", "cables"),
    ("league", "leagues"),
    ("furlong", "furlongs"),
    ("nautical_mile", "nmi"),
    ("chain", "chains"),
    ("ping", "pings"),
    ("acre", "acres"),
    ("hectare", "ha"),
    ("are", "ares"),
    ("litre", "l"),
    ("cup", "cups"),
    ("tablespoon", "tbsp"),
    ("teaspoon", "tsp"),
    ("quart", "quarts"),
    ("pint", "pints"),
    ("gallon", "gallons"),
    ("cbinch", "cu inch"),
    ("cbcm", "cm³"),
    ("gram", "g"),
    ("tonne", "t"),
    ("stone", "stones"),
    ("carat", "carats"),
    ("pound", "pounds"),
    ("centner", "centners"),
    ("ounce", "ounces"),
    ("celsius", "°C"),
    ("fahrenheit", "°F"),
    ("kelvin", "K"),
    ("radian", "rad"),
    ("degree", "degrees"),
    ("pixel", "px"),
    ("em", "em"),
    ("bit", "bits"),
    ("byte", "bytes"),
];

/// A probe: the lines with the phrase (`{}`), and the same lines in English.
struct Probe {
    lines: Vec<String>,
    reference: Vec<String>,
}

fn probe(key: &str, phrase: &str) -> Option<Probe> {
    let p = |template: &str| template.replace("{}", phrase);
    let make = |with: &[&str], english: &[&str]| Probe {
        lines: with.iter().map(|line| p(line)).collect(),
        reference: english.iter().map(|line| (*line).to_owned()).collect(),
    };
    if let Some(code) = key
        .strip_suffix(".variants")
        .filter(|code| is_currency_code(code))
    {
        let reference = format!("3 {code}");
        return Some(make(&["3 {}"], &[reference.as_str()]));
    }
    let stem = key.strip_suffix(".variants")?;
    if let Some((_, english)) = UNITS.iter().find(|(name, _)| *name == stem) {
        let reference = format!("3 {english}");
        return Some(make(&["3 {}"], &[reference.as_str()]));
    }
    // Phrases with `…` mark where the base value goes: `…的` is `50的20%`.
    if phrase.contains('…') && stem.starts_with("percent_o") {
        let base = phrase.replacen('…', "50", 1);
        let (english, share) = match stem {
            "percent_of" => ("20% of 50", "20%"),
            "percent_on" => ("20% on 50", "20%"),
            _ => ("20% off 50", "20%"),
        };
        return Some(Probe {
            lines: vec![format!("{base}{share}")],
            reference: vec![english.to_owned()],
        });
    }
    Some(match stem {
        "plus" => make(&["6 {} 3"], &["6 + 3"]),
        "minus" => make(&["6 {} 3"], &["6 - 3"]),
        "multiply" => make(&["6 {} 3"], &["6 * 3"]),
        "divide" => make(&["6 {} 3"], &["6 / 3"]),
        "conversion" => make(&["1 m {} cm"], &["1 m in cm"]),
        "assignment" => make(&["qx {} 7", "qx * 2"], &["qx = 7", "qx * 2"]),
        "sum" => make(&["2", "3", "{}"], &["2", "3", "sum"]),
        "avg" => make(&["2", "3", "{}"], &["2", "3", "average"]),
        "prev" => make(&["4", "{} * 2"], &["4", "prev * 2"]),
        "sqrt" => make(&["{} 16"], &["sqrt 16"]),
        "cbrt" => make(&["{} 27"], &["cbrt 27"]),
        "percent" => make(&["20 {}"], &["20%"]),
        "percent_of" => make(&["20 % {} 50"], &["20% of 50"]),
        "percent_on" => make(&["10 % {} 50"], &["10% on 50"]),
        "percent_off" => make(&["10 % {} 50"], &["10% off 50"]),
        "as_a_percent_of" => make(&["25 {} 50"], &["25 as a % of 50"]),
        "as_a_percent_on" => make(&["75 {} 50"], &["75 as a % on 50"]),
        "as_a_percent_off" => make(&["25 {} 50"], &["25 as a % off 50"]),
        "percent_of_what_is" => make(&["20 % {} 10"], &["20% of what is 10"]),
        "percent_on_what_is" => make(&["25 % {} 125"], &["25% on what is 125"]),
        "percent_off_what_is" => make(&["20 % {} 80"], &["20% off what is 80"]),
        "thousand" => make(&["2 {}"], &["2000"]),
        "million" => make(&["2 {}"], &["2000000"]),
        "billion" => make(&["2 {}"], &["2000000000"]),
        "today" => make(&["{}"], &["today"]),
        "tomorrow" => make(&["{}"], &["tomorrow"]),
        "yesterday" => make(&["{}"], &["yesterday"]),
        "current_time" => make(&["{}"], &["now"]),
        "local_time" => make(&["2:00 pm PST in {}"], &["2:00 pm PST in Europe/Berlin"]),
        "hex" => make(&["255 in {}"], &["255 in hex"]),
        "binary" => make(&["255 in {}"], &["255 in bin"]),
        "octal" => make(&["255 in {}"], &["255 in oct"]),
        "decimal" => make(&["0xFF in {}"], &["0xFF in dec"]),
        "scientific" => make(&["5300 in {}"], &["5300 in sci"]),
        _ => return None,
    })
}

fn is_currency_code(text: &str) -> bool {
    (3..=5).contains(&text.len()) && text.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// The last line's answer of a document.
fn last_answer(engine: &Engine, lines: &[String]) -> Option<Answer> {
    match &engine.evaluate(&lines.join("\n")).last()?.result {
        LineResult::Value(answer) => Some(answer.clone()),
        _ => None,
    }
}

/// Whether two answers mean the same: same number and unit, or the same text for dates.
fn same(a: &Answer, b: &Answer) -> bool {
    match (a.number, b.number) {
        (Some(x), Some(y)) => a.unit == b.unit && (x - y).abs() <= 1e-9 * x.abs().max(y.abs()).max(1.0),
        (None, None) => a.text == b.text,
        _ => false,
    }
}

/// Reads an Apple `.strings` file: `"key" = "value";` pairs, comments skipped.
fn parse_strings(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    let mut strings = Vec::new();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'*') => {
                let mut last = ' ';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut value = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => match chars.next() {
                            Some('n') => value.push('\n'),
                            Some(other) => value.push(other),
                            None => {}
                        },
                        '"' => break,
                        other => value.push(other),
                    }
                }
                strings.push(value);
            }
            ';' => {
                if let [key, value] = strings.as_slice() {
                    out.push((key.clone(), value.clone()));
                }
                strings.clear();
            }
            _ => {}
        }
    }
    out
}

fn read_text(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_default();
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        let little = bytes[0] == 0xFF;
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                if little {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            })
            .collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Splits a variants list: commas (also full-width) and the Japanese list comma.
fn split_phrases(value: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for phrase in value.split([',', '，', '、']) {
        let phrase = phrase.split_whitespace().collect::<Vec<_>>().join(" ");
        if !phrase.is_empty() && !out.contains(&phrase) {
            out.push(phrase);
        }
    }
    out
}

fn strings_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("NAPKIN_NUMI_STRINGS").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/numi-dmg/engine-strings"),
        PathBuf::from,
    );
    dir.is_dir().then_some(dir)
}

/// A listed miss: (key, phrase) → reason.
fn known_misses(tag: &str) -> BTreeMap<(String, String), String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/numi_compat/{tag}.txt"));
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let (key, phrase, reason) = (parts.next()?, parts.next()?, parts.next()?);
            Some(((key.to_owned(), phrase.to_owned()), reason.to_owned()))
        })
        .collect()
}

/// The outcome for one language.
#[derive(Default)]
struct Coverage {
    total: usize,
    accepted: usize,
    listed: usize,
    unexpected: Vec<String>,
    stale: Vec<String>,
    sample_errors: Vec<String>,
}

fn check_language(dir: &Path, language: InputLanguage) -> Coverage {
    let mut engine = common::engine();
    let mut settings = engine.settings().clone();
    settings.input_language = language;
    settings.number_style = NumberStyle::English;
    engine.set_settings(settings);
    let misses = known_misses(language.tag());
    let mut coverage = Coverage::default();
    let mut seen_misses = Vec::new();
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| entries.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.sort();
    for file in files {
        for (key, value) in parse_strings(&read_text(&file)) {
            if key == "sample.welcome" {
                let outputs = engine.evaluate(&value);
                for (line, output) in value.lines().zip(outputs) {
                    if let LineResult::Error(error) = output.result {
                        coverage.sample_errors.push(format!("{line:?}: {error}"));
                    }
                }
                continue;
            }
            if !key.ends_with(".variants") {
                continue;
            }
            for phrase in split_phrases(&value) {
                coverage.total += 1;
                let ok = probe(&key, &phrase).is_some_and(|probe| {
                    let reference = last_answer(&engine, &probe.reference);
                    let got = last_answer(&engine, &probe.lines);
                    matches!((&got, &reference), (Some(got), Some(want)) if same(got, want))
                });
                let listed = misses.contains_key(&(key.clone(), phrase.clone()));
                match (ok, listed) {
                    (true, false) => coverage.accepted += 1,
                    (true, true) => coverage.stale.push(format!("{key}\t{phrase}")),
                    (false, true) => {
                        coverage.listed += 1;
                        seen_misses.push((key.clone(), phrase.clone()));
                    }
                    (false, false) => {
                        let detail = probe(&key, &phrase).map_or_else(
                            || "no probe for this key".to_owned(),
                            |probe| {
                                let got = engine.evaluate(&probe.lines.join("\n"));
                                format!("{:?} → {:?}", probe.lines, got.last().map(|o| &o.result))
                            },
                        );
                        coverage.unexpected.push(format!("{key}\t{phrase}\t{detail}"));
                    }
                }
            }
        }
    }
    for (key, phrase) in misses.keys() {
        if !seen_misses.iter().any(|(k, p)| k == key && p == phrase)
            && !coverage.stale.iter().any(|s| *s == format!("{key}\t{phrase}"))
        {
            coverage
                .stale
                .push(format!("{key}\t{phrase} (not in Numi's list)"));
        }
    }
    coverage
}

#[test]
fn numi_vocabulary_is_understood() {
    let Some(root) = strings_dir() else {
        println!("skipped: Numi's engine strings are not present (tmp/numi-dmg/engine-strings)");
        return;
    };
    let only = std::env::var("NAPKIN_COMPAT_LANG").ok();
    let mut report = String::from(
        "| Language | Accepted | Total | Coverage | Explained misses |\n|---|---|---|---|---|\n",
    );
    let mut failures = String::new();
    for language in InputLanguage::ALL {
        if only.as_deref().is_some_and(|tag| tag != language.tag()) {
            continue;
        }
        let dir = root.join(format!("{}.lproj", language.tag()));
        if !dir.is_dir() {
            continue;
        }
        let coverage = check_language(&dir, language);
        let percent = if coverage.total == 0 {
            100.0
        } else {
            100.0 * f64::from(u32::try_from(coverage.accepted).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(coverage.total).unwrap_or(u32::MAX))
        };
        let _ = writeln!(
            report,
            "| {} | {} | {} | {percent:.1}% | {} |",
            language.tag(),
            coverage.accepted,
            coverage.total,
            coverage.listed
        );
        for (title, lines) in [
            ("unexpected misses", &coverage.unexpected),
            ("listed misses that pass or are gone", &coverage.stale),
            ("sample.welcome errors", &coverage.sample_errors),
        ] {
            if !lines.is_empty() {
                let _ = writeln!(failures, "\n{} {title} ({}):", language.tag(), lines.len());
                for line in lines {
                    let _ = writeln!(failures, "  {line}");
                }
            }
        }
    }
    println!("{report}");
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/numi-compat-coverage.md");
    let _ = fs::write(&out, format!("{report}{failures}"));
    assert!(failures.is_empty(), "{failures}");
}
