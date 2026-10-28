//! Shared setup: the spec's fixed clock and test rates.

use chrono::{TimeZone, Utc};
use napkin_core::{Clock, Engine, ExchangeRates};

/// The spec's test rates: `USD 1, EUR 0.8, GBP 0.75, CAD 1.25, JPY 150, CHF 0.9, BTC 0.00001`.
fn rates() -> ExchangeRates {
    let rates = [
        ("USD", 1.0),
        ("EUR", 0.8),
        ("GBP", 0.75),
        ("CAD", 1.25),
        ("JPY", 150.0),
        ("CHF", 0.9),
        ("BTC", 0.00001),
    ];
    ExchangeRates {
        base: "USD".to_owned(),
        updated: None,
        rates: rates
            .into_iter()
            .map(|(code, rate)| (code.to_owned(), rate))
            .collect(),
    }
}

/// An engine with default settings, the spec's fixed clock (2026-10-28 12:00 UTC, Berlin) and
/// test rates.
pub(crate) fn engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_clock(Clock::Fixed {
        now: Utc
            .with_ymd_and_hms(2026, 10, 28, 12, 0, 0)
            .single()
            .expect("valid instant"),
        time_zone: chrono_tz::Europe::Berlin,
    });
    engine.set_exchange_rates(rates());
    engine
}
