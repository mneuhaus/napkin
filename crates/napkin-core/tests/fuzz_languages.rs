//! Random soup of every language's own words never panics.
//!
//! The pieces come from each language's vocabulary file and sample note (words, and single characters for scripts
//! written without spaces) plus numbers and operators, glued with and without spaces. A panic in
//! the engine would take the whole app down while the user types, so every failing input is
//! collected and reported at once.
//!
//! `NAPKIN_FUZZ_ROUNDS=<n>` raises the number of documents per language for a deeper run.

mod common;

use std::panic::{AssertUnwindSafe, catch_unwind};

use napkin_core::{InputLanguage, NumberStyle, Settings};

const GENERIC: &[&str] = &[
    "1", "12", "3.5", "2,5", "1 000", "0", "-", "+", "*", "/", "%", "(", ")", "=", ":", ".", ",", "$", "€",
    "¥", "in", "to", "of", "prev", "sum", "14:30", "2026", "１２", "＋", "　", "-3", "x",
];

/// A small xorshift generator, so failures reproduce.
struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        usize::try_from(self.0 % u64::try_from(bound).expect("small bound")).expect("fits")
    }
}

/// Every phrase of the language's vocabulary file, the words of its sample note, every single
/// non-ASCII character, and the generic pieces.
fn pieces(language: InputLanguage) -> Vec<String> {
    let note = language.sample_note();
    let path = format!("{}/lang/{}.txt", env!("CARGO_MANIFEST_DIR"), language.tag());
    let vocabulary = std::fs::read_to_string(path).unwrap_or_default();
    let phrases = vocabulary
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once(" = ").map(|(_, value)| value))
        .flat_map(|value| value.split([',', '|']))
        .map(|phrase| phrase.split('{').next().unwrap_or_default().trim().to_owned())
        .filter(|phrase| !phrase.is_empty());
    let mut pieces: Vec<String> = note
        .split_whitespace()
        .map(str::to_owned)
        .chain(phrases)
        .chain(
            note.chars()
                .filter(|c| !c.is_ascii() && !c.is_whitespace())
                .map(String::from),
        )
        .chain(GENERIC.iter().map(|piece| (*piece).to_owned()))
        .collect();
    pieces.sort();
    pieces.dedup();
    pieces
}

#[test]
fn every_language_survives_random_input() {
    let rounds: usize = std::env::var("NAPKIN_FUZZ_ROUNDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(400);
    let mut failures = Vec::new();
    for (seed, language) in (1_u64..).zip(InputLanguage::ALL) {
        let mut engine = common::engine();
        for number_style in [NumberStyle::English, NumberStyle::German] {
            engine.set_settings(Settings {
                input_language: language,
                number_style,
                ..Settings::default()
            });
            let pieces = pieces(language);
            let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ seed);
            for _ in 0..rounds {
                let count = 1 + rng.below(7);
                let separator = if rng.below(2) == 0 { " " } else { "" };
                let line = (0..count)
                    .map(|_| pieces[rng.below(pieces.len())].as_str())
                    .collect::<Vec<_>>()
                    .join(separator);
                let document = format!("{line}\nprev\n{line} in cm");
                if catch_unwind(AssertUnwindSafe(|| engine.evaluate(&document))).is_err() {
                    failures.push(format!("{}: {line:?}", language.tag()));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} inputs panicked:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
