//! The acceptance examples table of `docs/spec.md`, row by row.

mod common;

use napkin_core::LineResult;

/// What a line of a row must evaluate to.
#[derive(Debug, Clone, Copy)]
enum Expect {
    /// A value with this text.
    Text(&'static str),
    /// `LineResult::None`.
    Nothing,
    /// Not checked (the spec shows `…`).
    Any,
}

use Expect::{Any, Nothing, Text};

/// Each row is one document: its lines and what each line shows.
const TABLE: &[(&[&str], &[Expect])] = &[
    (&["8 times 9"], &[Text("72")]),
    (&["6 (3)"], &[Text("18")]),
    (&["1 meter 20 cm"], &[Text("120 cm")]),
    (&["20 inches in cm"], &[Text("50.8 cm")]),
    (&["1 mile in km"], &[Text("1.61 km")]),
    (&["0b110111011"], &[Text("443")]),
    (&["0o1435343 in hex"], &[Text("0x63AE3")]),
    (&["5 300 in sci"], &[Text("5.3e3")]),
    (&["$30 in Euro"], &[Text("€24")]),
    (&["$30 CAD + 5 USD - 7EUR"], &[Text("€16.20")]),
    (&["$2k"], &[Text("$2,000")]),
    (&["2M eur"], &[Text("€2,000,000")]),
    (
        &["v = $20", "v2 = 5%", "v times 7 - v2", "v + $10"],
        &[Text("$20"), Text("5%"), Text("$133"), Text("$30")],
    ),
    (&["12 pt in px"], &[Text("16 px")]),
    (&["em = 20px", "1.2 em in px"], &[Text("20 px"), Text("24 px")]),
    (&["ppi = 326", "1 cm in px"], &[Text("326"), Text("128.35 px")]),
    (
        &["Line 1: $10", "Line 2: $15", "Result: sum"],
        &[Text("$10"), Text("$15"), Text("$25")],
    ),
    (
        &["Line 1: $10", "Line 2: $20", "Result: average"],
        &[Any, Any, Text("$15")],
    ),
    (
        &["Cost: $20 + 56 EUR", "Discounted: prev - 5% discount"],
        &[Text("€72"), Text("€68.40")],
    ),
    (&["round(1 month in days)"], &[Text("30")]),
    (&["1 month in days"], &[Text("30.42 days")]),
    (
        &["sqrt 16", "cbrt 8", "root 2 (8)"],
        &[Text("4"), Text("2"), Text("2.83")],
    ),
    (
        &["abs(-4)", "log 2 (10)", "ln 3", "fact 5"],
        &[Text("4"), Text("3.32"), Text("1.1"), Text("120")],
    ),
    (
        &["round 3.45", "ceil 3.76", "floor 2.56"],
        &[Text("3"), Text("4"), Text("2")],
    ),
    (
        &["sin 30", "sin 45°", "sinh 3"],
        &[Text("0.5"), Text("0.71"), Text("10.02")],
    ),
    (
        &["20 sq cm", "11 sqm", "20 cu cm", "11 cbm"],
        &[Text("20 cm²"), Text("11 m²"), Text("20 cm³"), Text("11 m³")],
    ),
    (
        &["3 GB in MB", "1 GiB in MB"],
        &[Text("3,000 MB"), Text("1,073.74 MB")],
    ),
    (
        &["100 °C in F", "0 K in C"],
        &[Text("212 °F"), Text("-273.15 °C")],
    ),
    (&["60 km / 2 h"], &[Text("30 km/h")]),
    (&["# This is header", "// This is comment"], &[Nothing, Nothing]),
    (&["$275 for the \"Model 227\""], &[Text("$275")]),
    (&["Price: $11 + $34.45"], &[Text("$45.45")]),
    (&["now", "time"], &[Text("13:00"), Text("13:00")]),
    (
        &["today", "today + 2 weeks"],
        &[Text("Wed, 28 Oct 2026"), Text("Wed, 11 Nov 2026")],
    ),
    (
        &["PST time", "New York time", "Time in Madrid"],
        &[Text("05:00"), Text("08:00"), Text("13:00")],
    ),
    (&["2:30 pm HKT in Berlin"], &[Text("07:30")]),
    (&["fromunix(1446587186)"], &[Text("Tue, 3 Nov 2015 22:46")]),
    (&["3:30 pm + 2 hours"], &[Text("17:30")]),
    (&["1 BTC in USD"], &[Text("$100,000")]),
    (&["$50 in BTC"], &[Text("0.0005 BTC")]),
];

#[test]
fn acceptance_table() {
    let engine = common::engine();
    let mut failures = Vec::new();
    let mut checked = 0;
    for (lines, expected) in TABLE {
        assert_eq!(lines.len(), expected.len(), "row {lines:?} is malformed");
        let outputs = engine.evaluate(&lines.join("\n"));
        assert_eq!(outputs.len(), lines.len(), "one output per line for {lines:?}");
        for ((line, want), output) in lines.iter().zip(expected.iter()).zip(&outputs) {
            checked += 1;
            let ok = match (want, &output.result) {
                (Text(text), LineResult::Value(answer)) => answer.text == *text,
                (Nothing, LineResult::None) | (Any, _) => true,
                _ => false,
            };
            if !ok {
                failures.push(format!("{line:?}: want {want:?}, got {:?}", output.result));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {checked} lines failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
