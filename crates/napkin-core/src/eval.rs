//! Evaluates expressions over quantities and moments.

use chrono::{DateTime, Days, Months, NaiveTime, TimeDelta, Utc};
use chrono_tz::Tz;

use crate::classify::{DocRef, Func, PercentOp};
use crate::functions::{self, Builtin};
use crate::num;
use crate::parser::{BinOp, Expr, Target};
use crate::units::{BUILTINS, BaseDim, Dimension, Form, Unit, UnitRef};
use crate::value::{Moment, MomentKind, Quantity, Units, Value};
use crate::zones::Zone;
use crate::{CustomFunction, EvalError, ExtValue, Settings};

/// Seconds per day.
const DAY_SECONDS: f64 = 86_400.0;

/// Looks up a variable: `None` when the name is unknown.
pub(crate) type Lookup<'a> = &'a dyn Fn(&str) -> Option<Result<Value, EvalError>>;

/// Everything an expression can refer to.
pub(crate) struct Env<'a> {
    pub(crate) units: Units<'a>,
    pub(crate) settings: &'a Settings,
    pub(crate) now: DateTime<Utc>,
    pub(crate) local: Zone,
    pub(crate) functions: &'a [CustomFunction],
    pub(crate) lookup: Lookup<'a>,
    /// The nearest value above.
    pub(crate) prev: Option<&'a Value>,
    /// The values of the current block, for `sum` and `average`.
    pub(crate) block: &'a [Value],
}

/// Adding or subtracting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sign {
    Plus,
    Minus,
}

impl Sign {
    fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            Self::Plus => a + b,
            Self::Minus => a - b,
        }
    }
}

fn err<T>(message: impl Into<String>) -> Result<T, EvalError> {
    Err(EvalError::new(message))
}

fn quantity(value: Value) -> Result<Quantity, EvalError> {
    match value {
        Value::Quantity(q) => Ok(q),
        Value::Moment(_) => err("dates and times cannot be used here"),
    }
}

/// A percentage as a plain fraction; other quantities unchanged.
fn fraction(q: Quantity) -> Quantity {
    if q.is_percent() {
        Quantity::plain(q.value / 100.0)
    } else {
        q
    }
}

fn percent(value: f64) -> Quantity {
    Quantity::new(value, Unit::simple(UnitRef::Percent, Form::Default))
}

fn builtin_unit(id: &str) -> Unit {
    let index = BUILTINS.by_id(id).expect("time units are built in");
    Unit::simple(UnitRef::Builtin(index), Form::Default)
}

fn time_dimension() -> Dimension {
    Dimension::of(BaseDim::Time, 1)
}

impl Env<'_> {
    /// Evaluates an expression; results are always finite.
    pub(crate) fn eval(&self, expr: &Expr) -> Result<Value, EvalError> {
        let value = self.eval_inner(expr)?;
        match &value {
            Value::Quantity(q) if !q.value.is_finite() => err("the result is not a finite number"),
            _ => Ok(value),
        }
    }

    fn eval_quantity(&self, expr: &Expr) -> Result<Quantity, EvalError> {
        quantity(self.eval(expr)?)
    }

    fn eval_inner(&self, expr: &Expr) -> Result<Value, EvalError> {
        let q = |q: Quantity| Ok(Value::Quantity(q));
        match expr {
            Expr::Num(n) => q(Quantity::plain(*n)),
            Expr::Neg(inner) => {
                let mut value = self.eval_quantity(inner)?;
                value.value = -value.value;
                q(value)
            }
            Expr::WithUnit(inner, unit) => q(self.attach(self.eval_quantity(inner)?, unit)?),
            Expr::BareUnit(unit) => q(Quantity::new(1.0, unit.clone())),
            Expr::Percent(inner) => {
                let value = self.eval_quantity(inner)?;
                if !value.is_plain() {
                    return err("only plain numbers can be percentages");
                }
                q(percent(value.value))
            }
            Expr::Fact(inner) => {
                let arg = self.eval_quantity(inner)?;
                q(functions::apply(
                    Builtin::Fact,
                    &[arg],
                    self.settings.angle_unit,
                    &|_| None,
                )?)
            }
            Expr::Binary(op, a, b) => self.binary(*op, self.eval(a)?, self.eval(b)?),
            Expr::Juxtapose(a, b) => self.juxtapose(self.eval(a)?, self.eval(b)?),
            Expr::Call(func, args) => self.call(*func, args),
            Expr::Var(name) => {
                (self.lookup)(name).unwrap_or_else(|| err(format!("unknown variable `{name}`")))
            }
            Expr::Doc(doc) => self.doc(*doc),
            Expr::Convert(inner, target) => self.convert(self.eval(inner)?, target),
            Expr::PercentOf(op, p, x) => {
                q(self.percent_of(*op, self.eval_quantity(p)?, self.eval_quantity(x)?)?)
            }
            Expr::PercentWhat(op, p, x) => q(percent_what(
                *op,
                &self.eval_quantity(p)?,
                self.eval_quantity(x)?,
            )?),
            Expr::AsPercent(op, x, y) => {
                q(self.as_percent(*op, self.eval_quantity(x)?, self.eval_quantity(y)?)?)
            }
            Expr::Now | Expr::Day(_) | Expr::TimeLit(..) | Expr::DateLit(..) | Expr::ZoneNow(_) => {
                Ok(Value::Moment(self.moment(expr)?))
            }
        }
    }

    fn moment(&self, expr: &Expr) -> Result<Moment, EvalError> {
        let zone_of = |tz: &Option<Tz>| tz.map_or(self.local, Zone::Named);
        let at = |zone: Zone, naive, kind| {
            let instant = zone
                .instant(naive)
                .ok_or_else(|| EvalError::new("that time does not exist"))?;
            Ok(Moment { instant, zone, kind })
        };
        match expr {
            Expr::Now => Ok(Moment {
                instant: self.now,
                zone: self.local,
                kind: MomentKind::Time,
            }),
            Expr::ZoneNow(tz) => Ok(Moment {
                instant: self.now,
                zone: Zone::Named(*tz),
                kind: MomentKind::Time,
            }),
            Expr::Day(offset) => {
                let today = self.local.naive(self.now).date();
                let date = if *offset >= 0 {
                    today.checked_add_days(Days::new(offset.unsigned_abs()))
                } else {
                    today.checked_sub_days(Days::new(offset.unsigned_abs()))
                };
                let date = date.ok_or_else(|| EvalError::new("date out of range"))?;
                at(self.local, date.and_time(NaiveTime::MIN), MomentKind::Date)
            }
            Expr::TimeLit(time, tz) => {
                let zone = zone_of(tz);
                let today = zone.naive(self.now).date();
                at(zone, today.and_time(*time), MomentKind::Time)
            }
            Expr::DateLit(date, tz) => at(zone_of(tz), date.and_time(NaiveTime::MIN), MomentKind::Date),
            _ => err("not a date or time"),
        }
    }

    fn binary(&self, op: BinOp, a: Value, b: Value) -> Result<Value, EvalError> {
        match op {
            BinOp::Add => self.add(a, b, Sign::Plus),
            BinOp::Sub => self.add(a, b, Sign::Minus),
            BinOp::Mul => Ok(Value::Quantity(self.multiply(quantity(a)?, quantity(b)?)?)),
            BinOp::Div => Ok(Value::Quantity(self.divide(quantity(a)?, quantity(b)?)?)),
            BinOp::Pow => Ok(Value::Quantity(power(quantity(a)?, quantity(b)?)?)),
            BinOp::Mod => Ok(Value::Quantity(self.modulo(quantity(a)?, quantity(b)?)?)),
            BinOp::BitAnd | BinOp::BitOr | BinOp::Xor | BinOp::Shl | BinOp::Shr => {
                Ok(Value::Quantity(bitwise(op, &quantity(a)?, &quantity(b)?)?))
            }
        }
    }

    fn add(&self, a: Value, b: Value, sign: Sign) -> Result<Value, EvalError> {
        match (a, b) {
            (Value::Moment(m), Value::Quantity(q)) => Ok(Value::Moment(self.shift(m, &q, sign)?)),
            (Value::Quantity(q), Value::Moment(m)) if sign == Sign::Plus => {
                Ok(Value::Moment(self.shift(m, &q, sign)?))
            }
            (Value::Moment(a), Value::Moment(b)) if sign == Sign::Minus => {
                Ok(Value::Quantity(difference(&a, &b)))
            }
            (Value::Quantity(a), Value::Quantity(b)) => Ok(Value::Quantity(self.add_quantities(a, b, sign)?)),
            _ => err("dates and times can only be shifted by durations or subtracted"),
        }
    }

    /// `+`/`-` on quantities: percent changes, plain numbers adopt units, compatible units
    /// convert to the right operand's unit.
    fn add_quantities(&self, a: Quantity, b: Quantity, sign: Sign) -> Result<Quantity, EvalError> {
        Ok(match (a.is_percent(), b.is_percent()) {
            (false, true) => {
                let change = sign.apply(1.0, b.value / 100.0);
                Quantity::new(a.value * change, a.unit)
            }
            (true, false) => match sign {
                Sign::Plus => Quantity::new(b.value * (1.0 + a.value / 100.0), b.unit),
                Sign::Minus => Quantity::new(a.value / 100.0 - b.value, b.unit),
            },
            (true, true) => percent(sign.apply(a.value, b.value)),
            (false, false) if a.is_plain() => Quantity::new(sign.apply(a.value, b.value), b.unit),
            (false, false) if b.is_plain() => Quantity::new(sign.apply(a.value, b.value), a.unit),
            (false, false) => {
                let left = self.units.convert(&a, &b.unit)?;
                Quantity::new(sign.apply(left, b.value), b.unit)
            }
        })
    }

    fn multiply(&self, a: Quantity, b: Quantity) -> Result<Quantity, EvalError> {
        match (a.is_percent(), b.is_percent()) {
            (true, false) if b.is_plain() => Ok(percent(a.value * b.value)),
            (true, true) => Ok(percent(a.value * b.value / 100.0)),
            _ => self.combine(fraction(a), fraction(b)),
        }
    }

    fn divide(&self, a: Quantity, b: Quantity) -> Result<Quantity, EvalError> {
        if b.value == 0.0 {
            return err("division by zero");
        }
        if a.is_percent() && b.is_plain() {
            return Ok(percent(a.value / b.value));
        }
        let b = fraction(b);
        let inverse = Quantity::new(1.0 / b.value, b.unit.pow(-1));
        self.combine(fraction(a), inverse)
    }

    /// Multiplies two quantities, merging equal units and converting units of the same
    /// dimension to the left operand's; a dimensionless compound becomes a plain number.
    fn combine(&self, a: Quantity, b: Quantity) -> Result<Quantity, EvalError> {
        let mut value = a.value * b.value;
        let mut factors = a.unit.factors;
        for factor in b.unit.factors {
            if let Some(existing) = factors.iter_mut().find(|f| f.unit == factor.unit) {
                existing.power = existing.power.saturating_add(factor.power);
                continue;
            }
            let dim = self.units.ref_dimension(&factor.unit);
            let same_dim = factors
                .iter_mut()
                .find(|f| !dim.is_none() && self.units.ref_dimension(&f.unit) == dim);
            if let Some(existing) = same_dim {
                let ratio = self.units.ref_factor(&factor.unit)? / self.units.ref_factor(&existing.unit)?;
                value *= ratio.powi(i32::from(factor.power));
                existing.power = existing.power.saturating_add(factor.power);
            } else {
                factors.push(factor);
            }
        }
        factors.retain(|f| f.power != 0);
        let unit = Unit { factors };
        if unit.factors.len() >= 2 && self.units.dimension(&unit).is_none() {
            return Ok(Quantity::plain(value * self.units.factor(&unit)?));
        }
        Ok(Quantity::new(value, unit))
    }

    fn modulo(&self, a: Quantity, b: Quantity) -> Result<Quantity, EvalError> {
        if b.value == 0.0 {
            return err("division by zero");
        }
        if a.is_plain() {
            return Ok(Quantity::new(a.value % b.value, b.unit));
        }
        let divisor = if b.is_plain() {
            b.value
        } else {
            self.units.convert(&b, &a.unit)?
        };
        Ok(Quantity::new(a.value % divisor, a.unit))
    }

    /// Next to each other: same-dimension quantities add up (`1 meter 20 cm`), a date and a
    /// time combine, everything else multiplies.
    fn juxtapose(&self, a: Value, b: Value) -> Result<Value, EvalError> {
        match (a, b) {
            (Value::Moment(date), Value::Moment(time)) | (Value::Moment(time), Value::Moment(date))
                if date.kind == MomentKind::Date && time.kind == MomentKind::Time =>
            {
                let naive = date.naive().date().and_time(time.naive().time());
                let instant = date
                    .zone
                    .instant(naive)
                    .ok_or_else(|| EvalError::new("that time does not exist"))?;
                Ok(Value::Moment(Moment {
                    instant,
                    zone: date.zone,
                    kind: MomentKind::DateTime,
                }))
            }
            (Value::Quantity(a), Value::Quantity(b)) => {
                let measured = |q: &Quantity| !q.is_plain() && !q.is_percent();
                if measured(&a)
                    && measured(&b)
                    && self.units.dimension(&a.unit) == self.units.dimension(&b.unit)
                {
                    Ok(Value::Quantity(self.add_quantities(a, b, Sign::Plus)?))
                } else {
                    Ok(Value::Quantity(self.multiply(a, b)?))
                }
            }
            _ => err("dates and times cannot be multiplied"),
        }
    }

    /// Puts a unit on a value: plain numbers take it, a currency replaces a currency
    /// (`$30 CAD`), anything else multiplies (`5 kW h`).
    fn attach(&self, q: Quantity, unit: &Unit) -> Result<Quantity, EvalError> {
        if q.is_plain() {
            return Ok(Quantity {
                unit: unit.clone(),
                ..q
            });
        }
        if q.unit.currency().is_some() && unit.currency().is_some() {
            return Ok(Quantity::new(q.value, unit.clone()));
        }
        if q.is_percent() {
            return err("a percentage cannot take a unit");
        }
        self.combine(q, Quantity::new(1.0, unit.clone()))
    }

    fn convert(&self, value: Value, target: &Target) -> Result<Value, EvalError> {
        match (value, target) {
            (Value::Moment(m), Target::Zone(tz)) => Ok(Value::Moment(Moment {
                zone: Zone::Named(*tz),
                ..m
            })),
            (Value::Quantity(q), Target::Radix(radix)) => Ok(Value::Quantity(Quantity {
                radix: Some(*radix),
                ..q
            })),
            (Value::Quantity(q), Target::Unit(unit)) => Ok(Value::Quantity(self.convert_quantity(q, unit)?)),
            (Value::Moment(_), _) => err("dates and times can only be converted to a time zone"),
            (Value::Quantity(_), Target::Zone(_)) => err("only times can be converted to a time zone"),
        }
    }

    fn convert_quantity(&self, q: Quantity, unit: &Unit) -> Result<Quantity, EvalError> {
        if unit.is_percent() {
            return match (q.is_percent(), q.is_plain()) {
                (true, _) => Ok(q),
                (false, true) => Ok(percent(q.value * 100.0)),
                (false, false) => err("only plain numbers convert to percent"),
            };
        }
        if q.is_plain() {
            return Ok(Quantity {
                unit: unit.clone(),
                ..q
            });
        }
        if q.is_percent() {
            return err("percentages cannot be converted to units");
        }
        Ok(Quantity::new(self.units.convert(&q, unit)?, unit.clone()))
    }

    fn percent_of(&self, op: PercentOp, p: Quantity, x: Quantity) -> Result<Quantity, EvalError> {
        if !p.is_percent() {
            return match op {
                PercentOp::Of => self.multiply(p, x),
                PercentOp::On | PercentOp::Off => err("`on` and `off` need a percentage"),
            };
        }
        let share = p.value / 100.0;
        let factor = match op {
            PercentOp::Of => share,
            PercentOp::On => 1.0 + share,
            PercentOp::Off => 1.0 - share,
        };
        Ok(Quantity::new(x.value * factor, x.unit))
    }
}

fn percent_what(op: PercentOp, p: &Quantity, x: Quantity) -> Result<Quantity, EvalError> {
    {
        if !p.is_percent() {
            return err("`of what is` needs a percentage");
        }
        let share = p.value / 100.0;
        let divisor = match op {
            PercentOp::Of => share,
            PercentOp::On => 1.0 + share,
            PercentOp::Off => 1.0 - share,
        };
        if divisor == 0.0 {
            return err("division by zero");
        }
        Ok(Quantity::new(x.value / divisor, x.unit))
    }
}

impl Env<'_> {
    fn as_percent(&self, op: PercentOp, x: Quantity, y: Quantity) -> Result<Quantity, EvalError> {
        let ratio = self.divide(x, y)?;
        if !ratio.is_plain() {
            return err("`as a %` needs two values of the same kind");
        }
        let r = ratio.value;
        Ok(percent(match op {
            PercentOp::Of => r * 100.0,
            PercentOp::On => (r - 1.0) * 100.0,
            PercentOp::Off => (1.0 - r) * 100.0,
        }))
    }

    /// Shifts a moment by a duration; whole days, weeks, months and years follow the calendar.
    fn shift(&self, m: Moment, q: &Quantity, sign: Sign) -> Result<Moment, EvalError> {
        if self.units.dimension(&q.unit) != time_dimension() {
            return err("dates and times can only be shifted by durations");
        }
        let amount = sign.apply(0.0, q.value);
        if let Some(UnitRef::Builtin(index)) = q.unit.as_simple()
            && amount.fract() == 0.0
            && let Some(moment) = shift_calendar(m, &BUILTINS.def(*index).id, amount)
        {
            return Ok(moment);
        }
        let seconds = sign.apply(0.0, self.units.convert(q, &builtin_unit("s"))?);
        let millis =
            num::to_i64((seconds * 1000.0).round()).ok_or_else(|| EvalError::new("duration out of range"))?;
        let delta =
            TimeDelta::try_milliseconds(millis).ok_or_else(|| EvalError::new("duration out of range"))?;
        let instant = m
            .instant
            .checked_add_signed(delta)
            .ok_or_else(|| EvalError::new("date out of range"))?;
        let kind = if m.kind == MomentKind::Date && seconds % DAY_SECONDS != 0.0 {
            MomentKind::DateTime
        } else {
            m.kind
        };
        Ok(Moment { instant, kind, ..m })
    }
}

fn shift_calendar(m: Moment, unit_id: &str, amount: f64) -> Option<Moment> {
    {
        let count = num::to_i64(amount)?;
        let naive = m.naive();
        let magnitude = u32::try_from(count.unsigned_abs()).ok()?;
        let shifted = match unit_id {
            "month" | "year" => {
                let months = Months::new(if unit_id == "year" {
                    magnitude.checked_mul(12)?
                } else {
                    magnitude
                });
                if count >= 0 {
                    naive.checked_add_months(months)
                } else {
                    naive.checked_sub_months(months)
                }
            }
            "day" | "week" => {
                let days = Days::new(u64::from(magnitude) * if unit_id == "week" { 7 } else { 1 });
                if count >= 0 {
                    naive.checked_add_days(days)
                } else {
                    naive.checked_sub_days(days)
                }
            }
            _ => None,
        }?;
        Some(Moment {
            instant: m.zone.instant(shifted)?,
            ..m
        })
    }
}

/// The wall-clock difference between two moments, in days, hours or minutes.
fn difference(a: &Moment, b: &Moment) -> Quantity {
    {
        let seconds = num::from_i64((a.naive() - b.naive()).num_milliseconds()) / 1000.0;
        let (id, size) = if a.kind == MomentKind::Date && b.kind == MomentKind::Date
            || seconds.abs() >= 2.0 * DAY_SECONDS
        {
            ("day", DAY_SECONDS)
        } else if seconds.abs() >= 3600.0 {
            ("h", 3600.0)
        } else {
            ("min", 60.0)
        };
        Quantity::new(seconds / size, builtin_unit(id))
    }
}

impl Env<'_> {
    fn doc(&self, doc: DocRef) -> Result<Value, EvalError> {
        match doc {
            DocRef::Prev => self
                .prev
                .cloned()
                .ok_or_else(|| EvalError::new("no previous result")),
            DocRef::Sum => Ok(Value::Quantity(self.sum()?.0)),
            DocRef::Average => {
                let (total, count) = self.sum()?;
                let count = num::from_usize(count.max(1));
                Ok(Value::Quantity(Quantity::new(total.value / count, total.unit)))
            }
        }
    }

    /// The sum of the block's quantities in the first one's unit, and how many there were.
    fn sum(&self) -> Result<(Quantity, usize), EvalError> {
        let mut values = self.block.iter().filter_map(|v| match v {
            Value::Quantity(q) => Some(q),
            Value::Moment(_) => None,
        });
        let Some(first) = values.next() else {
            return Ok((Quantity::plain(0.0), 0));
        };
        let mut total = Quantity::new(first.value, first.unit.clone());
        let mut count = 1;
        for value in values {
            total = self.add_quantities(total, Quantity::new(value.value, value.unit.clone()), Sign::Plus)?;
            count += 1;
        }
        if !first.is_plain() && !total.is_plain() && !total.is_percent() && total.unit != first.unit {
            total = Quantity::new(self.units.convert(&total, &first.unit)?, first.unit.clone());
        }
        Ok((total, count))
    }

    fn call(&self, func: Func, args: &[Expr]) -> Result<Value, EvalError> {
        let values = args
            .iter()
            .map(|arg| self.eval(arg))
            .collect::<Result<Vec<_>, _>>()?;
        match func {
            Func::Builtin(Builtin::FromUnix) => {
                let [Value::Quantity(seconds)] = values.as_slice() else {
                    return err("fromunix takes one timestamp");
                };
                let millis = num::to_i64((seconds.value * 1000.0).round())
                    .ok_or_else(|| EvalError::new("timestamp out of range"))?;
                let instant = DateTime::from_timestamp_millis(millis)
                    .ok_or_else(|| EvalError::new("timestamp out of range"))?;
                Ok(Value::Moment(Moment {
                    instant,
                    zone: self.local,
                    kind: MomentKind::DateTime,
                }))
            }
            Func::Builtin(builtin) => {
                let quantities = values.into_iter().map(quantity).collect::<Result<Vec<_>, _>>()?;
                let to_radians = |q: &Quantity| {
                    let angle = functions::is_angle(&self.units.dimension(&q.unit));
                    angle
                        .then(|| self.units.factor(&q.unit).ok().map(|f| q.value * f))
                        .flatten()
                };
                let result = functions::apply(builtin, &quantities, self.settings.angle_unit, &to_radians)?;
                Ok(Value::Quantity(result))
            }
            Func::Custom(index) => self.call_custom(index, &values),
        }
    }

    fn call_custom(&self, index: usize, values: &[Value]) -> Result<Value, EvalError> {
        let function = self
            .functions
            .get(index)
            .ok_or_else(|| EvalError::new("unknown function"))?;
        let args: Vec<ExtValue> = values
            .iter()
            .map(|value| match value {
                Value::Quantity(q) => ExtValue {
                    double: q.value,
                    unit_id: self.units.id(&q.unit),
                },
                Value::Moment(m) => ExtValue {
                    double: num::from_i64(m.instant.timestamp()),
                    unit_id: None,
                },
            })
            .collect();
        let result = (function.call)(&args).map_err(EvalError::new)?;
        Ok(Value::Quantity(self.quantity_from_ext(&result)?))
    }

    /// A quantity from an extension value.
    pub(crate) fn quantity_from_ext(&self, value: &ExtValue) -> Result<Quantity, EvalError> {
        match &value.unit_id {
            None => Ok(Quantity::plain(value.double)),
            Some(id) => {
                let unit = self
                    .units
                    .unit_for_id(id)
                    .ok_or_else(|| EvalError::new(format!("unknown unit id `{id}`")))?;
                Ok(Quantity::new(value.double, unit))
            }
        }
    }
}

fn power(base: Quantity, exponent: Quantity) -> Result<Quantity, EvalError> {
    let exponent = fraction(exponent);
    if !exponent.is_plain() {
        return err("exponents must be plain numbers");
    }
    let base = fraction(base);
    if base.is_plain() {
        return Ok(Quantity::plain(base.value.powf(exponent.value)));
    }
    let Some(power) = num::to_power(exponent.value) else {
        return err("units can only be raised to whole powers");
    };
    Ok(Quantity::new(
        base.value.powi(i32::from(power)),
        base.unit.pow(power),
    ))
}

fn integer(q: &Quantity) -> Result<i64, EvalError> {
    if !q.is_plain() || q.value.fract() != 0.0 {
        return err("bitwise operations need whole plain numbers");
    }
    num::to_i64(q.value).ok_or_else(|| EvalError::new("number too large for bitwise operations"))
}

fn bitwise(op: BinOp, a: &Quantity, b: &Quantity) -> Result<Quantity, EvalError> {
    let (a, b) = (integer(a)?, integer(b)?);
    let shift = || {
        u32::try_from(b)
            .ok()
            .filter(|s| *s < 64)
            .ok_or_else(|| EvalError::new("shift out of range"))
    };
    let result = match op {
        BinOp::BitAnd => a & b,
        BinOp::BitOr => a | b,
        BinOp::Xor => a ^ b,
        BinOp::Shl => a
            .checked_shl(shift()?)
            .ok_or_else(|| EvalError::new("shift out of range"))?,
        BinOp::Shr => a
            .checked_shr(shift()?)
            .ok_or_else(|| EvalError::new("shift out of range"))?,
        _ => return err("not a bitwise operator"),
    };
    Ok(Quantity::plain(num::from_i64(result)))
}
