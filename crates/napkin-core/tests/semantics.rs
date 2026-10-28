//! Lexer, parser and document edge cases, checked through the public API.

mod common;

use napkin_core::{AngleUnit, Engine, LineResult, NumberStyle, Settings, SpanKind, TimeFormat};

/// What the last line of `document` shows: its text, `error: ...` or `none`.
fn last(engine: &Engine, document: &str) -> String {
    match engine.evaluate(document).pop().map(|line| line.result) {
        Some(LineResult::Value(answer)) => answer.text,
        Some(LineResult::Error(error)) => format!("error: {error}"),
        Some(LineResult::None) | None => "none".to_owned(),
    }
}

/// Checks single-line documents against expected results.
fn check(engine: &Engine, cases: &[(&str, &str)]) {
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(input, want)| {
            let got = last(engine, input);
            (got != *want).then(|| format!("{input:?}: want {want:?}, got {got:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn is_error(engine: &Engine, input: &str) -> bool {
    matches!(
        engine.evaluate(input).pop().map(|l| l.result),
        Some(LineResult::Error(_))
    )
}

#[test]
fn prose_is_ignored() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("3 apples + 4 apples", "7"),
            ("I have 3 cats and 2 dogs", "5"),
            ("Buy 2 cups of flour and 3 eggs", "5 cups"),
            ("Discounted: 100 - 5% discount", "95"),
            ("$275 for the \"Model 227\"", "$275"),
            ("The result is 5 +", "5"),
            ("(1 + 2", "3"),
        ],
    );
}

#[test]
fn lines_without_numbers_are_none() {
    let engine = common::engine();
    for input in [
        "hello world",
        "in the morning",
        "I live in Berlin",
        "a = b",
        "",
        "   ",
        "Notes:",
        "// 5 + 5",
    ] {
        assert_eq!(engine.evaluate(input)[0].result, LineResult::None, "{input:?}");
    }
}

#[test]
fn failing_calculations_are_errors() {
    let engine = common::engine();
    let inputs = [
        "5 m + 3 kg",
        "10 / 0",
        "10 mod 0",
        "5 kg in km",
        "fact -1",
        "sin 5 kg",
        "m = 5",
        "b = 2",
        "100 rubles in USD",
    ];
    for input in inputs {
        assert!(is_error(&engine, input), "{input:?} should be an error");
    }
}

#[test]
fn labels_versus_times() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("Price: $11 + $34.45", "$45.45"),
            ("Line 1: $10", "$10"),
            ("2:30 pm", "14:30"),
            ("Meeting: 2:30 pm", "14:30"),
            ("at 14:30", "14:30"),
            ("Ratio:2", "2"),
        ],
    );
    let spans = &engine.evaluate("Meeting: 2:30 pm")[0].spans;
    assert_eq!(spans[0].kind, SpanKind::Label);
    assert_eq!(spans[0].range, 0..8);
    assert_eq!(spans[1].kind, SpanKind::DateTime);
}

#[test]
fn in_is_a_keyword_and_inches_are_words() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("20 inches in cm", "50.8 cm"),
            ("1 inch in px", "96 px"),
            ("2 in", "2"),
            ("5 feet 10 inches", "70 inches"),
            ("30 square inches", "30 square inches"),
        ],
    );
}

#[test]
fn case_sensitive_one_letter_units_and_scales() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("5 m", "5 m"),
            ("5 M", "5,000,000"),
            ("5 MB", "5 MB"),
            ("5 Mb in MB", "0.62 MB"),
            ("5 mm", "5 mm"),
            ("2k", "2,000"),
            ("300 K in C", "26.85 °C"),
            ("3 thousand", "3,000"),
            ("1.5 billion", "1,500,000,000"),
            ("2 bn", "2,000,000,000"),
        ],
    );
}

#[test]
fn currency_codes_follow_case_rules() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("2 cup in ml", "473.18 ml"),
            ("2 CUP", "2 CUP"),
            ("5 eur", "€5"),
            ("5 Usd", "$5"),
            ("5 all", "5"),
            ("5 ALL", "5 ALL"),
            ("5 btc", "5"),
            ("5 BTC", "5 BTC"),
            ("EUR 30", "€30"),
            ("30 €", "€30"),
            ("$30 CAD", "30 CAD"),
            ("5 pounds in kg", "2.27 kg"),
            ("£5 + 3 pounds", "£8"),
            ("1000 JPY in EUR", "€5.33"),
            ("10 CHF", "10 CHF"),
        ],
    );
}

#[test]
fn comma_decimal_input_in_german_style() {
    let mut engine = common::engine();
    engine.set_settings(Settings {
        number_style: NumberStyle::German,
        ..Settings::default()
    });
    check(
        &engine,
        &[
            ("1,5 + 1", "2,5"),
            ("1.234,5 + 0,5", "1.235"),
            ("2,5 km in m", "2.500 m"),
            ("log(2; 8)", "3"),
            ("12.03.2026", "Thu, 12 Mar 2026"),
            ("3/12/2026", "Thu, 3 Dec 2026"),
            ("€1.234,50", "€1.234,50"),
        ],
    );
    engine.set_settings(Settings {
        number_style: NumberStyle::Swiss,
        ..Settings::default()
    });
    check(&engine, &[("1'234.5 * 2", "2'469")]);
}

#[test]
fn implicit_multiplication_and_precedence() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("6 (3)", "18"),
            ("2(3+4)", "14"),
            ("2 pi", "6.28"),
            ("6 / 2(3)", "1"),
            ("1 + 2 * 3", "7"),
            ("2^3^2", "512"),
            ("-2^2", "4"),
            ("2 ** 3", "8"),
            ("10 - 2 - 3", "5"),
            ("1 + 2 << 1", "6"),
            ("6 & 3 | 8", "10"),
            ("6 xor 3", "5"),
            ("10 mod 4 + 1", "3"),
            ("5!", "120"),
            ("3 x 4", "12"),
            ("8 multiplied by 2", "16"),
            ("8 divided by 2", "4"),
            ("10 without 3", "7"),
            ("1 meter 20 cm", "120 cm"),
            ("1 hour 30 min", "90 min"),
            ("sqrt 16 + 9", "13"),
        ],
    );
}

#[test]
fn units_and_dimensions() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("1 meter + 20 cm", "120 cm"),
            ("$5 + 3", "$8"),
            ("3 m * 4 m", "12 m²"),
            ("10 m² / 2 m", "5 m"),
            ("10 m / 2 cm", "500"),
            ("60 km / 2 h", "30 km/h"),
            ("100 km/h in mph", "62.14 mph"),
            ("5 m/s in km/h", "18 km/h"),
            ("$25/h * 8 h", "$200"),
            ("1 GiB in MB", "1,073.74 MB"),
            ("1 kB in b", "8,000 b"),
            ("2 tbsp in tsp", "6 tsp"),
            ("1 ha in m²", "10,000 m²"),
            ("1 km2 in m2", "1,000,000 m²"),
            ("1 acre in sqft", "43,560 square feet"),
            ("1 nautical mile in km", "1.85 km"),
            ("2 ms in s", "0.002 s"),
            ("1 hour in seconds", "3,600 seconds"),
            ("1 year in months", "12 months"),
            ("70 °F in °C", "21.11 °C"),
            ("1 kHz in Hz", "1,000 Hz"),
            ("1 em in pt", "12 pt"),
            ("asin 0.5", "30°"),
            ("cos 180", "-1"),
        ],
    );
}

#[test]
fn percentages() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("$10 - 40%", "$6"),
            ("$10 + 40%", "$14"),
            ("20% of $10", "$2"),
            ("5% on $30", "$31.50"),
            ("6% off 40 EUR", "€37.60"),
            ("$50 as a % of $100", "50%"),
            ("$70 as a % on $20", "250%"),
            ("$20 as a % off $70", "71.43%"),
            ("5% of what is 6 EUR", "€120"),
            ("5% on what is 6 EUR", "€5.71"),
            ("5% off what is 6 EUR", "€6.32"),
            ("200 * 5%", "10"),
            ("5% * 2", "10%"),
            ("0.25 in %", "25%"),
        ],
    );
}

#[test]
fn number_formats() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("0x1F", "31"),
            ("0b101 + 1", "6"),
            ("255 in hex", "0xFF"),
            ("255 in binary", "0b11111111"),
            ("255 in octal", "0o377"),
            ("1e3", "1,000"),
            ("2.5E-4 in decimal", "0.00025"),
            ("2.5E-4 in sci", "2.5e-4"),
            ("1234 in scientific", "1.23e3"),
            ("1,000,000 + 1", "1,000,001"),
            ("5 300 + 1", "5,301"),
            ("1/3", "0.33"),
        ],
    );
}

#[test]
fn settings_change_results() {
    let mut engine = common::engine();
    engine.set_settings(Settings {
        precision: 4,
        ..Settings::default()
    });
    check(&engine, &[("1/3", "0.3333")]);
    engine.set_settings(Settings {
        angle_unit: AngleUnit::Radians,
        ..Settings::default()
    });
    check(&engine, &[("sin 30", "-0.99"), ("sin 90°", "1")]);
    engine.set_settings(Settings {
        time_format: TimeFormat::H12,
        ..Settings::default()
    });
    check(&engine, &[("now", "1:00 pm"), ("14:30 + 1 hour", "3:30 pm")]);
}

#[test]
fn dates_and_time_zones() {
    let engine = common::engine();
    check(
        &engine,
        &[
            ("tomorrow", "Thu, 29 Oct 2026"),
            ("today + 1 month", "Sat, 28 Nov 2026"),
            ("2026-12-24 - today", "57 days"),
            ("March 12, 2026", "Thu, 12 Mar 2026"),
            ("12 March", "Thu, 12 Mar 2026"),
            ("2026-03-12 14:30", "Thu, 12 Mar 2026 14:30"),
            ("11pm + 3 hours", "Thu 02:00"),
            ("14:30 - 12:00", "2.5 hours"),
            ("noon", "12:00"),
            ("New York time - London time", "-4 hours"),
            ("Europe/Berlin time", "13:00"),
            ("now in Tokyo", "21:00"),
            ("San Francisco time", "05:00"),
            ("Hong Kong time", "20:00"),
            ("IST time", "17:30"),
            ("Meeting at 3pm in New York", "10:00"),
        ],
    );
}

#[test]
fn variables_prev_sum_and_average() {
    let engine = common::engine();
    let doc = "a = 10\nqty = a * 2\nqty + a\nprev / 3\n\nx = 5 cm\n20 cm\ntotal\naverage";
    let texts: Vec<String> = engine
        .evaluate(doc)
        .into_iter()
        .map(|line| match line.result {
            LineResult::Value(answer) => answer.text,
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        texts,
        [
            "10", "20", "30", "10", "None", "5 cm", "20 cm", "25 cm", "16.67 cm"
        ]
    );
    assert_eq!(
        last(&engine, "Foo = 1\nfoo"),
        "none",
        "variables are case-sensitive"
    );
    assert_eq!(
        last(&engine, "# Block\n$10\n# Next\nsum"),
        "0",
        "headers end a block"
    );
}

#[test]
fn spans_are_sorted_disjoint_and_inside_the_line() {
    let engine = common::engine();
    let doc = "# Title\nPrice: $11 + $34.45 // note\n$275 for the \"Model 227\"\nv = 20 km/h in mph\n\
               2:30 pm HKT in Berlin\r\nsqrt 16 + pi\n1 nautical mile in km\nÜber 5 €";
    for (line, output) in doc.split('\n').zip(engine.evaluate(doc)) {
        let mut end = 0;
        for span in &output.spans {
            assert!(span.range.start >= end, "{line:?}: {:?} overlaps", output.spans);
            assert!(
                span.range.end <= line.trim_end_matches('\r').len(),
                "{line:?}: span past the end"
            );
            assert!(line.is_char_boundary(span.range.start) && line.is_char_boundary(span.range.end));
            end = span.range.end;
        }
    }
    let spans = &engine.evaluate("v = 20 km/h in mph")[0].spans;
    let kinds: Vec<SpanKind> = spans.iter().map(|s| s.kind).collect();
    let want = [
        SpanKind::Variable,
        SpanKind::Operator,
        SpanKind::Number,
        SpanKind::Unit,
        SpanKind::Operator,
        SpanKind::Unit,
        SpanKind::Keyword,
        SpanKind::Unit,
    ];
    assert_eq!(kinds, want);
    assert_eq!(engine.evaluate("# Title")[0].spans[0].kind, SpanKind::Header);
    let comment = &engine.evaluate("5 // note")[0].spans;
    assert_eq!(comment.last().map(|s| s.kind), Some(SpanKind::Comment));
}

#[test]
fn answers_carry_number_and_unit() {
    let engine = common::engine();
    let LineResult::Value(answer) = engine.evaluate("20 inches in cm").remove(0).result else {
        panic!("expected a value");
    };
    assert_eq!(answer.unit.as_deref(), Some("cm"));
    assert!(answer.number.is_some_and(|n| (n - 50.8).abs() < 1e-9));
    let LineResult::Value(money) = engine.evaluate("$5").remove(0).result else {
        panic!("expected a value")
    };
    assert_eq!(money.unit.as_deref(), Some("USD"));
    let LineResult::Value(date) = engine.evaluate("today").remove(0).result else {
        panic!("expected a value")
    };
    assert_eq!((date.number, date.unit), (None, None));
}

#[test]
fn evaluate_expression_reports_errors_and_nothing() {
    let engine = common::engine();
    assert_eq!(
        engine.evaluate_expression("20 inches in cm").map(|a| a.text),
        Ok("50.8 cm".to_owned())
    );
    assert!(engine.evaluate_expression("5 m + 3 kg").is_err());
    assert!(engine.evaluate_expression("hello").is_err());
}
