//! Parsing the two rate sources and merging them into one [`ExchangeRates`] table.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use napkin_core::ExchangeRates;
use serde::Deserialize;

/// Fiat rates against USD, published once a day, no API key.
pub(crate) const FIAT_URL: &str = "https://open.er-api.com/v6/latest/USD";

/// Crypto (and fiat) rates against USD from Coinbase, no API key.
pub(crate) const CRYPTO_URL: &str = "https://api.coinbase.com/v2/exchange-rates?currency=USD";

/// The base currency of every table this crate produces.
pub(crate) const BASE: &str = "USD";

/// Crypto tickers of the bundled snapshot. Only consulted when the fiat source is down and
/// fiat codes cannot be told apart from crypto tickers by the fresh fiat table; a test keeps it
/// in sync with the snapshot.
const KNOWN_CRYPTO: &[&str] = &[
    "AAVE", "ADA", "ALGO", "APT", "ARB", "ATOM", "AVAX", "BCH", "BNB", "BTC", "DAI", "DASH", "DOGE", "DOT",
    "EOS", "ETC", "ETH", "FIL", "HBAR", "ICP", "LINK", "LTC", "MKR", "NEAR", "OP", "PEPE", "SHIB", "SOL",
    "SUI", "TON", "UNI", "USDC", "USDT", "VET", "XLM", "XRP", "XTZ", "ZEC",
];

/// A fresh fiat table from open.er-api.com.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FiatRates {
    /// When the provider last updated the table.
    pub(crate) updated: Option<DateTime<Utc>>,
    /// Units per one USD by ISO 4217 code.
    pub(crate) rates: BTreeMap<String, f64>,
}

#[derive(Deserialize)]
struct FiatResponse {
    result: String,
    #[serde(rename = "error-type")]
    error_type: Option<String>,
    #[serde(default)]
    base_code: String,
    time_last_update_unix: Option<i64>,
    #[serde(default)]
    rates: BTreeMap<String, f64>,
}

#[derive(Deserialize)]
struct CryptoResponse {
    data: CryptoData,
}

#[derive(Deserialize)]
struct CryptoData {
    currency: String,
    rates: BTreeMap<String, String>,
}

/// Parses an open.er-api.com `/v6/latest/USD` response.
pub(crate) fn parse_fiat(json: &str) -> Result<FiatRates, String> {
    let response: FiatResponse =
        serde_json::from_str(json).map_err(|error| format!("unexpected response: {error}"))?;
    if response.result != "success" {
        let reason = response.error_type.unwrap_or_default();
        return Err(format!("provider answered {:?} {reason}", response.result)
            .trim_end()
            .to_owned());
    }
    if response.base_code != BASE {
        return Err(format!("expected base {BASE}, got {}", response.base_code));
    }
    let rates: BTreeMap<String, f64> = response
        .rates
        .into_iter()
        .filter(|(_, rate)| is_usable_rate(*rate))
        .collect();
    if rates.is_empty() {
        return Err("no usable rates in the response".to_owned());
    }
    let updated = response
        .time_last_update_unix
        .and_then(|secs| DateTime::from_timestamp(secs, 0));
    Ok(FiatRates { updated, rates })
}

/// Parses a Coinbase `/v2/exchange-rates?currency=USD` response. Rates arrive as decimal strings;
/// entries that do not parse to a finite, positive number are dropped.
pub(crate) fn parse_crypto(json: &str) -> Result<BTreeMap<String, f64>, String> {
    let response: CryptoResponse =
        serde_json::from_str(json).map_err(|error| format!("unexpected response: {error}"))?;
    if response.data.currency != BASE {
        return Err(format!("expected base {BASE}, got {}", response.data.currency));
    }
    let rates: BTreeMap<String, f64> = response
        .data
        .rates
        .into_iter()
        .filter_map(|(ticker, rate)| rate.trim().parse::<f64>().ok().map(|rate| (ticker, rate)))
        .filter(|(_, rate)| is_usable_rate(*rate))
        .collect();
    if rates.is_empty() {
        return Err("no usable rates in the response".to_owned());
    }
    Ok(rates)
}

/// The tickers taken from the crypto source: those of the bundled snapshot that are not fiat.
/// Fiat means "listed by the fresh fiat table"; without one, the known crypto list decides.
pub(crate) fn crypto_tickers(fiat: Option<&FiatRates>) -> BTreeSet<String> {
    ExchangeRates::bundled()
        .rates
        .into_keys()
        .filter(|ticker| match fiat {
            Some(fiat) => !fiat.rates.contains_key(ticker),
            None => KNOWN_CRYPTO.contains(&ticker.as_str()),
        })
        .collect()
}

/// Overlays whatever is fresh on top of `fallback` (the last cache or the bundled snapshot):
/// fresh fiat rates replace fiat entries, fresh crypto rates replace the crypto tickers the engine
/// knows. Entries a fresh source no longer lists keep their fallback value.
///
/// `updated` is the fiat publication time when the fiat table is fresh, else the fallback's.
pub(crate) fn merge(
    fallback: ExchangeRates,
    fiat: Option<FiatRates>,
    crypto: Option<BTreeMap<String, f64>>,
) -> ExchangeRates {
    let crypto_tickers = crypto_tickers(fiat.as_ref());
    let mut rates = fallback.rates;
    let mut updated = fallback.updated;
    if let Some(fiat) = fiat {
        updated = fiat.updated.or(updated);
        rates.extend(fiat.rates);
    }
    if let Some(crypto) = crypto {
        rates.extend(
            crypto
                .into_iter()
                .filter(|(ticker, _)| crypto_tickers.contains(ticker)),
        );
    }
    rates.insert(BASE.to_owned(), 1.0);
    ExchangeRates {
        base: BASE.to_owned(),
        updated,
        rates,
    }
}

/// Whether a table is usable as a fallback: USD-based with at least one valid rate.
pub(crate) fn is_valid_table(rates: &ExchangeRates) -> bool {
    rates.base == BASE && !rates.rates.is_empty() && rates.rates.values().all(|rate| is_usable_rate(*rate))
}

fn is_usable_rate(rate: f64) -> bool {
    rate.is_finite() && rate > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIAT_FIXTURE: &str = include_str!("../tests/fixtures/open-er-api-usd.json");
    const CRYPTO_FIXTURE: &str = include_str!("../tests/fixtures/coinbase-usd.json");

    fn table(entries: &[(&str, f64)]) -> ExchangeRates {
        ExchangeRates {
            base: BASE.to_owned(),
            updated: None,
            rates: entries
                .iter()
                .map(|(code, rate)| ((*code).to_owned(), *rate))
                .collect(),
        }
    }

    #[test]
    fn parses_the_recorded_fiat_response() {
        let fiat = parse_fiat(FIAT_FIXTURE).expect("fixture parses");
        assert_eq!(fiat.rates.get("USD"), Some(&1.0));
        assert!(fiat.rates.len() > 150, "only {} fiat rates", fiat.rates.len());
        assert!(fiat.rates.contains_key("EUR") && fiat.rates.contains_key("JPY"));
        assert!(!fiat.rates.contains_key("BTC"));
        assert_eq!(fiat.updated, DateTime::from_timestamp(1_793_145_751, 0));
    }

    #[test]
    fn rejects_fiat_errors_and_foreign_bases() {
        let error = r#"{"result":"error","error-type":"unsupported-code"}"#;
        assert!(parse_fiat(error).expect_err("error result").contains("error"));
        let eur = r#"{"result":"success","base_code":"EUR","rates":{"EUR":1,"USD":1.1}}"#;
        assert!(parse_fiat(eur).expect_err("foreign base").contains("EUR"));
        assert!(parse_fiat("<html>").is_err());
        let empty = r#"{"result":"success","base_code":"USD","rates":{"XXX":-1}}"#;
        assert!(parse_fiat(empty).is_err(), "only unusable rates");
    }

    #[test]
    fn parses_the_recorded_crypto_response() {
        let crypto = parse_crypto(CRYPTO_FIXTURE).expect("fixture parses");
        let btc = crypto["BTC"];
        assert!(btc > 0.0 && btc < 0.001, "BTC per USD is {btc}");
        assert!(crypto.contains_key("ETH") && crypto.contains_key("SOL"));
    }

    #[test]
    fn crypto_parsing_drops_unparsable_rates() {
        let json =
            r#"{"data":{"currency":"USD","rates":{"BTC":"0.00001","BAD":"n/a","NEG":"-2","ZERO":"0"}}}"#;
        let crypto = parse_crypto(json).expect("BTC is usable");
        assert_eq!(
            crypto.into_iter().collect::<Vec<_>>(),
            vec![("BTC".to_owned(), 0.000_01)]
        );
        let eur = r#"{"data":{"currency":"EUR","rates":{"BTC":"0.00001"}}}"#;
        assert!(parse_crypto(eur).is_err());
    }

    #[test]
    fn crypto_tickers_are_the_bundled_tickers_the_fiat_source_does_not_list() {
        let fiat = parse_fiat(FIAT_FIXTURE).expect("fixture parses");
        let from_fiat = crypto_tickers(Some(&fiat));
        let known: BTreeSet<String> = KNOWN_CRYPTO.iter().map(|&ticker| ticker.to_owned()).collect();
        assert_eq!(from_fiat, known, "KNOWN_CRYPTO drifted from the bundled snapshot");
        assert_eq!(crypto_tickers(None), known);
    }

    #[test]
    fn merge_takes_fiat_from_the_fiat_source_and_crypto_from_coinbase() {
        let fiat = parse_fiat(FIAT_FIXTURE).expect("fixture parses");
        let crypto = parse_crypto(CRYPTO_FIXTURE).expect("fixture parses");
        let fiat_eur = fiat.rates["EUR"];
        let coinbase_btc = crypto["BTC"];
        let merged = merge(ExchangeRates::bundled(), Some(fiat.clone()), Some(crypto));

        assert_eq!(merged.base, "USD");
        assert_eq!(merged.updated, fiat.updated);
        assert_eq!(
            merged.rates["EUR"], fiat_eur,
            "fiat wins over Coinbase's fiat rates"
        );
        assert_eq!(merged.rates["BTC"], coinbase_btc);
        assert!(
            !merged.rates.contains_key("1INCH"),
            "tickers outside the snapshot are dropped"
        );
        let bundled_keys: BTreeSet<String> = ExchangeRates::bundled().rates.into_keys().collect();
        let merged_keys: BTreeSet<String> = merged.rates.keys().cloned().collect();
        assert_eq!(merged_keys, bundled_keys);
    }

    #[test]
    fn merge_keeps_fallback_crypto_when_coinbase_fails() {
        let fiat = parse_fiat(FIAT_FIXTURE).expect("fixture parses");
        let fallback = table(&[("USD", 1.0), ("EUR", 0.5), ("BTC", 0.123)]);
        let merged = merge(fallback, Some(fiat), None);
        assert_eq!(merged.rates["BTC"], 0.123);
        assert!((merged.rates["EUR"] - 0.5).abs() > f64::EPSILON, "EUR is fresh");
    }

    #[test]
    fn merge_keeps_fallback_fiat_when_the_fiat_source_fails() {
        let crypto = parse_crypto(CRYPTO_FIXTURE).expect("fixture parses");
        let mut fallback = table(&[("USD", 1.0), ("EUR", 0.5), ("BTC", 0.123)]);
        fallback.updated = DateTime::from_timestamp(1_700_000_000, 0);
        let merged = merge(fallback.clone(), None, Some(crypto.clone()));
        assert_eq!(merged.rates["EUR"], 0.5, "Coinbase's fiat rates are never used");
        assert_eq!(merged.rates["BTC"], crypto["BTC"]);
        assert_eq!(
            merged.rates["ETH"], crypto["ETH"],
            "crypto missing in the fallback is added"
        );
        assert_eq!(merged.updated, fallback.updated);
    }

    #[test]
    fn merge_pins_the_base_to_one() {
        let merged = merge(table(&[("USD", 2.0), ("EUR", 0.9)]), None, None);
        assert_eq!(merged.rates["USD"], 1.0);
    }

    #[test]
    fn validates_tables() {
        assert!(is_valid_table(&ExchangeRates::bundled()));
        assert!(!is_valid_table(&table(&[])));
        assert!(!is_valid_table(&table(&[("USD", 1.0), ("EUR", f64::NAN)])));
        let mut eur = table(&[("EUR", 1.0)]);
        eur.base = "EUR".to_owned();
        assert!(!is_valid_table(&eur));
    }
}
