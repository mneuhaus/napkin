//! Building the engine: settings, cached (or freshly downloaded) rates, extensions.

use std::io::Write;
use std::path::PathBuf;

use clap::{Args, ValueEnum};
use napkin_core::{Engine, ExchangeRates, InputLanguage, NumberStyle, Settings};
use napkin_rates::RateCache;

/// Options that shape the engine, shared by every mode (also after `serve`).
#[derive(Debug, Args)]
pub(crate) struct EngineArgs {
    /// Download fresh exchange rates before evaluating (cached for the app and later calls)
    #[arg(long, global = true)]
    refresh_rates: bool,

    /// Do not load JavaScript extensions
    #[arg(long, global = true)]
    no_extensions: bool,

    /// Maximum number of decimal places in results (0-10) [default: 2]
    #[arg(long, global = true, value_name = "N", value_parser = clap::value_parser!(u8).range(0..=10))]
    precision: Option<u8>,

    /// Load extensions from DIR instead of Napkin's and Numi's extension folders (repeatable)
    #[arg(long = "extensions-dir", global = true, value_name = "DIR")]
    extension_dirs: Vec<PathBuf>,

    /// Exchange-rate cache file [default: ~/Library/Caches/Napkin/rates.json]
    #[arg(long, global = true, value_name = "FILE")]
    rates_cache: Option<PathBuf>,

    /// Input language besides English (words, unit names, dates): `auto` (`$NAPKIN_LANGUAGE`, else
    /// the system language) or one of en, af, bs-BA, cs, da, de, de-AT, de-CH, es, fi, fr, he, it,
    /// ja, ko, nl, nl-BE, no, pl, pt-BR, pt-PT, ro, ru, sv, tr, uk, zh-Hans, zh-Hant
    #[arg(long, global = true, value_name = "TAG", default_value = "auto", value_parser = parse_language)]
    language: Language,

    /// Digit grouping and decimal separator of input and results [default: english]
    #[arg(long, global = true, value_name = "STYLE", value_enum)]
    number_style: Option<Style>,

    /// Print extension logs and rejected registrations to stderr
    #[arg(short, long, global = true)]
    verbose: bool,
}

/// Builds the engine for `args`. Problems that still leave a working engine (a failed download,
/// a broken extension) are reported on `log` as warnings.
pub(crate) fn engine(args: &EngineArgs, log: &mut impl Write) -> anyhow::Result<Engine> {
    let mut settings = Settings::default();
    if let Some(precision) = args.precision {
        settings.precision = precision;
    }
    if let Some(style) = args.number_style {
        settings.number_style = style.into();
    }
    settings.input_language = match args.language {
        Language::Auto => crate::locale::automatic(),
        Language::Fixed(language) => language,
    };
    let mut engine = Engine::with_settings(settings);
    engine.set_exchange_rates(rates(args, log)?);
    if !args.no_extensions {
        load_extensions(&mut engine, args, log)?;
    }
    Ok(engine)
}

fn rates(args: &EngineArgs, log: &mut impl Write) -> anyhow::Result<ExchangeRates> {
    let Some(cache) = args
        .rates_cache
        .clone()
        .map(RateCache::new)
        .or_else(RateCache::default_location)
    else {
        if args.refresh_rates {
            writeln!(
                log,
                "napkin: warning: no cache folder for exchange rates; using the bundled rates"
            )?;
        }
        return Ok(ExchangeRates::bundled());
    };
    if !args.refresh_rates {
        return Ok(cache.load_or_bundled());
    }
    match cache.refresh() {
        Ok(rates) => Ok(rates),
        Err(error) => {
            writeln!(log, "napkin: warning: {error}; using the cached rates")?;
            Ok(cache.load_or_bundled())
        }
    }
}

fn load_extensions(engine: &mut Engine, args: &EngineArgs, log: &mut impl Write) -> anyhow::Result<()> {
    let dirs = if args.extension_dirs.is_empty() {
        napkin_extensions::default_extension_dirs()
    } else {
        args.extension_dirs.clone()
    };
    let report = napkin_extensions::load_into(engine, &dirs);
    for (path, reason) in &report.failed {
        writeln!(
            log,
            "napkin: warning: extension {} failed: {reason}",
            path.display()
        )?;
    }
    if args.verbose {
        for path in &report.loaded {
            writeln!(log, "napkin: loaded extension {}", path.display())?;
        }
        for line in &report.log {
            writeln!(log, "napkin: {line}")?;
        }
    }
    Ok(())
}

/// The `--language` choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    /// `$NAPKIN_LANGUAGE`, else the system language.
    Auto,
    Fixed(InputLanguage),
}

fn parse_language(text: &str) -> Result<Language, String> {
    if text.eq_ignore_ascii_case("auto") {
        return Ok(Language::Auto);
    }
    crate::locale::parse(text).map(Language::Fixed).ok_or_else(|| {
        let tags: Vec<&str> = InputLanguage::ALL.iter().map(|l| l.tag()).collect();
        format!("unknown language; use auto or one of {}", tags.join(", "))
    })
}

/// The `--number-style` choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Style {
    /// 1,234.56
    English,
    /// 1.234,56
    German,
    /// 1'234.56
    Swiss,
    /// 1 234,56
    French,
    /// 1234.56
    Plain,
}

impl From<Style> for NumberStyle {
    fn from(style: Style) -> Self {
        match style {
            Style::English => Self::English,
            Style::German => Self::German,
            Style::Swiss => Self::Swiss,
            Style::French => Self::French,
            Style::Plain => Self::Plain,
        }
    }
}
