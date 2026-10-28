//! Napkin's calculation engine: natural-language math with units, currencies, percentages,
//! dates and time zones, compatible with the syntax of [Numi](https://numi.app).
//!
//! The engine evaluates a whole document line by line, because lines can refer to each other
//! (variables, `prev`, `sum`, `avg`, `em = 20px`). Every line yields highlight spans for the
//! editor and, when it computes something, a formatted [`Answer`].
//!
//! ```
//! use napkin_core::{Engine, LineResult};
//!
//! let engine = Engine::new();
//! let lines = engine.evaluate("price = 12\nprice * 3");
//! assert!(matches!(&lines[1].result, LineResult::Value(answer) if answer.text == "36"));
//! ```
//!
//! STUB: this file pins the public API that the app, the CLI and the extension host build on.
//! The engine implementation replaces the bodies; the signatures below stay stable.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Evaluates Napkin documents. Cheap to share: `Engine` is `Send + Sync`.
#[derive(Debug, Clone, Default)]
pub struct Engine {
    settings: Settings,
    rates: ExchangeRates,
    clock: Clock,
    units: Vec<CustomUnit>,
    functions: Vec<CustomFunction>,
    globals: BTreeMap<String, ExtValue>,
}

impl Engine {
    /// An engine with default [`Settings`], the bundled exchange rates and the system clock.
    pub fn new() -> Self {
        Self::default()
    }

    /// An engine with the given settings.
    pub fn with_settings(settings: Settings) -> Self {
        Self { settings, ..Self::default() }
    }

    /// The current settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Replaces the settings; registered units, functions and globals stay.
    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    /// The exchange rates used for currency conversion.
    pub fn exchange_rates(&self) -> &ExchangeRates {
        &self.rates
    }

    /// Replaces the exchange rates (after a download, for example).
    pub fn set_exchange_rates(&mut self, rates: ExchangeRates) {
        self.rates = rates;
    }

    /// Replaces the clock; tests and screenshots use [`Clock::Fixed`].
    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
    }

    /// Evaluates a document. Returns exactly one [`LineOutput`] per line of `document`
    /// (split on `\n`; a trailing `\r` belongs to no span). Span ranges are byte offsets
    /// into their own line.
    pub fn evaluate(&self, document: &str) -> Vec<LineOutput> {
        document.split('\n').map(|line| self.evaluate_stub_line(line)).collect()
    }

    /// Evaluates a single expression without document context (CLI, Alfred).
    ///
    /// # Errors
    ///
    /// Returns [`EvalError`] when the input does not compute anything or fails to evaluate.
    pub fn evaluate_expression(&self, input: &str) -> Result<Answer, EvalError> {
        match self.evaluate_stub_line(input).result {
            LineResult::Value(answer) => Ok(answer),
            LineResult::Error(error) => Err(error),
            LineResult::None => Err(EvalError::new("nothing to calculate")),
        }
    }

    /// Registers a ratio-based custom unit (Numi `numi.addUnit`).
    ///
    /// # Errors
    ///
    /// Returns [`RegisterError`] when the id is already taken, `phrases` is empty, the ratio is
    /// not finite and positive, or `base_unit_id` is not a known base unit.
    pub fn register_unit(&mut self, unit: CustomUnit) -> Result<(), RegisterError> {
        self.units.push(unit);
        Ok(())
    }

    /// Registers a custom function (Numi `numi.addFunction`).
    ///
    /// # Errors
    ///
    /// Returns [`RegisterError`] when the id is already taken or `phrases` is empty.
    pub fn register_function(&mut self, function: CustomFunction) -> Result<(), RegisterError> {
        self.functions.push(function);
        Ok(())
    }

    /// Declares a global variable visible in every document (Numi `numi.setVariable`).
    ///
    /// # Errors
    ///
    /// Returns [`RegisterError`] when `name` is not a valid variable name or names a unit,
    /// keyword or function.
    pub fn set_global_variable(&mut self, name: &str, value: ExtValue) -> Result<(), RegisterError> {
        self.globals.insert(name.to_owned(), value);
        Ok(())
    }

    fn evaluate_stub_line(&self, line: &str) -> LineOutput {
        let trimmed = line.trim();
        let Ok(number) = trimmed.parse::<f64>() else {
            return LineOutput { spans: Vec::new(), result: LineResult::None };
        };
        let start = line.len() - line.trim_start().len();
        LineOutput {
            spans: vec![Span { range: start..start + trimmed.len(), kind: SpanKind::Number }],
            result: LineResult::Value(Answer { text: number.to_string(), number: Some(number), unit: None }),
        }
    }
}

/// The evaluation of one line.
#[derive(Debug, Clone, PartialEq)]
pub struct LineOutput {
    /// Highlight spans in source order, non-overlapping, byte offsets into the line.
    pub spans: Vec<Span>,
    /// What the line computes.
    pub result: LineResult,
}

/// What a line computes.
#[derive(Debug, Clone, PartialEq)]
pub enum LineResult {
    /// Nothing to show: empty line, header, comment, label-only or plain prose.
    None,
    /// A computed value.
    Value(Answer),
    /// The line looks like a calculation but failed (unknown currency, incompatible units, ...).
    /// The app shows nothing for it, like Numi; the CLI prints the message.
    Error(EvalError),
}

/// A computed, formatted result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    /// Display text, formatted with the current [`Settings`]: `"$1,234.50"`, `"120 cm"`, `"13:00"`.
    pub text: String,
    /// The plain numeric value in `unit`, when the result is a number (not a date or time).
    pub number: Option<f64>,
    /// The unit id of `number` (`"cm"`, `"USD"`, `"percent"`), `None` for plain numbers.
    pub unit: Option<String>,
}

/// A highlighted region of a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Byte range within the line.
    pub range: Range<usize>,
    /// What the region is.
    pub kind: SpanKind,
}

/// Syntax categories for highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpanKind {
    /// `42`, `0x1F`, `5 300`, `2k`.
    Number,
    /// `cm`, `kg`, `GB`, `°C`, `km/h`.
    Unit,
    /// `$`, `EUR`, `euro`, `BTC`.
    Currency,
    /// `+`, `*`, `^`, `(`, `)`, `=`, `%`.
    Operator,
    /// Word operators and connectives: `in`, `to`, `as`, `of`, `on`, `off`, `times`, `plus`.
    Keyword,
    /// A user or global variable, also `prev`, `sum`, `total`, `avg`, `average`.
    Variable,
    /// `sqrt`, `sin`, `round`, `fromunix`, custom functions.
    Function,
    /// `pi`, `e`.
    Constant,
    /// `today`, `now`, `time`, time zone and city names, dates and times of day.
    DateTime,
    /// `// ...` and `"quoted text"`.
    Comment,
    /// A `# header` line.
    Header,
    /// `Label:` prefix.
    Label,
    /// Words the engine ignores (prose between the numbers).
    Text,
}

/// Why a line failed to evaluate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalError {
    message: String,
}

impl EvalError {
    /// An error with a human-readable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    /// The human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for EvalError {}

/// Why registering an extension unit, function or variable failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RegisterError {
    message: String,
}

impl RegisterError {
    /// An error with a human-readable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

/// User-facing formatting and evaluation preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Maximum decimal places in results (trailing zeros trimmed). Numi's default is 2.
    pub precision: u8,
    /// Digit grouping and decimal separator for input and output.
    pub number_style: NumberStyle,
    /// How trigonometric functions read plain numbers.
    pub angle_unit: AngleUnit,
    /// How times of day are shown.
    pub time_format: TimeFormat,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            precision: 2,
            number_style: NumberStyle::English,
            angle_unit: AngleUnit::Degrees,
            time_format: TimeFormat::H24,
        }
    }
}

/// Digit grouping and decimal separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NumberStyle {
    /// `1,234.56`
    #[default]
    English,
    /// `1.234,56`
    German,
    /// `1'234.56`
    Swiss,
    /// `1 234,56`
    French,
    /// `1234.56`
    Plain,
}

/// Unit for plain numbers passed to trigonometric functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AngleUnit {
    /// `sin 30` = 0.5
    #[default]
    Degrees,
    /// `sin 30` = -0.99
    Radians,
}

/// Clock style for times of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TimeFormat {
    /// `14:30`
    #[default]
    H24,
    /// `2:30 pm`
    H12,
}

/// Source of "now" and of the local time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Clock {
    /// The system clock and the system time zone.
    #[default]
    System,
    /// A fixed instant and time zone, for tests and screenshots.
    Fixed {
        /// The instant `now` evaluates to.
        now: DateTime<Utc>,
        /// The local time zone.
        time_zone: chrono_tz::Tz,
    },
}

/// Exchange rates against a base currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRates {
    /// ISO code of the base currency, normally `"USD"`.
    pub base: String,
    /// When the rates were published, if known.
    pub updated: Option<DateTime<Utc>>,
    /// Units of each currency per one unit of `base` (`"EUR": 0.9` means 1 USD = 0.9 EUR).
    /// Fiat by ISO 4217 code, crypto by ticker.
    pub rates: BTreeMap<String, f64>,
}

impl ExchangeRates {
    /// The snapshot compiled into the binary, used until a download succeeds.
    pub fn bundled() -> Self {
        serde_json::from_str(include_str!("../data/rates-fallback.json"))
            .expect("the bundled rates file is valid JSON checked by the test suite")
    }
}

impl Default for ExchangeRates {
    fn default() -> Self {
        Self::bundled()
    }
}

/// A value crossing the extension boundary, shaped like Numi's JavaScript values
/// (`{ "double": 5, "unitId": "USD" }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtValue {
    /// The number.
    pub double: f64,
    /// A unit id (`"USD"`, `"m"`, a custom unit id), `None` for plain numbers.
    #[serde(rename = "unitId", default, skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<String>,
}

/// A ratio-based unit added by an extension (Numi `numi.addUnit`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomUnit {
    /// Unique id, also usable as `unitId` in [`ExtValue`].
    pub id: String,
    /// Words and abbreviations that name the unit (`["horse", "horses", "hrs"]`).
    pub phrases: Vec<String>,
    /// Numi base unit id: `m`, `second`, `percent`, `m2`, `m3`, `bit`, `byte`, `radian`,
    /// `gram`, `USD`, or the id of another custom unit.
    pub base_unit_id: String,
    /// How many base units one of this unit is.
    pub ratio: f64,
    /// Suffix used when formatting results (`"hrs"`).
    pub format: String,
}

/// The callable behind a [`CustomFunction`].
pub type ExtFunction = Arc<dyn Fn(&[ExtValue]) -> Result<ExtValue, String> + Send + Sync>;

/// A function added by an extension (Numi `numi.addFunction`). Called as `zum(1; 2)`.
#[derive(Clone)]
pub struct CustomFunction {
    /// Unique id.
    pub id: String,
    /// Names the function answers to.
    pub phrases: Vec<String>,
    /// The implementation; receives the arguments in order.
    pub call: ExtFunction,
}

impl fmt::Debug for CustomFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomFunction")
            .field("id", &self.id)
            .field("phrases", &self.phrases)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Engine>();
    }

    #[test]
    fn bundled_rates_parse() {
        let rates = ExchangeRates::bundled();
        assert_eq!(rates.base, "USD");
        assert!(rates.rates.contains_key("EUR"));
        assert!(rates.rates.contains_key("BTC"));
    }
}
