//! The globals an extension script sees (`numi.addUnit`, `numi.addFunction`, `numi.setVariable`,
//! `log`) and the conversion of values across the JavaScript boundary.
//!
//! Numi's conventions: a value is `{ double, unitId }` or a plain number, `phrases` is a
//! comma-separated string, and `baseUnitId` names a Numi base unit or another custom unit.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::mem;
use std::rc::Rc;

use napkin_core::{CustomUnit, ExtValue};
use rquickjs::function::Rest;
use rquickjs::{Array, CaughtError, Coerced, Ctx, Exception, Function, Object, Persistent, Value};

/// A function declared with `numi.addFunction`, still inside the JavaScript runtime.
pub(crate) struct DeclaredFunction {
    pub(crate) id: String,
    pub(crate) phrases: Vec<String>,
    pub(crate) function: Persistent<Function<'static>>,
}

/// Everything one script declared while it loaded, in call order per kind.
#[derive(Default)]
pub(crate) struct Declarations {
    pub(crate) units: Vec<CustomUnit>,
    pub(crate) functions: Vec<DeclaredFunction>,
    pub(crate) variables: Vec<(String, ExtValue)>,
    pub(crate) log: Vec<String>,
}

/// Collects one script's declarations while it loads.
///
/// Closed once the script has run: later `numi.*` calls (from inside a custom function) throw,
/// so no JavaScript value can end up in Rust after the host took the declarations, and `log`
/// calls are dropped.
#[derive(Default)]
pub(crate) struct Collector {
    declarations: Declarations,
    closed: bool,
}

/// The collector shared by the API functions of one context.
pub(crate) type SharedCollector = Rc<RefCell<Collector>>;

impl Collector {
    /// Ends the loading phase and hands out what the script declared.
    pub(crate) fn close(&mut self) -> Declarations {
        self.closed = true;
        mem::take(&mut self.declarations)
    }
}

/// Installs `numi`, `log` and `console.log` as globals of `ctx`.
pub(crate) fn install<'js>(ctx: &Ctx<'js>, collector: &SharedCollector) -> rquickjs::Result<()> {
    let numi = Object::new(ctx.clone())?;
    let add_unit = {
        let collector = Rc::clone(collector);
        move |ctx: Ctx<'js>, spec: Value<'js>| add_unit(&ctx, &collector, &spec)
    };
    numi.set(
        "addUnit",
        Function::new(ctx.clone(), add_unit)?.with_name("addUnit")?,
    )?;
    let add_function = {
        let collector = Rc::clone(collector);
        move |ctx: Ctx<'js>, spec: Value<'js>, function: Value<'js>| {
            add_function(&ctx, &collector, &spec, function)
        }
    };
    numi.set(
        "addFunction",
        Function::new(ctx.clone(), add_function)?.with_name("addFunction")?,
    )?;
    let set_variable = {
        let collector = Rc::clone(collector);
        move |ctx: Ctx<'js>, name: Value<'js>, value: Value<'js>| {
            set_variable(&ctx, &collector, &name, &value)
        }
    };
    numi.set(
        "setVariable",
        Function::new(ctx.clone(), set_variable)?.with_name("setVariable")?,
    )?;

    let log = {
        let collector = Rc::clone(collector);
        move |ctx: Ctx<'js>, args: Rest<Value<'js>>| log(&ctx, &collector, &args.0)
    };
    let log = Function::new(ctx.clone(), log)?.with_name("log")?;
    let console = Object::new(ctx.clone())?;
    console.set("log", log.clone())?;

    let globals = ctx.globals();
    globals.set("numi", numi)?;
    globals.set("log", log)?;
    globals.set("console", console)?;
    Ok(())
}

fn add_unit<'js>(ctx: &Ctx<'js>, collector: &SharedCollector, spec: &Value<'js>) -> rquickjs::Result<()> {
    let unit = unit_from_js(spec).map_err(|message| throw_type(ctx, &format!("numi.addUnit: {message}")))?;
    open_collector(ctx, collector, "numi.addUnit")?
        .declarations
        .units
        .push(unit);
    Ok(())
}

fn add_function<'js>(
    ctx: &Ctx<'js>,
    collector: &SharedCollector,
    spec: &Value<'js>,
    function: Value<'js>,
) -> rquickjs::Result<()> {
    let (id, phrases) = function_spec_from_js(spec)
        .map_err(|message| throw_type(ctx, &format!("numi.addFunction: {message}")))?;
    let Some(function) = function.into_function() else {
        return Err(throw_type(
            ctx,
            "numi.addFunction: the second argument must be a function",
        ));
    };
    let function = Persistent::save(ctx, function);
    open_collector(ctx, collector, "numi.addFunction")?
        .declarations
        .functions
        .push(DeclaredFunction {
            id,
            phrases,
            function,
        });
    Ok(())
}

fn set_variable<'js>(
    ctx: &Ctx<'js>,
    collector: &SharedCollector,
    name: &Value<'js>,
    value: &Value<'js>,
) -> rquickjs::Result<()> {
    let name = text(name)
        .ok()
        .flatten()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| throw_type(ctx, "numi.setVariable: the name must be a non-empty string"))?;
    let value = ext_value_from_js(value)
        .map_err(|message| throw_type(ctx, &format!("numi.setVariable({name:?}): {message}")))?;
    open_collector(ctx, collector, "numi.setVariable")?
        .declarations
        .variables
        .push((name, value));
    Ok(())
}

fn log<'js>(ctx: &Ctx<'js>, collector: &SharedCollector, args: &[Value<'js>]) {
    let line = args
        .iter()
        .map(|value| display_value(ctx, value))
        .collect::<Vec<_>>()
        .join(" ");
    let mut collector = collector.borrow_mut();
    if !collector.closed {
        collector.declarations.log.push(line);
    }
}

/// The collector, unless loading is over. Converts arguments before borrowing so that property
/// getters running script code can never re-enter a held borrow.
fn open_collector<'a>(
    ctx: &Ctx<'_>,
    collector: &'a SharedCollector,
    api: &str,
) -> rquickjs::Result<std::cell::RefMut<'a, Collector>> {
    let collector = collector.borrow_mut();
    if collector.closed {
        return Err(throw_type(
            ctx,
            &format!("{api} is only available while the extension loads"),
        ));
    }
    Ok(collector)
}

fn throw_type(ctx: &Ctx<'_>, message: &str) -> rquickjs::Error {
    Exception::throw_type(ctx, message)
}

/// Reads a `numi.addUnit` spec. `id` and `phrases` are required; `baseUnitId` defaults to the
/// unit's own id (a unit of its own dimension, which other units can be based on), `format` to
/// the first phrase and `ratio` to 1.
pub(crate) fn unit_from_js(spec: &Value<'_>) -> Result<CustomUnit, String> {
    let object =
        plain_object(spec).ok_or("expects an object like { id, phrases, baseUnitId, format, ratio }")?;
    let id = required_text(object, "id")?;
    let phrases = phrases(object)?;
    let base_unit_id = optional_text(object, "baseUnitId")?.unwrap_or_else(|| id.clone());
    let format = optional_text(object, "format")?.unwrap_or_else(|| phrases[0].clone());
    let ratio = number_property(object, "ratio")?.unwrap_or(1.0);
    Ok(CustomUnit {
        id,
        phrases,
        base_unit_id,
        ratio,
        format,
    })
}

/// Reads a `numi.addFunction` spec: the id and the phrases the function answers to.
pub(crate) fn function_spec_from_js(spec: &Value<'_>) -> Result<(String, Vec<String>), String> {
    let object = plain_object(spec).ok_or("expects an object like { id, phrases } as the first argument")?;
    Ok((required_text(object, "id")?, phrases(object)?))
}

/// Converts a JavaScript value (`{ double, unitId }` or a plain number) to an [`ExtValue`].
pub(crate) fn ext_value_from_js(value: &Value<'_>) -> Result<ExtValue, String> {
    let ext_value = if let Some(number) = value.as_number() {
        ExtValue {
            double: number,
            unit_id: None,
        }
    } else if let Some(object) = plain_object(value) {
        let double = number_property(object, "double")?.ok_or("the value has no \"double\" number")?;
        let unit_id = optional_text(object, "unitId")?;
        ExtValue { double, unit_id }
    } else if value.is_undefined() || value.is_null() {
        return Err("no value".to_owned());
    } else {
        return Err(format!(
            "expected a number or {{ double, unitId }}, got a {}",
            value.type_name()
        ));
    };
    if ext_value.double.is_finite() {
        Ok(ext_value)
    } else {
        Err(format!("{} is not a finite number", ext_value.double))
    }
}

/// Converts an [`ExtValue`] to `{ double, unitId }` (`unitId` omitted for plain numbers).
pub(crate) fn ext_value_to_js<'js>(ctx: &Ctx<'js>, value: &ExtValue) -> rquickjs::Result<Object<'js>> {
    let object = Object::new(ctx.clone())?;
    object.set("double", value.double)?;
    if let Some(unit_id) = &value.unit_id {
        object.set("unitId", unit_id.as_str())?;
    }
    Ok(object)
}

/// The argument array a custom function receives.
pub(crate) fn arguments_to_js<'js>(ctx: &Ctx<'js>, values: &[ExtValue]) -> rquickjs::Result<Array<'js>> {
    let array = Array::new(ctx.clone())?;
    for (index, value) in values.iter().enumerate() {
        array.set(index, ext_value_to_js(ctx, value)?)?;
    }
    Ok(array)
}

/// A one-line description of a JavaScript exception: `TypeError: message (at file.js:3:5)`.
pub(crate) fn describe_error<'js>(ctx: &Ctx<'js>, error: CaughtError<'js>) -> String {
    match error {
        CaughtError::Exception(exception) => {
            let name: Option<String> = exception
                .get::<_, Option<Coerced<String>>>("name")
                .ok()
                .flatten()
                .map(|name| name.0);
            let message = exception.message().unwrap_or_default();
            let mut text = match name {
                Some(name) if !name.is_empty() && !message.is_empty() => format!("{name}: {message}"),
                Some(name) if !name.is_empty() => name,
                _ => message,
            };
            let frame = exception.stack().and_then(|stack| {
                stack
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty())
                    .map(str::to_owned)
            });
            if let Some(frame) = frame {
                let _ = write!(text, " ({frame})");
            }
            text
        }
        CaughtError::Value(value) => format!("uncaught exception: {}", display_value(ctx, &value)),
        CaughtError::Error(error) => error.to_string(),
    }
}

/// How `log` shows a value: strings as they are, objects as JSON, everything else via `String()`.
fn display_value<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> String {
    if let Some(string) = value.as_string() {
        return string.to_string().unwrap_or_default();
    }
    if value.is_object()
        && !value.is_function()
        && let Ok(Some(json)) = ctx.json_stringify(value.clone())
        && let Ok(json) = json.to_string()
    {
        return json;
    }
    value
        .get::<Coerced<String>>()
        .map_or_else(|_| format!("[{}]", value.type_name()), |coerced| coerced.0)
}

/// The value as an object, unless it is an array or a function.
fn plain_object<'a, 'js>(value: &'a Value<'js>) -> Option<&'a Object<'js>> {
    if value.is_array() || value.is_function() {
        return None;
    }
    value.as_object()
}

fn required_text(object: &Object<'_>, key: &str) -> Result<String, String> {
    optional_text(object, key)?.ok_or_else(|| format!("{key:?} is required"))
}

/// A string (or number) property, trimmed; `None` when missing, `null` or empty.
fn optional_text(object: &Object<'_>, key: &str) -> Result<Option<String>, String> {
    let value: Value<'_> = object
        .get(key)
        .map_err(|error| format!("could not read {key:?}: {error}"))?;
    text(&value).map_err(|kind| format!("{key:?} must be a string, got a {kind}"))
}

/// A string or number as trimmed text; `None` for `undefined`, `null` and blank strings.
fn text(value: &Value<'_>) -> Result<Option<String>, &'static str> {
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    let text = if let Some(string) = value.as_string() {
        string.to_string().map_err(|_| "invalid string")?
    } else if let Some(number) = value.as_number() {
        number.to_string()
    } else {
        return Err(value.type_name());
    };
    let text = text.trim();
    Ok((!text.is_empty()).then(|| text.to_owned()))
}

/// A number (or numeric string) property; `None` when missing or `null`.
fn number_property(object: &Object<'_>, key: &str) -> Result<Option<f64>, String> {
    let value: Value<'_> = object
        .get(key)
        .map_err(|error| format!("could not read {key:?}: {error}"))?;
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    if let Some(number) = value.as_number() {
        return Ok(Some(number));
    }
    let parsed = text(&value)
        .ok()
        .flatten()
        .and_then(|text| text.parse::<f64>().ok());
    parsed
        .map(Some)
        .ok_or_else(|| format!("{key:?} must be a number"))
}

/// `phrases`: Numi's comma-separated string, or an array of strings. Trimmed, blanks and
/// duplicates dropped, order kept; at least one phrase is required.
fn phrases(object: &Object<'_>) -> Result<Vec<String>, String> {
    let value: Value<'_> = object
        .get("phrases")
        .map_err(|error| format!("could not read \"phrases\": {error}"))?;
    let mut raw = Vec::new();
    if let Some(array) = value.as_array() {
        for item in array.iter::<Value<'_>>() {
            let item = item.map_err(|error| format!("could not read \"phrases\": {error}"))?;
            raw.extend(text(&item).map_err(|kind| format!("\"phrases\" must hold strings, got a {kind}"))?);
        }
    } else {
        raw.extend(text(&value).map_err(|kind| format!("\"phrases\" must be a string, got a {kind}"))?);
    }
    let mut phrases: Vec<String> = Vec::new();
    for phrase in raw.iter().flat_map(|text| text.split(',')).map(str::trim) {
        if !phrase.is_empty() && !phrases.iter().any(|known| known == phrase) {
            phrases.push(phrase.to_owned());
        }
    }
    if phrases.is_empty() {
        return Err("\"phrases\" needs at least one phrase".to_owned());
    }
    Ok(phrases)
}
