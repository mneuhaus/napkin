//! Currency codes, names, signs and crypto tickers, with Numi's case rules.

/// Crypto tickers the engine knows; they are only recognised in uppercase.
pub(crate) const CRYPTO: &[&str] = &[
    "BTC", "ETH", "USDT", "USDC", "BNB", "SOL", "XRP", "DOGE", "ADA", "TRX", "AVAX", "LINK", "DOT", "LTC",
    "BCH", "XLM", "XMR", "ETC", "ATOM", "UNI", "SHIB", "NEAR", "APT", "ARB", "OP", "FIL", "ALGO", "XTZ",
    "AAVE", "MKR", "SUI", "PEPE", "DAI", "HBAR", "ICP", "VET", "EOS", "ZEC", "DASH", "TON",
];

/// Common currencies whose codes are also recognised in lowercase or mixed case.
const CASE_INSENSITIVE: &[&str] = &[
    "USD", "EUR", "GBP", "JPY", "CHF", "CAD", "AUD", "NZD", "CNY", "HKD", "SGD", "SEK", "NOK", "DKK", "PLN",
    "CZK", "HUF", "RUB", "UAH", "INR", "KRW", "BRL", "MXN", "ZAR", "TRY", "ILS", "THB", "AED", "SAR",
];

/// Active ISO 4217 codes (plus a few widely quoted ones such as `CNH`).
const ISO_CODES: &[&str] = &[
    "AED", "AFN", "ALL", "AMD", "ANG", "AOA", "ARS", "AUD", "AWG", "AZN", "BAM", "BBD", "BDT", "BGN", "BHD",
    "BIF", "BMD", "BND", "BOB", "BRL", "BSD", "BTN", "BWP", "BYN", "BZD", "CAD", "CDF", "CHF", "CLF", "CLP",
    "CNH", "CNY", "COP", "CRC", "CUC", "CUP", "CVE", "CZK", "DJF", "DKK", "DOP", "DZD", "EGP", "ERN", "ETB",
    "EUR", "FJD", "FKP", "FOK", "GBP", "GEL", "GGP", "GHS", "GIP", "GMD", "GNF", "GTQ", "GYD", "HKD", "HNL",
    "HRK", "HTG", "HUF", "IDR", "ILS", "IMP", "INR", "IQD", "IRR", "ISK", "JEP", "JMD", "JOD", "JPY", "KES",
    "KGS", "KHR", "KID", "KMF", "KPW", "KRW", "KWD", "KYD", "KZT", "LAK", "LBP", "LKR", "LRD", "LSL", "LYD",
    "MAD", "MDL", "MGA", "MKD", "MMK", "MNT", "MOP", "MRU", "MUR", "MVR", "MWK", "MXN", "MYR", "MZN", "NAD",
    "NGN", "NIO", "NOK", "NPR", "NZD", "OMR", "PAB", "PEN", "PGK", "PHP", "PKR", "PLN", "PYG", "QAR", "RON",
    "RSD", "RUB", "RWF", "SAR", "SBD", "SCR", "SDG", "SEK", "SGD", "SHP", "SLE", "SLL", "SOS", "SRD", "SSP",
    "STN", "SYP", "SZL", "THB", "TJS", "TMT", "TND", "TOP", "TRY", "TTD", "TVD", "TWD", "TZS", "UAH", "UGX",
    "USD", "UYU", "UZS", "VES", "VND", "VUV", "WST", "XAF", "XCD", "XCG", "XDR", "XOF", "XPF", "YER", "ZAR",
    "ZMW", "ZWL",
];

/// Currency names (lowercase) and the code they stand for.
const NAMES: &[(&str, &str)] = &[
    ("dollar", "USD"),
    ("dollars", "USD"),
    ("buck", "USD"),
    ("bucks", "USD"),
    ("euro", "EUR"),
    ("euros", "EUR"),
    ("pound", "GBP"),
    ("pounds", "GBP"),
    ("pound sterling", "GBP"),
    ("pounds sterling", "GBP"),
    ("quid", "GBP"),
    ("yen", "JPY"),
    ("rouble", "RUB"),
    ("roubles", "RUB"),
    ("ruble", "RUB"),
    ("rubles", "RUB"),
    ("franc", "CHF"),
    ("francs", "CHF"),
    ("swiss franc", "CHF"),
    ("swiss francs", "CHF"),
    ("rupee", "INR"),
    ("rupees", "INR"),
    ("yuan", "CNY"),
    ("renminbi", "CNY"),
    ("won", "KRW"),
    ("lira", "TRY"),
    ("liras", "TRY"),
    ("hryvnia", "UAH"),
    ("hryvnias", "UAH"),
    ("shekel", "ILS"),
    ("shekels", "ILS"),
    ("baht", "THB"),
    ("zloty", "PLN"),
    ("zlotys", "PLN"),
    ("forint", "HUF"),
    ("forints", "HUF"),
    ("koruna", "CZK"),
    ("korunas", "CZK"),
    ("krona", "SEK"),
    ("kronor", "SEK"),
    ("krone", "NOK"),
    ("kroner", "NOK"),
    ("reais", "BRL"),
    ("rand", "ZAR"),
    ("peso", "MXN"),
    ("pesos", "MXN"),
    ("dirham", "AED"),
    ("dirhams", "AED"),
    ("riyal", "SAR"),
    ("riyals", "SAR"),
    ("canadian dollar", "CAD"),
    ("canadian dollars", "CAD"),
    ("australian dollar", "AUD"),
    ("australian dollars", "AUD"),
    ("bitcoin", "BTC"),
    ("bitcoins", "BTC"),
    ("ether", "ETH"),
    ("ethereum", "ETH"),
];

/// Currency signs and their codes.
const SIGNS: &[(char, &str)] = &[
    ('$', "USD"),
    ('€', "EUR"),
    ('£', "GBP"),
    ('¥', "JPY"),
    ('₽', "RUB"),
    ('₹', "INR"),
    ('₩', "KRW"),
    ('₺', "TRY"),
    ('₴', "UAH"),
    ('₪', "ILS"),
    ('฿', "THB"),
    ('₿', "BTC"),
];

/// Currencies written with a prefix sign in results (`$5`, `€5`); all others get a code suffix.
const PREFIX_DISPLAY: &[(&str, &str)] = &[("USD", "$"), ("EUR", "€"), ("GBP", "£"), ("JPY", "¥")];

/// The currency code for a sign character.
pub(crate) fn sign(c: char) -> Option<&'static str> {
    SIGNS.iter().find(|(s, _)| *s == c).map(|(_, code)| *code)
}

/// Whether `code` is a crypto ticker.
pub(crate) fn is_crypto(code: &str) -> bool {
    CRYPTO.contains(&code)
}

/// The code a word stands for, following the case rules: allowlisted codes in any case, every
/// other ISO code and every crypto ticker only in uppercase.
pub(crate) fn code(word: &str) -> Option<&'static str> {
    let upper = word.to_ascii_uppercase();
    if word == upper {
        ISO_CODES.iter().chain(CRYPTO).find(|c| **c == word).copied()
    } else {
        CASE_INSENSITIVE.iter().find(|c| **c == upper).copied()
    }
}

/// The code a currency name (one or more words, any case) stands for.
pub(crate) fn name(words: &str) -> Option<&'static str> {
    let lower = words.to_lowercase();
    NAMES.iter().find(|(n, _)| *n == lower).map(|(_, code)| *code)
}

/// The display sign for currencies shown as a prefix.
pub(crate) fn prefix_sign(code: &str) -> Option<&'static str> {
    PREFIX_DISPLAY
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, sign)| *sign)
}

/// The canonical code for a known code in any case (for extension unit ids).
pub(crate) fn canonical(code_text: &str) -> Option<&'static str> {
    let upper = code_text.to_ascii_uppercase();
    ISO_CODES.iter().chain(CRYPTO).find(|c| **c == upper).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_rules_keep_words_as_words() {
        assert_eq!(code("eur"), Some("EUR"));
        assert_eq!(code("Usd"), Some("USD"));
        assert_eq!(code("CUP"), Some("CUP"));
        assert_eq!(code("cup"), None);
        assert_eq!(code("all"), None);
        assert_eq!(code("ALL"), Some("ALL"));
        assert_eq!(code("btc"), None);
        assert_eq!(code("BTC"), Some("BTC"));
        assert_eq!(code("top"), None);
    }

    #[test]
    fn names_and_signs() {
        assert_eq!(name("Euros"), Some("EUR"));
        assert_eq!(name("pound sterling"), Some("GBP"));
        assert_eq!(sign('₿'), Some("BTC"));
        assert_eq!(prefix_sign("CHF"), None);
    }
}
