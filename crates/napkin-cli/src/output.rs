//! Rendering answers and documents as text or JSON.

use std::io::{self, Write};

use napkin_core::{Answer, EvalError, LineOutput, LineResult};
use serde::Serialize;

/// How results are printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    /// One-shot: the result. Document: `line  → result` per line, the bare line when it has none.
    Text,
    /// Document: only the results, one per input line, blank for lines without one.
    ResultsOnly,
    /// One JSON object per expression, a JSON array of them per document.
    Json,
}

/// The JSON shape of one evaluated line. Both `result` and `error` are always present (`null`
/// when absent), so consumers need no key checks.
#[derive(Debug, Serialize)]
struct JsonLine<'a> {
    input: &'a str,
    result: Option<&'a Answer>,
    error: Option<&'a str>,
}

impl<'a> JsonLine<'a> {
    fn new(input: &'a str, result: &'a LineResult) -> Self {
        match result {
            LineResult::Value(answer) => Self {
                input,
                result: Some(answer),
                error: None,
            },
            LineResult::Error(error) => Self {
                input,
                result: None,
                error: Some(error.message()),
            },
            LineResult::None => Self {
                input,
                result: None,
                error: None,
            },
        }
    }
}

/// Writes the outcome of a one-shot expression. Errors produce no text output (the caller
/// reports them on stderr); in JSON they become `{"input", "result": null, "error"}`.
pub(crate) fn write_answer(
    out: &mut impl Write,
    input: &str,
    result: &Result<Answer, EvalError>,
    format: Format,
) -> io::Result<()> {
    match (format, result) {
        (Format::Json, Ok(answer)) => write_json(
            out,
            &JsonLine {
                input,
                result: Some(answer),
                error: None,
            },
        ),
        (Format::Json, Err(error)) => write_json(
            out,
            &JsonLine {
                input,
                result: None,
                error: Some(error.message()),
            },
        ),
        (Format::Text | Format::ResultsOnly, Ok(answer)) => writeln!(out, "{}", answer.text),
        (Format::Text | Format::ResultsOnly, Err(_)) => Ok(()),
    }
}

/// Writes an evaluated document: `lines` holds one output per line of `document`.
pub(crate) fn write_document(
    out: &mut impl Write,
    document: &str,
    lines: &[LineOutput],
    format: Format,
) -> io::Result<()> {
    let inputs = document
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    let pairs = inputs.zip(lines.iter().map(|line| &line.result));
    match format {
        Format::Json => {
            let lines: Vec<JsonLine<'_>> = pairs
                .map(|(input, result)| JsonLine::new(input, result))
                .collect();
            write_json(out, &lines)
        }
        Format::Text => {
            for (input, result) in pairs {
                match result {
                    LineResult::Value(answer) => writeln!(out, "{input}  → {}", answer.text)?,
                    LineResult::Error(error) => writeln!(out, "{input}  → error: {}", error.message())?,
                    LineResult::None => writeln!(out, "{input}")?,
                }
            }
            Ok(())
        }
        Format::ResultsOnly => {
            for (_, result) in pairs {
                match result {
                    LineResult::Value(answer) => writeln!(out, "{}", answer.text)?,
                    LineResult::None | LineResult::Error(_) => writeln!(out)?,
                }
            }
            Ok(())
        }
    }
}

fn write_json(out: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    writeln!(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str, number: Option<f64>, unit: Option<&str>) -> LineOutput {
        LineOutput {
            spans: Vec::new(),
            result: LineResult::Value(Answer {
                text: text.to_owned(),
                number,
                unit: unit.map(str::to_owned),
            }),
        }
    }

    fn none() -> LineOutput {
        LineOutput {
            spans: Vec::new(),
            result: LineResult::None,
        }
    }

    fn error(message: &str) -> LineOutput {
        LineOutput {
            spans: Vec::new(),
            result: LineResult::Error(EvalError::new(message)),
        }
    }

    fn render(document: &str, lines: &[LineOutput], format: Format) -> String {
        let mut out = Vec::new();
        write_document(&mut out, document, lines, format).expect("writing to a Vec works");
        String::from_utf8(out).expect("UTF-8")
    }

    const DOCUMENT: &str = "# Costs\r\n20 inches in cm\n5 m + 2 kg";

    fn lines() -> Vec<LineOutput> {
        vec![
            none(),
            value("50.8 cm", Some(50.8), Some("cm")),
            error("incompatible units"),
        ]
    }

    #[test]
    fn text_document_shows_results_and_errors_next_to_the_lines() {
        assert_eq!(
            render(DOCUMENT, &lines(), Format::Text),
            "# Costs\n20 inches in cm  → 50.8 cm\n5 m + 2 kg  → error: incompatible units\n"
        );
    }

    #[test]
    fn results_only_keeps_one_row_per_line() {
        assert_eq!(render(DOCUMENT, &lines(), Format::ResultsOnly), "\n50.8 cm\n\n");
    }

    #[test]
    fn json_document_is_an_array_of_lines() {
        let json: serde_json::Value =
            serde_json::from_str(&render(DOCUMENT, &lines(), Format::Json)).expect("valid JSON");
        assert_eq!(
            json,
            serde_json::json!([
                { "input": "# Costs", "result": null, "error": null },
                { "input": "20 inches in cm", "result": { "text": "50.8 cm", "number": 50.8, "unit": "cm" }, "error": null },
                { "input": "5 m + 2 kg", "result": null, "error": "incompatible units" },
            ])
        );
    }

    #[test]
    fn one_shot_answers() {
        let answer = Answer {
            text: "$8".to_owned(),
            number: Some(8.0),
            unit: Some("USD".to_owned()),
        };
        let mut out = Vec::new();
        write_answer(&mut out, "$5 + 3", &Ok(answer.clone()), Format::Text).expect("write");
        assert_eq!(out, b"$8\n");

        let mut out = Vec::new();
        write_answer(&mut out, "$5 + 3", &Ok(answer), Format::Json).expect("write");
        let json: serde_json::Value = serde_json::from_slice(&out).expect("valid JSON");
        assert_eq!(json["result"]["text"], "$8");
        assert_eq!(json["error"], serde_json::Value::Null);

        let mut out = Vec::new();
        write_answer(&mut out, "5 m + 2 kg", &Err(EvalError::new("nope")), Format::Text).expect("write");
        assert!(out.is_empty(), "errors go to stderr");

        let mut out = Vec::new();
        write_answer(&mut out, "5 m + 2 kg", &Err(EvalError::new("nope")), Format::Json).expect("write");
        let json: serde_json::Value = serde_json::from_slice(&out).expect("valid JSON");
        assert_eq!(
            json,
            serde_json::json!({ "input": "5 m + 2 kg", "result": null, "error": "nope" })
        );
    }
}
