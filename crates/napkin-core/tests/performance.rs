//! A realistic 500-line document evaluates well under 10 ms in release builds.
//!
//! Run with `cargo test -p napkin-core --release --test performance -- --ignored`.

mod common;

use std::time::{Duration, Instant};

use napkin_core::LineResult;

/// Lines typical of a Numi note: budgets, conversions, dates, prose and comments.
const LINES: &[&str] = &[
    "# Trip budget",
    "Flights: $1,240 + 2 × €180",
    "Hotel: 4 nights times 95 EUR",
    "Food: 30 EUR per day * 5 days",
    "Total: sum",
    "Discounted: prev - 12% discount",
    "",
    "rate = 85 EUR",
    "worked = 37.5",
    "rate * worked in USD",
    "// travel time",
    "2:30 pm HKT in Berlin",
    "New York time",
    "today + 3 weeks",
    "fromunix(1446587186)",
    "1 mile in km",
    "20 inches in cm",
    "5 feet 10 inches in cm",
    "100 °C in F",
    "3 GB in MB",
    "60 km / 2 h",
    "1 hour 30 min in hours",
    "sqrt 16 + log 2 (10) + sin 30",
    "$50 as a % of $200",
    "5% of what is 6 EUR",
    "Buy 2 cups of flour and 3 eggs",
    "The \"Model 227\" costs $275",
    "0x1F + 0b101 in hex",
    "em = 18px",
    "1.5 em in px",
    "",
];

#[test]
#[ignore = "timing check; run in release with --ignored"]
fn five_hundred_lines_in_under_ten_milliseconds() {
    let engine = common::engine();
    let document: Vec<&str> = LINES.iter().copied().cycle().take(500).collect();
    let document = document.join("\n");
    let warm = engine.evaluate(&document);
    assert_eq!(warm.len(), 500);
    let errors: Vec<&str> = document
        .split('\n')
        .zip(&warm)
        .filter(|(_, out)| matches!(out.result, LineResult::Error(_)))
        .map(|(line, _)| line)
        .collect();
    assert!(
        errors.is_empty(),
        "the benchmark document should compute cleanly: {errors:?}"
    );
    let mut timings: Vec<Duration> = (0..21)
        .map(|_| {
            let start = Instant::now();
            let lines = engine.evaluate(&document);
            let elapsed = start.elapsed();
            assert_eq!(lines.len(), 500);
            elapsed
        })
        .collect();
    timings.sort();
    let median = timings[timings.len() / 2];
    println!("500 lines: median {median:?}, best {:?}", timings[0]);
    assert!(
        median < Duration::from_millis(10),
        "median {median:?} is too slow"
    );
}
