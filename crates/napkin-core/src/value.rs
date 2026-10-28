//! Runtime values (quantities and moments), unit conversion and result display.

use chrono::{DateTime, NaiveDateTime, Utc};

use crate::custom::{self, Base, Registry};
use crate::format::{self, Radix};
use crate::units::{BUILTINS, BaseDim, Dimension, Factor, Form, Scale, Style, Unit, UnitRef, superscript};
use crate::vocab::{self, Vocab};
use crate::zones::Zone;
use crate::{Answer, EvalError, ExchangeRates, InputLanguage, NumberStyle, Settings, TimeFormat, currency};

/// Metres per inch, for CSS units.
const INCH: f64 = 0.0254;

/// A number with a unit (possibly none, possibly percent or a currency).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Quantity {
    pub(crate) value: f64,
    pub(crate) unit: Unit,
    /// A display format chosen with `in hex`, `in sci`, ...
    pub(crate) radix: Option<Radix>,
}

impl Quantity {
    pub(crate) fn plain(value: f64) -> Self {
        Self {
            value,
            unit: Unit::default(),
            radix: None,
        }
    }

    pub(crate) fn new(value: f64, unit: Unit) -> Self {
        Self {
            value,
            unit,
            radix: None,
        }
    }

    pub(crate) fn is_plain(&self) -> bool {
        self.unit.is_empty()
    }

    pub(crate) fn is_percent(&self) -> bool {
        self.unit.is_percent()
    }
}

/// What part of a moment is meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MomentKind {
    /// A time of day (`13:00`).
    Time,
    /// A calendar date (`Wed, 28 Oct 2026`).
    Date,
    /// Both (`Wed, 28 Oct 2026 13:00`).
    DateTime,
}

/// A point in time shown in a zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Moment {
    pub(crate) instant: DateTime<Utc>,
    pub(crate) zone: Zone,
    pub(crate) kind: MomentKind,
}

impl Moment {
    pub(crate) fn naive(&self) -> NaiveDateTime {
        self.zone.naive(self.instant)
    }
}

/// A computed value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Quantity(Quantity),
    Moment(Moment),
}

/// Everything unit conversion depends on: extension units, rates and the CSS settings.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Units<'a> {
    pub(crate) custom: &'a Registry,
    pub(crate) rates: &'a ExchangeRates,
    pub(crate) ppi: f64,
    pub(crate) em_px: f64,
}

impl Units<'_> {
    pub(crate) fn ref_dimension(&self, unit: &UnitRef) -> Dimension {
        match unit {
            UnitRef::Builtin(index) => BUILTINS.def(*index).dim.clone(),
            UnitRef::Custom(index) => match &self.custom.units[*index].base {
                Base::Physical { dim, .. } => dim.clone(),
                Base::Currency { .. } => Dimension::of(BaseDim::Currency, 1),
            },
            UnitRef::Currency(_) => Dimension::of(BaseDim::Currency, 1),
            UnitRef::Percent => Dimension::default(),
        }
    }

    /// How many base units one `code` is worth.
    fn currency_factor(&self, code: &str) -> Result<f64, EvalError> {
        if code == self.rates.base {
            return Ok(1.0);
        }
        match self.rates.rates.get(code) {
            Some(rate) if rate.is_finite() && *rate > 0.0 => Ok(1.0 / rate),
            _ => Err(EvalError::new(format!("no exchange rate for {code}"))),
        }
    }

    /// How many SI base units (or base-currency units) one `unit` is.
    pub(crate) fn ref_factor(&self, unit: &UnitRef) -> Result<f64, EvalError> {
        match unit {
            UnitRef::Builtin(index) => Ok(match BUILTINS.def(*index).scale {
                Scale::Factor(factor) | Scale::Temperature { factor, .. } => factor,
                Scale::Pixel => INCH / self.ppi,
                Scale::Em => self.em_px * INCH / self.ppi,
            }),
            UnitRef::Custom(index) => match &self.custom.units[*index].base {
                Base::Physical { factor, .. } => Ok(*factor),
                Base::Currency { code, ratio } => Ok(ratio * self.currency_factor(code)?),
            },
            UnitRef::Currency(code) => self.currency_factor(code),
            UnitRef::Percent => Ok(0.01),
        }
    }

    pub(crate) fn dimension(&self, unit: &Unit) -> Dimension {
        unit.factors.iter().fold(Dimension::default(), |acc, f| {
            acc.mul_pow(&self.ref_dimension(&f.unit), f.power)
        })
    }

    pub(crate) fn factor(&self, unit: &Unit) -> Result<f64, EvalError> {
        unit.factors.iter().try_fold(1.0, |acc, f| {
            Ok(acc * self.ref_factor(&f.unit)?.powi(i32::from(f.power)))
        })
    }

    /// The value of `quantity` expressed in `to`, which must have the same dimension.
    pub(crate) fn convert(&self, quantity: &Quantity, to: &Unit) -> Result<f64, EvalError> {
        if quantity.unit == *to {
            return Ok(quantity.value);
        }
        if self.dimension(&quantity.unit) != self.dimension(to) {
            return Err(EvalError::new(format!(
                "cannot convert {} to {}",
                self.describe(&quantity.unit),
                self.describe(to)
            )));
        }
        if let (Some(from_offset), Some(to_offset)) = (temperature(&quantity.unit), temperature(to)) {
            let kelvin = (quantity.value + from_offset.1) * from_offset.0;
            let value = kelvin / to_offset.0 - to_offset.1;
            // `32 °F in °C` is 0, not 5.7e-14.
            let noise = (kelvin / to_offset.0).abs() * 1e-12;
            return Ok(if value.abs() <= noise { 0.0 } else { value });
        }
        Ok(quantity.value * self.factor(&quantity.unit)? / self.factor(to)?)
    }

    /// A short name of a unit for error messages.
    pub(crate) fn describe(&self, unit: &Unit) -> String {
        if unit.is_empty() {
            "a plain number".to_owned()
        } else {
            self.symbols(unit)
        }
    }

    /// The id reported in [`Answer::unit`].
    pub(crate) fn id(&self, unit: &Unit) -> Option<String> {
        if unit.is_empty() {
            return None;
        }
        let one = |f: &Factor| {
            let base = match &f.unit {
                UnitRef::Builtin(index) => BUILTINS.def(*index).id.clone(),
                UnitRef::Custom(index) => self.custom.units[*index].id.clone(),
                UnitRef::Currency(code) => code.clone(),
                UnitRef::Percent => "percent".to_owned(),
            };
            match f.power.unsigned_abs() {
                1 => base,
                power => format!("{base}{power}"),
            }
        };
        let numerator: Vec<String> = unit.factors.iter().filter(|f| f.power > 0).map(one).collect();
        let denominator: Vec<String> = unit.factors.iter().filter(|f| f.power < 0).map(one).collect();
        Some(match (numerator.is_empty(), denominator.is_empty()) {
            (_, true) => numerator.join("*"),
            (true, false) => format!("1/{}", denominator.join("*")),
            (false, false) => format!("{}/{}", numerator.join("*"), denominator.join("*")),
        })
    }

    fn ref_symbol(&self, unit: &UnitRef) -> String {
        match unit {
            UnitRef::Builtin(index) => BUILTINS.def(*index).symbol.clone(),
            UnitRef::Custom(index) => self.custom.units[*index].format.clone(),
            UnitRef::Currency(code) => code.clone(),
            UnitRef::Percent => "%".to_owned(),
        }
    }

    /// Compound symbol notation: `km/h`, `m²`, `kg·m/s²`.
    pub(crate) fn symbols(&self, unit: &Unit) -> String {
        let part = |factors: Vec<&Factor>, sign: i8| {
            factors
                .iter()
                .map(|f| format!("{}{}", self.ref_symbol(&f.unit), superscript(f.power * sign)))
                .collect::<Vec<_>>()
                .join("·")
        };
        let numerator: Vec<&Factor> = unit.factors.iter().filter(|f| f.power > 0).collect();
        let denominator: Vec<&Factor> = unit.factors.iter().filter(|f| f.power < 0).collect();
        match (numerator.is_empty(), denominator.is_empty()) {
            (_, true) => part(numerator, 1),
            (true, false) => part(denominator, 1),
            (false, false) => format!("{}/{}", part(numerator, 1), part(denominator, -1)),
        }
    }

    /// The display text of a value.
    pub(crate) fn display(&self, value: &Value, settings: &Settings, now: DateTime<Utc>) -> String {
        match value {
            Value::Quantity(quantity) => self.display_quantity(quantity, settings),
            Value::Moment(moment) => display_moment(moment, settings, now),
        }
    }

    fn display_quantity(&self, quantity: &Quantity, settings: &Settings) -> String {
        let style = settings.number_style;
        let number = match quantity.radix {
            Some(radix @ (Radix::Hex | Radix::Binary | Radix::Octal)) => {
                format::radix(quantity.value, radix).unwrap_or_else(|| quantity.value.to_string())
            }
            Some(Radix::Scientific) => format::scientific(quantity.value, settings.precision, style),
            Some(Radix::Decimal) | None => {
                if let Some(code) = quantity.unit.currency() {
                    return format::currency(quantity.value, code, style);
                }
                format::number(quantity.value, settings.precision, style)
            }
        };
        self.with_unit(&number, quantity, style, settings.input_language)
    }

    fn with_unit(
        &self,
        number: &str,
        quantity: &Quantity,
        style: NumberStyle,
        language: InputLanguage,
    ) -> String {
        let unit = &quantity.unit;
        match unit.factors.as_slice() {
            [] => number.to_owned(),
            [
                Factor {
                    unit: UnitRef::Percent,
                    power: 1,
                    ..
                },
            ] => format!("{number}%"),
            [
                Factor {
                    unit: UnitRef::Builtin(index),
                    power,
                    form,
                },
            ] => {
                let def = BUILTINS.def(*index);
                match (def.style, form, *power) {
                    (Style::Word, Form::Abbrev(abbrev), 1) => format!("{number} {abbrev}"),
                    (Style::Word, _, 1) => format!("{number} {}", word(*index, number, style, language)),
                    (Style::Word, _, power @ (2 | 3)) if language == InputLanguage::English => {
                        let word = plural(def.singular, def.plural, number);
                        let power = if power == 2 { "square" } else { "cubic" };
                        format!("{number} {power} {word}")
                    }
                    (Style::Word, _, power @ (2 | 3)) => {
                        let localized = vocab::vocab(language);
                        match (
                            localized.power_template(power, number, style),
                            localized.unit_word(*index, number, style),
                        ) {
                            (Some(template), Some(word)) => format!("{number} {}", compound(template, word)),
                            _ => format!("{number} {}", self.symbols(unit)),
                        }
                    }
                    (Style::Attached, _, _) => format!("{number}{}{}", def.symbol, superscript(*power)),
                    _ => format!("{number} {}", self.symbols(unit)),
                }
            }
            factors => {
                let positive: Vec<&Factor> = factors.iter().filter(|f| f.power > 0).collect();
                if let [money] = positive.as_slice()
                    && let UnitRef::Currency(code) = &money.unit
                    && money.power == 1
                    && currency::prefix_sign(code).is_some()
                    && quantity.radix.is_none()
                {
                    let per = Unit {
                        factors: factors.iter().filter(|f| f.power < 0).cloned().collect(),
                    };
                    let amount = format::currency(quantity.value, code, style);
                    return format!("{amount}/{}", self.symbols(&per.pow(-1)));
                }
                format!("{number} {}", self.symbols(unit))
            }
        }
    }

    /// The answer for a value.
    pub(crate) fn answer(&self, value: &Value, settings: &Settings, now: DateTime<Utc>) -> Answer {
        let text = self.display(value, settings, now);
        match value {
            Value::Quantity(quantity) => Answer {
                text,
                number: Some(quantity.value),
                unit: self.id(&quantity.unit),
            },
            Value::Moment(_) => Answer {
                text,
                number: None,
                unit: None,
            },
        }
    }

    /// A unit for an extension unit id: Numi base ids, custom ids, built-in ids, currency codes
    /// and finally any unit phrase.
    pub(crate) fn unit_for_id(&self, id: &str) -> Option<Unit> {
        if let Some((unit, power)) = custom::numi_base(id) {
            return Some(Unit::power_of(unit, power, Form::Default));
        }
        if let Some(index) = BUILTINS.by_id(id) {
            return Some(Unit::simple(UnitRef::Builtin(index), Form::Default));
        }
        if let Some(code) = currency::canonical(id) {
            return Some(Unit::simple(UnitRef::Currency(code.to_owned()), Form::Default));
        }
        if let Some(index) = self.custom.unit_by_id(id).or_else(|| self.custom.unit(id)) {
            return Some(Unit::simple(UnitRef::Custom(index), Form::Default));
        }
        let hit = BUILTINS.lookup(id)?;
        Some(Unit::simple(UnitRef::Builtin(hit.index), Form::Default))
    }
}

/// The scale factor and offset of an absolute temperature unit.
fn temperature(unit: &Unit) -> Option<(f64, f64)> {
    match unit.as_simple()? {
        UnitRef::Builtin(index) => match BUILTINS.def(*index).scale {
            Scale::Temperature { factor, offset } => Some((factor, offset)),
            _ => None,
        },
        _ => None,
    }
}

fn plural<'a>(singular: &'a str, plural: &'a str, number: &str) -> &'a str {
    if number == "1" || number == "-1" {
        singular
    } else {
        plural
    }
}

/// The word for a word-style unit after `number`: the input language's form when it has one,
/// else English.
fn word(index: usize, number: &str, style: NumberStyle, language: InputLanguage) -> &'static str {
    let def = BUILTINS.def(index);
    if language != InputLanguage::English
        && let Some(word) = vocab::vocab(language).unit_word(index, number, style)
    {
        return word;
    }
    plural(def.singular, def.plural, number)
}

fn display_moment(moment: &Moment, settings: &Settings, now: DateTime<Utc>) -> String {
    let naive = moment.naive();
    let time = match settings.time_format {
        TimeFormat::H24 => naive.format("%H:%M").to_string(),
        TimeFormat::H12 => naive.format("%-I:%M %P").to_string(),
    };
    let localized: Option<&Vocab> =
        (settings.input_language != InputLanguage::English).then(|| vocab::vocab(settings.input_language));
    let date = || {
        localized
            .and_then(|vocab| vocab.date(naive.date()))
            .unwrap_or_else(|| naive.format("%a, %-d %b %Y").to_string())
    };
    match moment.kind {
        MomentKind::Time => {
            // Today shows the time; within a week the weekday is unambiguous; beyond, the date.
            let days_away = (naive.date() - moment.zone.naive(now).date()).num_days().abs();
            match days_away {
                0 => time,
                1..=6 => {
                    let weekday = localized
                        .and_then(|vocab| vocab.weekday(naive.date()))
                        .map_or_else(|| naive.format("%a").to_string(), str::to_owned);
                    format!("{weekday} {time}")
                }
                _ => format!("{} {time}", date()),
            }
        }
        MomentKind::Date => date(),
        MomentKind::DateTime => format!("{} {time}", date()),
    }
}

/// Fills a display template: `Quadrat*` with `Fuß` is `Quadratfuß`, `* carré` with `pied` is
/// `pied carré`. A word joined to a letter loses its capital, as in German compounds.
fn compound(template: &str, word: &str) -> String {
    let joined = template
        .split('*')
        .next()
        .and_then(|head| head.chars().last())
        .is_some_and(char::is_alphabetic);
    if !joined {
        return template.replace('*', word);
    }
    let mut chars = word.chars();
    let lowered: String = chars
        .next()
        .map(|first| first.to_lowercase().chain(chars).collect())
        .unwrap_or_default();
    template.replace('*', &lowered)
}
