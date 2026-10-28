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
            ("5 Mb in MB", "0.63 MB"),
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

/// Fragments the fuzz test glues together.
const PIECES: &[&str] = &[
    "1",
    "0",
    "5 300",
    "2.5",
    "1e400",
    "0x",
    "0xFF",
    "0b2",
    "12:3",
    "2:30",
    "pm",
    "am",
    "12.03.2026",
    "3/12/",
    "2026-13-45",
    "$",
    "€",
    "₿",
    "%",
    "+",
    "-",
    "*",
    "/",
    "^",
    "**",
    "(",
    ")",
    "[",
    "!",
    ";",
    "=",
    "²",
    "³",
    "°",
    "°C",
    "K",
    "k",
    "M",
    "m",
    "in",
    "to",
    "as",
    "of",
    "what",
    "is",
    "on",
    "off",
    "sq",
    "cubic",
    "sqrt",
    "log",
    "root",
    "fromunix",
    "fact",
    "round",
    "today",
    "now",
    "time",
    "New",
    "York",
    "Berlin",
    "PST",
    "HKT",
    "Europe",
    "/",
    "Berlin",
    "EUR",
    "usd",
    "CUP",
    "cup",
    "pound",
    "sum",
    "avg",
    "prev",
    "pi",
    "e",
    "x",
    "v",
    "\"",
    "“",
    "//",
    ":",
    ",",
    ".",
    "#",
    "ü",
    "日本",
    "🙂",
    "\t",
    " ",
    "  ",
    "mod",
    "xor",
    "<<",
    ">>",
    "&",
    "|",
    "fl",
    "oz",
    "March",
    "12",
    "2026",
    "hex",
    "sci",
    "a",
    "%",
    "em",
    "ppi",
    "1e-9",
    "999999999999999999",
];

/// Random token soup never panics, and spans stay valid.
#[test]
fn arbitrary_input_never_panics() {
    let engine = common::engine();
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = |bound: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        usize::try_from(state % u64::try_from(bound).expect("small bound")).expect("fits")
    };
    for _ in 0..3000 {
        let count = 1 + next(9);
        let line: Vec<&str> = (0..count).map(|_| PIECES[next(PIECES.len())]).collect();
        let separator = if next(2) == 0 { " " } else { "" };
        let line = line.join(separator);
        let document = format!("{line}\nprev\nsum\n{line} in cm");
        for (text, output) in document.split('\n').zip(engine.evaluate(&document)) {
            for span in &output.spans {
                assert!(span.range.end <= text.len(), "{text:?}: {:?}", output.spans);
                assert!(
                    text.is_char_boundary(span.range.start) && text.is_char_boundary(span.range.end),
                    "{text:?}"
                );
            }
            let mut ends = output.spans.iter().map(|s| (s.range.start, s.range.end));
            let mut last_end = 0;
            assert!(
                ends.all(|(start, end)| {
                    let ok = start >= last_end && start <= end;
                    last_end = end;
                    ok
                }),
                "{text:?}: {:?}",
                output.spans
            );
        }
    }
}

#[test]
fn system_clock_uses_the_local_zone() {
    let engine = Engine::new();
    let outputs = engine.evaluate("now\ntoday\nnow in UTC\n1 USD in EUR");
    for output in &outputs {
        assert!(
            matches!(output.result, LineResult::Value(_)),
            "{:?}",
            output.result
        );
    }
    let LineResult::Value(now) = &outputs[0].result else {
        unreachable!("checked above")
    };
    let (hours, minutes) = now.text.split_once(':').expect("a time of day like 13:00");
    assert!(hours.parse::<u32>().is_ok_and(|h| h < 24) && minutes.parse::<u32>().is_ok_and(|m| m < 60));
}

/// Findings of an adversarial review, pinned.
#[test]
fn review_regressions() {
    let engine = common::engine();
    check(
        &engine,
        &[
            // Cancellation noise is zero.
            ("32 °F in °C", "0 °C"),
            ("1 ft - 12 inches", "0 inches"),
            ("0.1 + 0.2 - 0.3", "0"),
            // Separators between digits are not multiplication.
            ("12,99 EUR + 3,50 EUR", "€16.49"),
            ("3,25", "3.25"),
            ("Ratio 3:4", "none"),
            ("version 1.2.3", "none"),
            ("24:00", "00:00"),
            ("5\u{a0}300", "5,300"),
            ("5 — 3", "2"),
            // Undefined and out-of-range results are errors.
            ("tan 90", "error: tan is undefined there"),
            ("1e30 in hex", "error: the number is out of range for this base"),
            // Zoned times keep their zone next to a date.
            ("2027-01-15 9:00 EST in Berlin", "Fri, 15 Jan 2027 15:00"),
            ("tomorrow 9am EST in Berlin", "Thu, 29 Oct 2026 14:00"),
            // A compound unit after a number is one unit.
            ("30 km / 60 km/h", "0.5 h"),
            ("100 km / 50 km/h in min", "120 min"),
            ("100 GB / 10 MB/s", "10,000 s"),
            ("$5 $5", "$10"),
            // Every SI prefix, and the 2022 ones.
            ("1 megaliter in l", "1,000,000 l"),
            ("1 ks in s", "1,000 s"),
            ("1 mK in K", "0.001 K"),
            ("1 mrad in rad", "0.001 rad"),
            ("1 Qm in Rm", "1,000 Rm"),
            ("1 Pm in Tm", "1,000 Tm"),
            ("11 qm", "11 m²"),
            ("1 mcg in µg", "1 µg"),
            ("1 fortnight in days", "14 days"),
            ("20 ºC in F", "68 °F"),
            // Cities with diacritics and abbreviations with dots.
            ("São Paulo time", "09:00"),
            ("St. Petersburg time", "15:00"),
            ("UTC+2 time", "14:00"),
            ("Etc/GMT+5 time", "07:00"),
            ("50 as a percent of 200", "25%"),
            ("0.0000001 BTC", "0.0000001 BTC"),
            ("prev", "none"),
            ("2abc = 4", "error: `2abc` is not a valid variable name"),
            ("ppi = 2 cm", "error: `ppi` needs a plain number"),
            ("2026-10-28T14:30", "Wed, 28 Oct 2026 14:30"),
            ("now + 2 weeks", "Wed, 11 Nov 2026 13:00"),
            // Prose between numbers does not multiply.
            ("3 apples 4 pears", "3"),
            ("$275 for the Model 227", "$275"),
        ],
    );
    for name in ["k", "hex", "what", "PST", "thousand"] {
        assert!(is_error(&engine, &format!("{name} = 7")), "{name} is a keyword");
    }
    assert_eq!(
        last(&engine, "Berlin = 3\nBerlin time"),
        "13:00",
        "zones win in zone context"
    );
}

/// Pathological lines are refused, not a stack overflow, even on a small thread.
#[test]
fn deep_or_long_lines_do_not_overflow_the_stack() {
    let lines = [
        format!("{}1", "(".repeat(5000)),
        format!("{}5", "-".repeat(5000)),
        format!("1{}", " + 1".repeat(20_000)),
        "5 apples ".repeat(20_000),
        "1,".repeat(50_000),
        format!("{}1{}", "(".repeat(90), ")".repeat(90)),
    ];
    let worker = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            let engine = common::engine();
            lines
                .iter()
                .map(|line| engine.evaluate(line).remove(0).result)
                .collect::<Vec<_>>()
        })
        .expect("thread starts");
    let results = worker.join().expect("no stack overflow");
    assert!(
        matches!(results[5], LineResult::Value(_)),
        "90 levels of parentheses are fine"
    );
}
