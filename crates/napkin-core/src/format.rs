//! Number and currency formatting for results.

use crate::NumberStyle;
use crate::currency;
use crate::num;

/// Values at or above this magnitude are shown in scientific notation.
const SCIENTIFIC_ABOVE: f64 = 1e15;

/// Nonzero values below this magnitude are shown in scientific notation.
const SCIENTIFIC_BELOW: f64 = 1e-6;

/// Decimal places for crypto currencies (trailing zeros trimmed).
const CRYPTO_DECIMALS: u8 = 8;

/// A number format chosen with `in hex`, `in sci`, ...
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Radix {
    Decimal,
    Hex,
    Binary,
    Octal,
    Scientific,
}

impl Radix {
    /// The format named by a conversion target word.
    pub(crate) fn from_word(word: &str) -> Option<Self> {
        Some(match word.to_lowercase().as_str() {
            "hex" | "hexadecimal" => Self::Hex,
            "bin" | "binary" => Self::Binary,
            "oct" | "octal" => Self::Octal,
            "sci" | "scientific" => Self::Scientific,
            "dec" | "decimal" => Self::Decimal,
            _ => return None,
        })
    }
}

fn separators(style: NumberStyle) -> (&'static str, &'static str) {
    match style {
        NumberStyle::English => (",", "."),
        NumberStyle::German => (".", ","),
        NumberStyle::Swiss => ("'", "."),
        NumberStyle::French => (" ", ","),
        NumberStyle::Plain => ("", "."),
    }
}

/// Groups the digits of an unsigned integer string in threes.
fn group(digits: &str, separator: &str) -> String {
    if separator.is_empty() || digits.len() <= 3 {
        return digits.to_owned();
    }
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let first = digits.len() % 3;
    if first > 0 {
        out.push_str(&digits[..first]);
    }
    for (i, chunk) in digits.as_bytes()[first..].chunks(3).enumerate() {
        if i > 0 || first > 0 {
            out.push_str(separator);
        }
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
    }
    out
}

/// Formats `x` with exactly `decimals` places, grouped per `style`, optionally trimming zeros.
fn fixed(x: f64, decimals: u8, style: NumberStyle, trim: Trim) -> String {
    let (group_sep, decimal_sep) = separators(style);
    let text = format!("{:.*}", usize::from(decimals), x.abs());
    let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
    let frac = match trim {
        Trim::Zeros => frac.trim_end_matches('0'),
        Trim::Keep => frac,
    };
    let is_zero = int.bytes().all(|b| b == b'0') && frac.bytes().all(|b| b == b'0');
    let sign = if x.is_sign_negative() && !is_zero { "-" } else { "" };
    let int = group(int, group_sep);
    if frac.is_empty() {
        format!("{sign}{int}")
    } else {
        format!("{sign}{int}{decimal_sep}{frac}")
    }
}

/// Whether to trim trailing zeros after the decimal separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trim {
    Zeros,
    Keep,
}

/// A number with at most `precision` decimals, trailing zeros trimmed. Values that would round
/// to zero keep `precision` significant digits (`0.002`); very large and very small values use
/// scientific notation.
pub(crate) fn number(x: f64, precision: u8, style: NumberStyle) -> String {
    let abs = x.abs();
    if x == 0.0 {
        return "0".to_owned();
    }
    if abs >= SCIENTIFIC_ABOVE || abs < SCIENTIFIC_BELOW {
        return scientific(x, precision, style);
    }
    let smallest = 0.5 * 10f64.powi(-i32::from(precision));
    if abs < smallest {
        let leading_zeros = num::to_i64(-abs.log10().floor())
            .and_then(|z| u8::try_from(z).ok())
            .unwrap_or(0);
        let decimals = leading_zeros.saturating_add(precision.max(1)).saturating_sub(1);
        return fixed(x, decimals, style, Trim::Zeros);
    }
    fixed(x, precision, style, Trim::Zeros)
}

/// Scientific notation: `5.3e3`, `-1.5e-7`.
pub(crate) fn scientific(x: f64, precision: u8, style: NumberStyle) -> String {
    if x == 0.0 {
        return "0".to_owned();
    }
    let mut exponent = x.abs().log10().floor();
    let mut mantissa = x / 10f64.powf(exponent);
    let scale = 10f64.powi(i32::from(precision));
    if (mantissa.abs() * scale).round() / scale >= 10.0 {
        mantissa /= 10.0;
        exponent += 1.0;
    }
    let mantissa = fixed(mantissa, precision, style, Trim::Zeros);
    format!("{mantissa}e{}", num::to_i64(exponent).unwrap_or_default())
}

/// An integer in base 16, 2 or 8 with its `0x`/`0b`/`0o` prefix.
pub(crate) fn radix(x: f64, radix: Radix) -> Option<String> {
    let int = num::to_i64(x.round())?;
    let sign = if int < 0 { "-" } else { "" };
    let abs = int.unsigned_abs();
    Some(match radix {
        Radix::Hex => format!("{sign}0x{abs:X}"),
        Radix::Binary => format!("{sign}0b{abs:b}"),
        Radix::Octal => format!("{sign}0o{abs:o}"),
        Radix::Decimal | Radix::Scientific => return None,
    })
}

/// A currency amount: `$1,234.50`, `€24`, `27.53 CHF`, `0.0005 BTC`.
pub(crate) fn currency(x: f64, code: &str, style: NumberStyle) -> String {
    let amount = if x.abs() >= SCIENTIFIC_ABOVE {
        scientific(x, 2, style)
    } else if currency::is_crypto(code) {
        number(x, CRYPTO_DECIMALS, style)
    } else {
        let cents = (x * 100.0).round();
        let decimals = if cents % 100.0 == 0.0 { 0 } else { 2 };
        fixed(cents / 100.0, decimals, style, Trim::Keep)
    };
    match currency::prefix_sign(code) {
        Some(sign) => match amount.strip_prefix('-') {
            Some(abs) => format!("-{sign}{abs}"),
            None => format!("{sign}{amount}"),
        },
        None => format!("{amount} {code}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_trim_and_group() {
        assert_eq!(number(1234.5678, 2, NumberStyle::English), "1,234.57");
        assert_eq!(number(1234.5678, 2, NumberStyle::German), "1.234,57");
        assert_eq!(number(1234.5, 2, NumberStyle::Swiss), "1'234.5");
        assert_eq!(number(1234.5, 2, NumberStyle::French), "1 234,5");
        assert_eq!(number(1234.5, 2, NumberStyle::Plain), "1234.5");
        assert_eq!(number(3.000_000_1, 2, NumberStyle::English), "3");
        assert_eq!(number(-0.001, 2, NumberStyle::English), "-0.001");
        assert_eq!(number(0.000_123_4, 2, NumberStyle::English), "0.00012");
        assert_eq!(number(1.5e-7, 2, NumberStyle::English), "1.5e-7");
        assert_eq!(number(-2.0, 2, NumberStyle::English), "-2");
        assert_eq!(number(123_456_789.0, 0, NumberStyle::English), "123,456,789");
    }

    #[test]
    fn scientific_and_radix() {
        assert_eq!(scientific(5300.0, 2, NumberStyle::English), "5.3e3");
        assert_eq!(scientific(9.999, 2, NumberStyle::English), "1e1");
        assert_eq!(radix(408_291.0, Radix::Hex).as_deref(), Some("0x63AE3"));
        assert_eq!(radix(-5.0, Radix::Binary).as_deref(), Some("-0b101"));
    }

    #[test]
    fn currencies() {
        assert_eq!(currency(1234.5, "USD", NumberStyle::English), "$1,234.50");
        assert_eq!(currency(24.0, "EUR", NumberStyle::English), "€24");
        assert_eq!(currency(-5.0, "USD", NumberStyle::English), "-$5");
        assert_eq!(currency(27.531, "CHF", NumberStyle::English), "27.53 CHF");
        assert_eq!(currency(0.0005, "BTC", NumberStyle::English), "0.0005 BTC");
        assert_eq!(currency(2.999, "USD", NumberStyle::English), "$3");
    }
}
