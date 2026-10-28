//! Behaviour of the extension host: errors, isolation, limits, threads and folder loading.

use std::fs;
use std::path::Path;
use std::thread;

use napkin_core::{Engine, ExtValue};
use napkin_extensions::{Extension, LoadError, load_into, load_source};
use tempfile::TempDir;

fn number(double: f64) -> ExtValue {
    ExtValue {
        double,
        unit_id: None,
    }
}

fn load(source: &str) -> Extension {
    load_source("test.js", source).unwrap_or_else(|error| panic!("loads: {error}"))
}

fn load_error(source: &str) -> String {
    match load_source("broken.js", source) {
        Ok(extension) => panic!("expected an error, loaded {extension:?}"),
        Err(error) => error.to_string(),
    }
}

fn call(extension: &Extension, id: &str, args: &[ExtValue]) -> Result<ExtValue, String> {
    let function = extension
        .functions
        .iter()
        .find(|function| function.id == id)
        .expect("function exists");
    (function.call)(args)
}

fn write(dir: &Path, name: &str, source: &str) {
    fs::write(dir.join(name), source).expect("write fixture");
}

#[test]
fn values_cross_the_boundary_with_their_unit() {
    let extension = load(
        "numi.addFunction({ id: 'twice', phrases: 'twice' }, (values) => \
             ({ double: values[0].double * 2, unitId: values[0].unitId }));\
         numi.addFunction({ id: 'has_unit', phrases: 'has_unit' }, (values) => 'unitId' in values[0] ? 1 : 0);",
    );
    let usd = ExtValue {
        double: 21.0,
        unit_id: Some("USD".to_owned()),
    };
    assert_eq!(
        call(&extension, "twice", &[usd]),
        Ok(ExtValue {
            double: 42.0,
            unit_id: Some("USD".to_owned())
        })
    );
    assert_eq!(
        call(&extension, "twice", &[number(2.0)]),
        Ok(number(4.0)),
        "plain numbers stay plain"
    );
    assert_eq!(call(&extension, "has_unit", &[number(2.0)]), Ok(number(0.0)));
}

#[test]
fn variables_accept_objects_and_plain_numbers() {
    let extension = load(
        r#"numi.setVariable("xxx", { "double": 5, "unitId": "USD" });
           numi.setVariable("yyy", 122);"#,
    );
    assert_eq!(
        extension.variables,
        vec![
            (
                "xxx".to_owned(),
                ExtValue {
                    double: 5.0,
                    unit_id: Some("USD".to_owned())
                }
            ),
            ("yyy".to_owned(), number(122.0)),
        ]
    );
}

#[test]
fn log_collects_lines_while_loading() {
    let extension = load(r#"log("Function called!"); console.log("a", 1, { b: 2 }, [3], null);"#);
    assert_eq!(extension.log, ["Function called!", r#"a 1 {"b":2} [3] null"#]);
}

#[test]
fn a_javascript_exception_becomes_the_functions_error() {
    let extension = load(
        "numi.addFunction({ id: 'boom', phrases: 'boom' }, () => { throw new Error('kaboom'); });\
         numi.addFunction({ id: 'oops', phrases: 'oops' }, () => undefinedName + 1);",
    );
    let error = call(&extension, "boom", &[]).expect_err("throws");
    assert!(error.starts_with("Error: kaboom"), "{error}");
    let error = call(&extension, "oops", &[]).expect_err("throws");
    assert!(
        error.contains("ReferenceError") && error.contains("undefinedName"),
        "{error}"
    );
}

#[test]
fn unusable_return_values_are_errors() {
    let extension = load(
        "numi.addFunction({ id: 'nothing', phrases: 'nothing' }, () => {});\
         numi.addFunction({ id: 'nan', phrases: 'nan' }, () => NaN);\
         numi.addFunction({ id: 'text', phrases: 'text' }, () => 'five');\
         numi.addFunction({ id: 'later', phrases: 'later' }, async () => 5);",
    );
    for (id, expected) in [
        ("nothing", "no value"),
        ("nan", "finite"),
        ("text", "string"),
        ("later", "Promise"),
    ] {
        let error = call(&extension, id, &[]).expect_err("unusable");
        assert!(error.contains(expected), "{id}: {error}");
    }
}

#[test]
fn script_errors_name_the_problem() {
    let error = load_error("numi.addUnit({ phrases: 'x' });");
    assert!(
        error.contains("numi.addUnit") && error.contains("\"id\" is required"),
        "{error}"
    );
    let error = load_error("numi.addUnit({ id: 'x', phrases: ' , ' });");
    assert!(error.contains("at least one phrase"), "{error}");
    let error = load_error("numi.addFunction({ id: 'f', phrases: 'f' }, 42);");
    assert!(error.contains("must be a function"), "{error}");
    let error = load_error("numi.setVariable('v', 'text');");
    assert!(error.contains("numi.setVariable"), "{error}");
    let error = load_error("let = ;");
    assert!(
        error.contains("SyntaxError") && error.contains("broken.js"),
        "{error}"
    );
    let error = load_error("throw 'plain string';");
    assert!(error.contains("plain string"), "{error}");
}

#[test]
fn a_failing_script_registers_nothing() {
    let error = load_source(
        "half.js",
        "numi.addUnit({ id: 'a', phrases: 'a' }); throw new Error('late');",
    )
    .expect_err("throws after declaring");
    assert!(
        matches!(error, LoadError::Script(ref message) if message.contains("late")),
        "{error:?}"
    );
}

#[test]
fn numi_calls_after_loading_are_refused() {
    let extension = load(
        "numi.addFunction({ id: 'sneaky', phrases: 'sneaky' }, () => { numi.addUnit({ id: 'u', phrases: 'u' }); return 1; });",
    );
    let error = call(&extension, "sneaky", &[]).expect_err("addUnit inside a call");
    assert!(
        error.contains("only available while the extension loads"),
        "{error}"
    );
}

#[test]
fn unit_fields_accept_numeric_strings_and_phrase_arrays() {
    let extension =
        load("numi.addUnit({ id: 'x', phrases: ['a, b', 'c', 'a'], ratio: '2.5', baseUnitId: 'm' });");
    let unit = &extension.units[0];
    assert_eq!(unit.phrases, ["a", "b", "c"]);
    assert!((unit.ratio - 2.5).abs() < f64::EPSILON);
    assert_eq!(unit.format, "a", "format defaults to the first phrase");
    assert!(
        load_source(
            "bad.js",
            "numi.addUnit({ id: 'x', phrases: 'x', ratio: 'lots' });"
        )
        .is_err()
    );
}

#[test]
fn runaway_scripts_and_functions_time_out() {
    let error = load_error("while (true) {}");
    assert!(error.contains("timed out"), "{error}");

    let extension = load(
        "numi.addFunction({ id: 'spin', phrases: 'spin' }, () => { while (true) {} });\
         numi.addFunction({ id: 'one', phrases: 'one' }, () => 1);",
    );
    let error = call(&extension, "spin", &[]).expect_err("never returns");
    assert!(error.contains("timed out"), "{error}");
    assert_eq!(
        call(&extension, "one", &[]),
        Ok(number(1.0)),
        "the runtime survives a timeout"
    );
}

#[test]
fn deep_recursion_and_huge_allocations_are_exceptions() {
    let extension = load(
        "function down(n) { return down(n + 1) + 1; }\
         numi.addFunction({ id: 'down', phrases: 'down' }, () => down(0));\
         numi.addFunction({ id: 'hog', phrases: 'hog' }, () => { const a = []; while (true) a.push(new Array(1e6).fill(1)); });\
         numi.addFunction({ id: 'one', phrases: 'one' }, () => 1);",
    );
    let error = call(&extension, "down", &[]).expect_err("stack overflow");
    assert!(error.contains("RangeError") || error.contains("stack"), "{error}");
    assert!(call(&extension, "hog", &[]).is_err());
    assert_eq!(call(&extension, "one", &[]), Ok(number(1.0)));
}

#[test]
fn each_file_has_its_own_globals() {
    let dir = TempDir::new().expect("temp dir");
    write(
        dir.path(),
        "a.js",
        "let ratio = 2; function helper(x) { return x * ratio; }\
         numi.addFunction({ id: 'a', phrases: 'a' }, (v) => helper(v[0].double));",
    );
    write(
        dir.path(),
        "b.js",
        "let ratio = 10; function helper(x) { return x + ratio; }\
         numi.addFunction({ id: 'b', phrases: 'b' }, (v) => helper(v[0].double));",
    );
    let mut engine = Engine::new();
    let report = load_into(&mut engine, &[dir.path().to_path_buf()]);
    assert!(
        report.failed.is_empty(),
        "redeclaring `let` in another file works: {:?}",
        report.failed
    );

    let a = napkin_extensions::load_file(&dir.path().join("a.js")).expect("a loads");
    let b = napkin_extensions::load_file(&dir.path().join("b.js")).expect("b loads");
    assert_eq!(call(&a, "a", &[number(3.0)]), Ok(number(6.0)));
    assert_eq!(call(&b, "b", &[number(3.0)]), Ok(number(13.0)));
}

#[test]
fn functions_are_callable_from_many_threads() {
    let extension = load("numi.addFunction({ id: 'sq', phrases: 'sq' }, (v) => v[0].double * v[0].double);");
    let function = extension.functions[0].clone();
    drop(extension);
    let workers: Vec<_> = (0..8_i32)
        .map(|index| {
            let function = function.clone();
            thread::spawn(move || {
                let x = f64::from(index);
                (0..50).all(|_| (function.call)(&[number(x)]) == Ok(number(x * x)))
            })
        })
        .collect();
    for worker in workers {
        assert!(worker.join().expect("no panic"), "every call answered correctly");
    }
}

#[test]
fn one_broken_file_never_stops_the_others() {
    let dir = TempDir::new().expect("temp dir");
    write(
        dir.path(),
        "1-good.js",
        "log('first'); numi.addUnit({ id: 'one', phrases: 'one', baseUnitId: 'm' });",
    );
    write(
        dir.path(),
        "2-broken.js",
        "log('never runs'); this is not javascript",
    );
    write(
        dir.path(),
        "3-throws.js",
        "log('before the crash'); throw new TypeError('nope');",
    );
    write(dir.path(), "4-good.js", "numi.setVariable('answer', 42);");
    write(dir.path(), "notes.txt", "not an extension");
    fs::create_dir(dir.path().join("folder.js")).expect("mkdir");

    let mut engine = Engine::new();
    let missing = dir.path().join("missing");
    let report = load_into(
        &mut engine,
        &[missing, dir.path().to_path_buf(), dir.path().to_path_buf()],
    );

    let names = |paths: Vec<&Path>| -> Vec<String> {
        paths
            .iter()
            .map(|path| path.file_name().expect("file").to_string_lossy().into_owned())
            .collect()
    };
    assert_eq!(
        names(report.loaded.iter().map(std::path::PathBuf::as_path).collect()),
        ["1-good.js", "4-good.js"]
    );
    assert_eq!(
        names(report.failed.iter().map(|(path, _)| path.as_path()).collect()),
        ["2-broken.js", "3-throws.js"]
    );
    assert!(
        report.failed[0].1.contains("SyntaxError"),
        "{:?}",
        report.failed[0]
    );
    assert!(
        report.failed[1].1.contains("TypeError: nope"),
        "{:?}",
        report.failed[1]
    );
    assert_eq!(report.log, ["1-good.js: first", "3-throws.js: before the crash"]);
    assert!(report.is_loaded(&dir.path().join("4-good.js")));
    assert!(!report.is_loaded(&dir.path().join("3-throws.js")));
}

#[test]
fn nothing_to_load_is_an_empty_report() {
    let dir = TempDir::new().expect("temp dir");
    let mut engine = Engine::new();
    assert_eq!(
        load_into(&mut engine, &[dir.path().join("nope")]),
        napkin_extensions::LoadReport::default()
    );
    assert_eq!(
        load_into(&mut engine, &[]),
        napkin_extensions::LoadReport::default()
    );
}
