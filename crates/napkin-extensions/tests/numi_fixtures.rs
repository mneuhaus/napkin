//! Numi's sample extension and community extensions (copied unchanged into
//! `tests/fixtures/numi/`) run on the host and declare what they declare in Numi.

use std::path::{Path, PathBuf};

use napkin_core::{CustomUnit, Engine, ExtValue};
use napkin_extensions::{Extension, load_file, load_into};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/numi")
}

fn load(file: &str) -> Extension {
    load_file(&fixture_dir().join(file)).unwrap_or_else(|error| panic!("{file} loads: {error}"))
}

fn number(double: f64) -> ExtValue {
    ExtValue {
        double,
        unit_id: None,
    }
}

fn call(extension: &Extension, id: &str, args: &[f64]) -> Result<ExtValue, String> {
    let function = extension
        .functions
        .iter()
        .find(|function| function.id == id)
        .expect("function exists");
    let args: Vec<ExtValue> = args.iter().copied().map(number).collect();
    (function.call)(&args)
}

fn call_number(extension: &Extension, id: &str, args: &[f64]) -> f64 {
    let value = call(extension, id, args).unwrap_or_else(|error| panic!("{id}({args:?}) fails: {error}"));
    assert_eq!(value.unit_id, None, "{id}({args:?})");
    value.double
}

fn unit<'a>(extension: &'a Extension, id: &str) -> &'a CustomUnit {
    extension
        .units
        .iter()
        .find(|unit| unit.id == id)
        .unwrap_or_else(|| panic!("unit {id} exists"))
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} is not {expected}");
}

#[test]
fn sample_fluid_ounce_unit() {
    let extension = load("sample.js");
    assert_eq!(
        extension.units,
        vec![CustomUnit {
            id: "floz".to_owned(),
            phrases: [
                "fl oz",
                "floz",
                "fl. oz.",
                "fl.oz.",
                "oz. fl.",
                "oz.fl.",
                "ozfl",
                "fluid ounce",
                "fluid ounces",
                "fl ounce",
                "fl ounces"
            ]
            .map(str::to_owned)
            .to_vec(),
            base_unit_id: "m3".to_owned(),
            ratio: 0.000_029_573_5,
            format: "fl oz".to_owned(),
        }]
    );
    assert!(extension.functions.is_empty() && extension.variables.is_empty() && extension.log.is_empty());
}

#[test]
fn satoshis_are_based_on_bitcoin() {
    let sats = load("BitcoinSatoshis.js");
    let unit = unit(&sats, "sats");
    assert_eq!(unit.base_unit_id, "BTC");
    assert_close(unit.ratio, 1e-8);
    assert_eq!(
        unit.phrases,
        ["sats", "satoshis", "Satoshi", "Satoshis", "Sats", "SATS"]
    );
}

#[test]
fn canadian_change_units() {
    let change = load("CADChange.js");
    let ids: Vec<&str> = change.units.iter().map(|unit| unit.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "FiveCents",
            "TenCents",
            "TwentyFiveCents",
            "OneDollars",
            "TwoDollars"
        ]
    );
    assert!(change.units.iter().all(|unit| unit.base_unit_id == "CAD"));
    assert_close(unit(&change, "TwentyFiveCents").ratio, 0.25);
    assert_eq!(unit(&change, "TwoDollars").format, "Toonies");
}

#[test]
fn data_rates_are_generated_in_loops() {
    let rates = load("DataRates.js");
    assert_eq!(rates.units.len(), 18);
    assert_close(unit(&rates, "Mbps").ratio, 1e6);
    assert_close(unit(&rates, "kBps").ratio, 8e3);
    assert_close(unit(&rates, "kiBps").ratio, 1024.0 * 8.0);
    assert_close(unit(&rates, "Mibps").ratio, 1024.0 * 1024.0);
    assert_eq!(unit(&rates, "Gbps").phrases, ["Gbps", "Gigabits per second"]);
    assert_eq!(unit(&rates, "bps").format, "bits/s");
}

#[test]
fn a_unit_without_base_becomes_its_own_base() {
    let power = load("Electrical-conversion.js");
    let watt = unit(&power, "watt");
    assert_eq!(watt.base_unit_id, "watt");
    assert_eq!(watt.format, "W");
    assert_eq!(unit(&power, "kilowatt").base_unit_id, "watt");
}

#[test]
fn units_with_unknown_bases_still_load() {
    // "w" is no base unit; the engine rejects it at registration, the script itself is fine.
    let engine_power = load("EnginePower.js");
    assert_eq!(unit(&engine_power, "kw").base_unit_id, "w");
}

#[test]
fn pressure_declares_its_duplicate_unit_twice() {
    let pressure = load("Pressure.js");
    assert_eq!(pressure.units.len(), 11);
    let megapascals = pressure
        .units
        .iter()
        .filter(|unit| unit.id == "megapascal")
        .count();
    assert_eq!(megapascals, 2, "the engine decides about the duplicate");
    assert_eq!(unit(&pressure, "pascal").phrases, ["pascal", "Pa"]);
}

#[test]
fn speed_units() {
    let speed = load("Speed.js");
    assert_close(unit(&speed, "mph").ratio, 1.609_344);
    assert_eq!(unit(&speed, "kmh").phrases.len(), 6);
}

#[test]
fn combinatorics_functions() {
    let combinatorics = load("BasicCombinatorics.js");
    assert_close(call_number(&combinatorics, "choose", &[6.0, 3.0]), 20.0);
    assert_close(call_number(&combinatorics, "permute", &[6.0, 3.0]), 120.0);
}

#[test]
fn min_max_take_any_number_of_arguments() {
    let min_max = load("MinMax.js");
    assert_close(call_number(&min_max, "nmin", &[5.0, 3.0, 7.0]), 3.0);
    assert_close(call_number(&min_max, "nmax", &[7.0, 25.0]), 25.0);
}

#[test]
fn nett_uses_optional_chaining_and_a_default() {
    let nett = load("NettCalc.js");
    assert_close(call_number(&nett, "nett", &[12.1, 21.0]), 10.0);
    assert!((call_number(&nett, "nett", &[100.0]) - 83.333).abs() < 1e-3);
}

#[test]
fn percent_change_returns_a_percentage_or_nothing() {
    let change = load("PercentChange.js");
    let value = call(&change, "pc", &[10.0, 7.0]).expect("two arguments work");
    assert_eq!(value.unit_id.as_deref(), Some("percent"));
    assert_close(value.double, -30.0);
    let error = call(&change, "pc", &[10.0]).expect_err("one argument returns undefined");
    assert!(error.contains("no value"), "{error}");
}

#[test]
fn logarithm_with_a_base() {
    let logarithm = load("LogarithmCalc.js");
    assert_close(call_number(&logarithm, "log", &[2.0, 8.0]), 3.0);
}

#[test]
fn standard_deviation_answers_to_two_phrases() {
    let stddev = load("StandardDeviation.js");
    assert_eq!(stddev.functions[0].phrases, ["stddev", "sd"]);
    assert!((call_number(&stddev, "stddev", &[4.0, 21.0, 11.0, 6.0, 8.0]) - 5.966_574).abs() < 1e-6);
}

#[test]
fn clamp_limits_a_range() {
    let clamp = load("clamp.js");
    assert_close(call_number(&clamp, "clamp", &[15.0, 3.0, 7.0]), 7.0);
    assert_close(call_number(&clamp, "clamp", &[1.0, 3.0, 7.0]), 3.0);
    assert_close(call_number(&clamp, "clamp", &[5.0, 3.0, 7.0]), 5.0);
}

#[test]
fn screen_units_and_their_helper_function() {
    let screen = load("ScreenUnits.js");
    assert_eq!(unit(&screen, "rem").base_unit_id, "px");
    assert_close(unit(&screen, "rem").ratio, 16.0);
    assert_eq!(screen.functions[0].phrases, ["toRem", "convertRem"]);
    assert_close(call_number(&screen, "toRem", &[32.0]), 2.0);
    assert_close(call_number(&screen, "toRem", &[32.0, 8.0]), 4.0);
}

#[test]
fn vector_functions_keep_state_between_calls() {
    let vectors = load("VectorCalc.js");
    let u = call_number(&vectors, "vec", &[4.0, 5.0, -2.0]);
    let v = call_number(&vectors, "vec", &[1.0, 2.0, 1.0]);
    assert_close(u, 0.0);
    assert_close(v, 1.0);
    assert_close(call_number(&vectors, "vec", &[4.0, 5.0, -2.0]), 0.0);
    assert_close(call_number(&vectors, "dot", &[u, v]), 12.0);
    let angle = call(&vectors, "angle", &[u, v]).expect("two vectors");
    assert_eq!(angle.unit_id.as_deref(), Some("radian"));
    assert!((angle.double - 0.752).abs() < 1e-3, "{}", angle.double);
}

#[test]
fn the_whole_folder_loads_into_an_engine() {
    let mut engine = Engine::new();
    let report = load_into(&mut engine, &[fixture_dir()]);
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert_eq!(report.loaded.len(), 17);
    let names: Vec<String> = report
        .loaded
        .iter()
        .map(|path| path.file_name().expect("file").to_string_lossy().into_owned())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "files load in file-name order");
    assert!(report.is_loaded(&fixture_dir().join("sample.js")));
}
