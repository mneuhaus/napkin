//! Checked conversions between `f64` and integers.

/// The largest magnitude below which every `f64` integer converts to `i64` exactly.
const I64_LIMIT: f64 = 9_223_372_036_854_775_808.0;

/// `x` truncated to an `i64`, or `None` when it is not finite or out of range.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "range checked above"
)]
pub(crate) fn to_i64(x: f64) -> Option<i64> {
    (x.is_finite() && x.abs() < I64_LIMIT).then(|| x.trunc() as i64)
}

/// `x` as the nearest `f64`.
#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    reason = "rounding to f64 is intended"
)]
pub(crate) fn from_i64(x: i64) -> f64 {
    x as f64
}

/// A count as the nearest `f64`.
pub(crate) fn from_usize(n: usize) -> f64 {
    i64::try_from(n).map_or(f64::INFINITY, from_i64)
}

/// `x` as an exact small integer power, when it is one.
pub(crate) fn to_power(x: f64) -> Option<i8> {
    if x.fract() != 0.0 {
        return None;
    }
    to_i64(x).and_then(|int| i8::try_from(int).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_check_range() {
        assert_eq!(to_i64(3.9), Some(3));
        assert_eq!(to_i64(f64::NAN), None);
        assert_eq!(to_i64(1e30), None);
        assert_eq!(to_power(2.0), Some(2));
        assert_eq!(to_power(2.5), None);
        assert!((from_usize(3) - 3.0).abs() < f64::EPSILON);
    }
}
