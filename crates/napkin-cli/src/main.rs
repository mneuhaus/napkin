//! `napkin`: evaluate Napkin expressions from the command line.
//!
//! STUB: argument handling, document mode, JSON output and rate refresh get added.

use std::io::Write as _;

fn main() -> anyhow::Result<()> {
    let engine = napkin_core::Engine::new();
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let answer = engine.evaluate_expression(&input)?;
    writeln!(std::io::stdout().lock(), "{}", answer.text)?;
    Ok(())
}
