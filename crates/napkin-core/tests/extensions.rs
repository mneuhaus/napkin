//! Extension hooks (Numi's `numi.addUnit`, `numi.addFunction`, `numi.setVariable`) with Rust
//! closures.

mod common;

use std::sync::Arc;

use napkin_core::{CustomFunction, CustomUnit, Engine, ExtValue, LineResult};

fn last(engine: &Engine, document: &str) -> String {
    match engine.evaluate(document).pop().map(|line| line.result) {
        Some(LineResult::Value(answer)) => answer.text,
        Some(LineResult::Error(error)) => format!("error: {error}"),
        Some(LineResult::None) | None => "none".to_owned(),
    }
}

fn unit(id: &str, phrases: &[&str], base: &str, ratio: f64, format: &str) -> CustomUnit {
    CustomUnit {
        id: id.to_owned(),
        phrases: phrases.iter().map(|p| (*p).to_owned()).collect(),
        base_unit_id: base.to_owned(),
        ratio,
        format: format.to_owned(),
    }
}

fn function(
    id: &str,
    phrases: &[&str],
    call: impl Fn(&[ExtValue]) -> Result<ExtValue, String> + Send + Sync + 'static,
) -> CustomFunction {
    CustomFunction {
        id: id.to_owned(),
        phrases: phrases.iter().map(|p| (*p).to_owned()).collect(),
        call: Arc::new(call),
    }
}

/// Numi's sample: `numi.addUnit({ id: "horse", phrases: "horse, horses, hrs", baseUnitId: "m",
/// format: "hrs", ratio: 2.4 })`.
#[test]
fn custom_unit_based_on_metres() {
    let mut engine = common::engine();
    engine
        .register_unit(unit("horse", &["horse", "horses", "hrs"], "m", 2.4, "hrs"))
        .expect("valid unit");
    assert_eq!(last(&engine, "5 horses in m"), "12 m");
    assert_eq!(last(&engine, "12 m in horses"), "5 hrs");
    assert_eq!(last(&engine, "1 horse + 60 cm"), "300 cm");
    assert_eq!(last(&engine, "3 hrs"), "3 hrs");
    let LineResult::Value(answer) = engine.evaluate("2 horses").remove(0).result else {
        panic!("a value")
    };
    assert_eq!(answer.unit.as_deref(), Some("horse"));
    let spans = &engine.evaluate("5 horses in m")[0].spans;
    assert_eq!(spans[1].kind, napkin_core::SpanKind::Unit);
}

/// Numi's fluid ounce sample: multi-word and dotted phrases, a cubic-metre base.
#[test]
fn custom_unit_with_phrases_and_volume_base() {
    let mut engine = common::engine();
    let phrases = ["fl oz", "floz", "fl. oz.", "fluid ounces", "fluid ounce"];
    engine
        .register_unit(unit("floz", &phrases, "m3", 0.000_029_573_5, "fl oz"))
        .expect("valid unit");
    assert_eq!(last(&engine, "10 fl oz in ml"), "295.74 ml");
    assert_eq!(last(&engine, "2 fl. oz. in ml"), "59.15 ml");
    assert_eq!(last(&engine, "1 l in floz"), "33.81 fl oz");
}

#[test]
fn custom_units_chain_and_found_dimensions() {
    let mut engine = common::engine();
    engine
        .register_unit(unit("px2", &["dots"], "px2", 1.0, "dots"))
        .expect("own dimension");
    engine
        .register_unit(unit("rem", &["rems"], "px2", 16.0, "rem"))
        .expect("based on a custom unit");
    engine
        .register_unit(unit("cent", &["cents"], "USD", 0.01, "¢"))
        .expect("currency base");
    engine
        .register_unit(unit("bp", &["bps", "basis points"], "percent", 0.01, "bp"))
        .expect("percent base");
    assert_eq!(last(&engine, "32 dots in rems"), "2 rem");
    assert!(
        last(&engine, "32 dots in m").starts_with("error"),
        "a dimension of its own"
    );
    assert_eq!(last(&engine, "250 cents in USD"), "$2.50");
    assert_eq!(last(&engine, "$1 in cents"), "100 ¢");
    assert_eq!(last(&engine, "50 bps in %"), "0.5%");
}

#[test]
fn register_unit_validates() {
    let mut engine = common::engine();
    engine
        .register_unit(unit("horse", &["horse"], "m", 2.4, "hrs"))
        .expect("valid unit");
    let duplicate = engine.register_unit(unit("horse", &["pony"], "m", 1.0, "p"));
    assert!(duplicate.is_err(), "ids are unique");
    assert!(
        engine.register_unit(unit("a", &[], "m", 1.0, "a")).is_err(),
        "phrases are required"
    );
    assert!(
        engine.register_unit(unit("b", &[" "], "m", 1.0, "b")).is_err(),
        "blank phrases do not count"
    );
    assert!(
        engine.register_unit(unit("c", &["c"], "m", 0.0, "c")).is_err(),
        "ratio must be positive"
    );
    assert!(
        engine
            .register_unit(unit("d", &["d"], "m", f64::NAN, "d"))
            .is_err(),
        "ratio must be finite"
    );
    assert!(
        engine
            .register_unit(unit("e", &["e"], "parsec", 1.0, "e"))
            .is_err(),
        "unknown base unit"
    );
    for base in [
        "m", "second", "percent", "m2", "m3", "bit", "byte", "radian", "gram", "USD", "horse",
    ] {
        let id = format!("on-{base}");
        engine
            .register_unit(unit(&id, &[&id], base, 1.0, &id))
            .unwrap_or_else(|e| panic!("{base}: {e}"));
    }
}

/// Numi's sample: `numi.addFunction({ id: "zum", phrases: "zum" }, values => values[0] + values[1])`.
#[test]
fn custom_function_with_semicolon_arguments() {
    let mut engine = common::engine();
    let zum = function("zum", &["zum"], |values| match values {
        [a, b] => Ok(ExtValue {
            double: a.double + b.double,
            unit_id: None,
        }),
        _ => Err("zum takes two values".to_owned()),
    });
    engine.register_function(zum).expect("valid function");
    assert_eq!(last(&engine, "zum(1;2)"), "3");
    assert_eq!(last(&engine, "zum(1; 2) * 10"), "30");
    assert_eq!(last(&engine, "ZUM(2;3)"), "5");
    assert_eq!(last(&engine, "zum(1)"), "error: zum takes two values");
    let spans = &engine.evaluate("zum(1;2)")[0].spans;
    assert_eq!(spans[0].kind, napkin_core::SpanKind::Function);
}

#[test]
fn custom_functions_see_and_return_units() {
    let mut engine = common::engine();
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    let double = function("double", &["double", "twice"], move |values| {
        log.lock().expect("lock poisoned").extend(values.iter().cloned());
        let [value] = values else {
            return Err("one value".to_owned());
        };
        Ok(ExtValue {
            double: value.double * 2.0,
            unit_id: value.unit_id.clone(),
        })
    });
    engine.register_function(double).expect("valid function");
    assert_eq!(last(&engine, "double($5)"), "$10");
    assert_eq!(last(&engine, "twice 3 m"), "6 m");
    let pc = function("pc", &["pc"], |values| match values {
        [from, to] => Ok(ExtValue {
            double: (to.double - from.double) / from.double * 100.0,
            unit_id: Some("percent".to_owned()),
        }),
        _ => Err("two values".to_owned()),
    });
    engine.register_function(pc).expect("valid function");
    assert_eq!(last(&engine, "pc(50; 75)"), "50%");
    let args = seen.lock().expect("lock poisoned").clone();
    assert_eq!(
        args[0],
        ExtValue {
            double: 5.0,
            unit_id: Some("USD".to_owned())
        }
    );
}

#[test]
fn register_function_validates() {
    let mut engine = common::engine();
    let ok = |_: &[ExtValue]| {
        Ok(ExtValue {
            double: 1.0,
            unit_id: None,
        })
    };
    engine
        .register_function(function("one", &["one"], ok))
        .expect("valid function");
    assert!(
        engine.register_function(function("one", &["uno"], ok)).is_err(),
        "ids are unique"
    );
    assert!(
        engine.register_function(function("two", &[], ok)).is_err(),
        "phrases are required"
    );
    assert!(
        engine.register_function(function("", &["three"], ok)).is_err(),
        "ids are required"
    );
}

#[test]
fn global_variables() {
    let mut engine = common::engine();
    engine
        .set_global_variable(
            "xxx",
            ExtValue {
                double: 5.0,
                unit_id: Some("USD".to_owned()),
            },
        )
        .expect("valid name");
    engine
        .set_global_variable(
            "yyy",
            ExtValue {
                double: 122.0,
                unit_id: None,
            },
        )
        .expect("valid name");
    assert_eq!(last(&engine, "xxx * 2"), "$10");
    assert_eq!(last(&engine, "yyy + 1"), "123");
    assert_eq!(last(&engine, "xxx = 1\nxxx"), "1", "documents may shadow globals");
    assert_eq!(
        last(&engine, "xxx"),
        "$5",
        "each document starts from the globals"
    );
    for bad in ["m", "sqrt", "in", "pi", "2fast", "with space", "", "kg", "EUR"] {
        assert!(
            engine
                .set_global_variable(
                    bad,
                    ExtValue {
                        double: 1.0,
                        unit_id: None
                    }
                )
                .is_err(),
            "{bad:?}"
        );
    }
    engine
        .set_global_variable(
            "broken",
            ExtValue {
                double: 1.0,
                unit_id: Some("nope".to_owned()),
            },
        )
        .expect("valid name");
    assert!(
        last(&engine, "broken + 1").starts_with("error"),
        "unknown unit ids fail where used"
    );
}

#[test]
fn engine_with_extensions_is_shareable() {
    let mut engine = common::engine();
    engine
        .register_unit(unit("horse", &["horse"], "m", 2.4, "hrs"))
        .expect("valid unit");
    let shared = Arc::new(engine);
    let worker = {
        let engine = Arc::clone(&shared);
        std::thread::spawn(move || last(&engine, "2 horse in m"))
    };
    assert_eq!(worker.join().expect("thread finished"), "4.8 m");
}
