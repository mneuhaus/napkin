//! Per-language acceptance tables (the spec's fixed clock and test rates) and the sample note
//! of every language.

#[path = "../common/mod.rs"]
mod common;

mod cs;
mod de;
mod es;
mod fr;
mod he;
mod it;
mod ja;
mod ko;
mod nl;
mod nordic;
mod pl;
mod pt;
mod ro;
mod ru;
mod tr;
mod zh;

use napkin_core::{Engine, InputLanguage, LineResult, NumberStyle};

/// What a line of a row must evaluate to.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Expect {
    /// A value with this text.
    Text(&'static str),
    /// `LineResult::None`.
    Nothing,
    /// `LineResult::Error`.
    Fails,
}

/// Rows of a table: the lines of one document and what each line shows.
pub(crate) type Table = &'static [(&'static [&'static str], &'static [Expect])];

/// The spec's test engine with `language` selected and numbers in `style`.
pub(crate) fn engine(language: InputLanguage, style: NumberStyle) -> Engine {
    let mut engine = common::engine();
    let mut settings = engine.settings().clone();
    settings.input_language = language;
    settings.number_style = style;
    engine.set_settings(settings);
    engine
}

/// Evaluates every row of `table` as its own document and reports every line that differs.
pub(crate) fn assert_table(language: InputLanguage, style: NumberStyle, table: Table) {
    let engine = engine(language, style);
    let mut failures = Vec::new();
    let mut checked = 0;
    for (lines, expected) in table {
        assert_eq!(lines.len(), expected.len(), "row {lines:?} is malformed");
        let outputs = engine.evaluate(&lines.join("\n"));
        for ((line, want), output) in lines.iter().zip(expected.iter()).zip(&outputs) {
            checked += 1;
            let ok = match (want, &output.result) {
                (Expect::Text(text), LineResult::Value(answer)) => answer.text == *text,
                (Expect::Nothing, LineResult::None) | (Expect::Fails, LineResult::Error(_)) => true,
                _ => false,
            };
            if !ok {
                failures.push(format!("{line:?}: want {want:?}, got {:?}", output.result));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{}: {} of {checked} lines failed:\n{}",
        language.tag(),
        failures.len(),
        failures.join("\n")
    );
}

/// Whether a sample line is a calculation (not empty, a header or a comment).
fn is_calculation(line: &str) -> bool {
    let line = line.trim();
    !(line.is_empty() || line.starts_with(['#', '＃']) || line.starts_with("//"))
}

#[test]
fn sample_notes_compute_every_line() {
    let mut failures = Vec::new();
    for language in InputLanguage::ALL {
        let note = language.sample_note();
        let outputs = engine(language, NumberStyle::English).evaluate(note);
        for (line, output) in note.lines().zip(&outputs) {
            if is_calculation(line) && !matches!(output.result, LineResult::Value(_)) {
                failures.push(format!("{}: {line:?} gave {:?}", language.tag(), output.result));
            }
        }
        assert!(
            note.lines().filter(|line| is_calculation(line)).count() >= 3,
            "{}: the sample note needs a few calculations",
            language.tag()
        );
    }
    assert!(
        failures.is_empty(),
        "sample lines without a value:\n{}",
        failures.join("\n")
    );
}
