//! Built-in functions.

use std::f64::consts::PI;

use crate::units::{BUILTINS, BaseDim, Dimension, Form, Unit, UnitRef};
use crate::value::Quantity;
use crate::{AngleUnit, EvalError};

/// Trigonometric results smaller than this are floating-point noise (`sin 180` is 0).
const ROUNDING_NOISE: f64 = 1e-12;

/// A built-in function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Builtin {
    Sqrt,
    Cbrt,
    Root,
    Abs,
    Log,
    Ln,
    Fact,
    Round,
    Ceil,
    Floor,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    FromUnix,
}

impl Builtin {
    /// The function a word names (any case).
    pub(crate) fn from_name(word: &str) -> Option<Self> {
        use Builtin::*;
        Some(match word.to_lowercase().as_str() {
            "sqrt" => Sqrt,
            "cbrt" => Cbrt,
            "root" => Root,
            "abs" => Abs,
            "log" => Log,
            "ln" => Ln,
            "fact" | "factorial" => Fact,
            "round" => Round,
            "ceil" => Ceil,
            "floor" => Floor,
            "sin" => Sin,
            "cos" => Cos,
            "tan" => Tan,
            "arcsin" | "asin" => Asin,
            "arccos" | "acos" => Acos,
            "arctan" | "atan" => Atan,
            "sinh" => Sinh,
            "cosh" => Cosh,
            "tanh" => Tanh,
            "fromunix" => FromUnix,
            _ => return None,
        })
    }

    /// Whether the function takes a leading argument before a parenthesised one (`log 2 (10)`).
    pub(crate) fn takes_leading_argument(self) -> bool {
        matches!(self, Self::Log | Self::Root)
    }
}

fn plain_number(arg: &Quantity) -> f64 {
    if arg.is_percent() {
        arg.value / 100.0
    } else {
        arg.value
    }
}

fn one(args: &[Quantity]) -> Result<&Quantity, EvalError> {
    match args {
        [arg] => Ok(arg),
        _ => Err(EvalError::new(format!(
            "expected one argument, got {}",
            args.len()
        ))),
    }
}

/// Converts an angle quantity to radians; `None` when the quantity is not an angle.
pub(crate) type ToRadians<'a> = &'a dyn Fn(&Quantity) -> Option<f64>;

/// An angle argument in radians: angle quantities convert, plain numbers follow the setting.
fn radians(arg: &Quantity, angle_unit: AngleUnit, to_radians: ToRadians<'_>) -> Result<f64, EvalError> {
    if arg.is_plain() {
        return Ok(match angle_unit {
            AngleUnit::Degrees => arg.value.to_radians(),
            AngleUnit::Radians => arg.value,
        });
    }
    to_radians(arg).ok_or_else(|| EvalError::new("trigonometric functions take angles or plain numbers"))
}

/// An inverse trigonometric result: degrees in degree mode, a plain number in radian mode.
fn angle_result(radians: f64, angle_unit: AngleUnit) -> Quantity {
    match angle_unit {
        AngleUnit::Degrees => {
            let degree = BUILTINS.by_id("°").expect("the degree unit is built in");
            Quantity::new(
                radians * 180.0 / PI,
                Unit::simple(UnitRef::Builtin(degree), Form::Default),
            )
        }
        AngleUnit::Radians => Quantity::plain(radians),
    }
}

fn factorial(n: f64) -> Result<f64, EvalError> {
    if n < 0.0 || n.fract() != 0.0 || n > 170.0 {
        return Err(EvalError::new("factorial needs a whole number from 0 to 170"));
    }
    let mut product = 1.0;
    let mut k = 2.0;
    while k <= n {
        product *= k;
        k += 1.0;
    }
    Ok(product)
}

/// Rounding and `abs` keep money and percentages; everything else becomes a plain number.
fn keep_money(arg: &Quantity, value: f64) -> Quantity {
    if arg.unit.currency().is_some() || arg.is_percent() {
        Quantity::new(value, arg.unit.clone())
    } else {
        Quantity::plain(value)
    }
}

/// Applies a numeric built-in (everything but `fromunix`).
pub(crate) fn apply(
    function: Builtin,
    args: &[Quantity],
    angle_unit: AngleUnit,
    to_radians: ToRadians<'_>,
) -> Result<Quantity, EvalError> {
    use Builtin::*;
    let result = match function {
        Root => match args {
            [n, x] => Quantity::plain(plain_number(x).powf(1.0 / plain_number(n))),
            _ => return Err(EvalError::new("root needs a degree and a value: root 2 (8)")),
        },
        Log => match args {
            [x] => Quantity::plain(plain_number(x).log10()),
            [base, x] => Quantity::plain(plain_number(x).log(plain_number(base))),
            _ => return Err(EvalError::new("log takes a value, or a base and a value")),
        },
        Sin | Cos | Tan => {
            let x = radians(one(args)?, angle_unit, to_radians)?;
            let y = match function {
                Sin => x.sin(),
                Cos => x.cos(),
                _ => x.tan(),
            };
            Quantity::plain(if y.abs() < ROUNDING_NOISE { 0.0 } else { y })
        }
        Asin | Acos | Atan => {
            let x = plain_number(one(args)?);
            angle_result(
                match function {
                    Asin => x.asin(),
                    Acos => x.acos(),
                    _ => x.atan(),
                },
                angle_unit,
            )
        }
        Round | Ceil | Floor | Abs => {
            let arg = one(args)?;
            let value = match function {
                Round => arg.value.round(),
                Ceil => arg.value.ceil(),
                Floor => arg.value.floor(),
                _ => arg.value.abs(),
            };
            keep_money(arg, value)
        }
        Fact => Quantity::plain(factorial(plain_number(one(args)?))?),
        Sqrt | Cbrt | Ln | Sinh | Cosh | Tanh => {
            let x = plain_number(one(args)?);
            Quantity::plain(match function {
                Sqrt => x.sqrt(),
                Cbrt => x.cbrt(),
                Ln => x.ln(),
                Sinh => x.sinh(),
                Cosh => x.cosh(),
                _ => x.tanh(),
            })
        }
        FromUnix => return Err(EvalError::new("fromunix returns a date")),
    };
    if result.value.is_finite() {
        Ok(result)
    } else {
        Err(EvalError::new("the result is not a real number"))
    }
}

/// Whether a dimension is an angle.
pub(crate) fn is_angle(dim: &Dimension) -> bool {
    *dim == Dimension::of(BaseDim::Angle, 1)
}
