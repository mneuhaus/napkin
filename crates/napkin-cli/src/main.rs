//! `napkin`: evaluate Napkin expressions from the command line.
//!
//! ```text
//! napkin "20 inches in cm"         one expression: 50.8 cm
//! napkin < budget.numi             a document: `line  → result` per line
//! napkin -f budget.numi --json     machine-readable
//! napkin serve                     Numi's Alfred endpoint on 127.0.0.1:15055
//! ```

mod output;
mod serve;
mod setup;

use std::fs;
use std::io::{self, BufWriter, IsTerminal as _, Read as _, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context as _;
use clap::{CommandFactory as _, Parser, Subcommand};

use crate::output::Format;
use crate::setup::EngineArgs;

/// Napkin, the natural-language calculator (Numi-compatible), on the command line.
#[derive(Debug, Parser)]
#[command(name = "napkin", version, allow_negative_numbers = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Expression to evaluate; without it a document is read from --file or stdin
    #[arg(value_name = "EXPRESSION", conflicts_with = "file")]
    expression: Vec<String>,

    /// Evaluate a document file (.numi, .txt) line by line
    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,

    /// Print only the results of a document, one row per input line (blank for none)
    #[arg(long, conflicts_with = "json")]
    results_only: bool,

    /// Print machine-readable JSON (an object per expression, an array of them per document)
    #[arg(long)]
    json: bool,

    #[command(flatten)]
    engine: EngineArgs,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Answer http://127.0.0.1:PORT/?q=<expression> with the result as plain text (Numi's Alfred API)
    Serve {
        /// Port to listen on (0 picks a free one)
        #[arg(long, default_value_t = serve::DEFAULT_PORT)]
        port: u16,
    },
}

/// Exit code for usage problems, as clap uses for invalid arguments.
const USAGE_ERROR: u8 = 2;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => code,
        // `napkin < big.numi | head` closes the pipe early; that is not an error.
        Err(error) if is_broken_pipe(&error) => ExitCode::SUCCESS,
        Err(error) => {
            // Nothing sensible is left to do when stderr itself fails.
            let _ = writeln!(io::stderr(), "napkin: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> anyhow::Result<ExitCode> {
    let mut stderr = io::stderr().lock();
    let engine = setup::engine(&cli.engine, &mut stderr)?;
    if let Some(Command::Serve { port }) = cli.command {
        serve::run(&engine, port, &mut stderr)?;
        return Ok(ExitCode::SUCCESS);
    }

    let format = if cli.json {
        Format::Json
    } else if cli.results_only {
        Format::ResultsOnly
    } else {
        Format::Text
    };
    let mut stdout = BufWriter::new(io::stdout().lock());

    if !cli.expression.is_empty() {
        let input = cli.expression.join(" ");
        let result = engine.evaluate_expression(&input);
        output::write_answer(&mut stdout, &input, &result, format)?;
        stdout.flush()?;
        return match result {
            Ok(_) => Ok(ExitCode::SUCCESS),
            Err(error) => {
                writeln!(stderr, "napkin: {error}")?;
                Ok(ExitCode::FAILURE)
            }
        };
    }

    let document = match &cli.file {
        Some(path) => {
            fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?
        }
        None if io::stdin().is_terminal() => {
            writeln!(stderr, "{}", Cli::command().render_help())?;
            return Ok(ExitCode::from(USAGE_ERROR));
        }
        None => {
            let mut document = String::new();
            io::stdin()
                .read_to_string(&mut document)
                .context("could not read stdin")?;
            document
        }
    };
    // A final newline ends the last line rather than starting an empty one.
    let document = document.strip_suffix('\n').unwrap_or(&document);
    let lines = if document.is_empty() {
        Vec::new()
    } else {
        engine.evaluate(document)
    };
    output::write_document(&mut stdout, document, &lines, format)?;
    stdout.flush()?;
    Ok(ExitCode::SUCCESS)
}

fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|io| io.kind() == io::ErrorKind::BrokenPipe)
    })
}
