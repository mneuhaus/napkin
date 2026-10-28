//! Turns raw tokens into semantic tokens, deciding what each word means from its context, and
//! assigns every token its highlight kind.
//!
//! English is built in; the selected input language's phrases ([`Lexicon`]) are tried before
//! English for every word, with these rules where the two (or two meanings of one phrase) meet:
//!
//! - Longer phrases win over shorter ones (`multipliziert mit` over `mit`).
//! - A user variable shadows every language word.
//! - English unit symbols keep their meaning: a language word that is spelt like one (`m`, also
//!   `m.`) cannot be a scale, unit or currency (`m` stays metre, not German `Million`).
//! - An operator word that has another meaning (`e` in Italian is "and" and Euler's number,
//!   `min` in Dutch is "minus" and minutes, `sin` in Spanish is "without" and sine) is an
//!   operator only between two operands; a conversion word that is also a unit (`em` in
//!   Portuguese) converts only when a unit, currency, zone or format follows.
//! - Percentage words (`von`, `of what is`) apply after a percentage before anything else.
//! - A word that is both a weight and a currency (`Pfund`, like English `pound`) is the
//!   currency on lines with money on them and the weight elsewhere.

use std::f64::consts::{E, PI};
use std::ops::Range;

use chrono::{NaiveDate, NaiveTime};
use chrono_tz::Tz;

use crate::SpanKind;
use crate::currency;
use crate::custom::{self, Registry};
use crate::format::Radix;
use crate::functions::Builtin;
use crate::lexer::{Raw, RawKind, Sym};
use crate::units::{BUILTINS, Form, Unit, UnitRef, is_length};
use crate::vocab::{Keyword, Lexicon, Meaning};
use crate::zones;

/// An operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
    BitAnd,
    BitOr,
    Xor,
    Shl,
    Shr,
    Fact,
    Percent,
    LParen,
    RParen,
    Semi,
    Sup2,
    Sup3,
    Equals,
}

/// `of`, `on`, `off` in percentage phrases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PercentOp {
    Of,
    On,
    Off,
}

/// Which conversion keyword was used; `as` also starts `as a % of`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConvertWord {
    As,
    In,
}

/// A document reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocRef {
    Prev,
    Sum,
    Average,
}

/// A callable function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Func {
    Builtin(Builtin),
    Custom(usize),
}

/// A semantic token.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Tok {
    Num(f64),
    /// A unit following a value or inside a conversion target.
    Unit(Unit),
    /// A currency written before its amount (`$30`, `EUR 30`).
    PrefixCurrency(String),
    Op(Op),
    Convert(ConvertWord),
    Percent(PercentOp),
    What,
    Is,
    /// `as a % of` and its translations in one token.
    AsPercent(PercentOp),
    /// `of what is` and its translations in one token.
    PercentWhat(PercentOp),
    /// `sq`/`square` (2) or `cubic`/`cu`/`cb` (3) before a length unit.
    Power(i8),
    Func(Func),
    Const(f64),
    Var(String),
    Doc(DocRef),
    /// `now`, `time`.
    Now,
    /// A day relative to today: `today` (0), `tomorrow` (1), `yesterday` (-1).
    Day(i64),
    Time(NaiveTime),
    Date(NaiveDate),
    Zone(Tz),
    /// `local`: the local time zone.
    LocalZone,
    Radix(Radix),
    /// Ignored prose.
    Text,
    /// Where ignored prose separated two operands; stops implicit multiplication.
    Gap,
}

impl Tok {
    /// Whether a value can end right before this token's position (so a unit can attach).
    pub(crate) fn ends_value(&self) -> bool {
        matches!(
            self,
            Self::Num(_)
                | Self::Unit(_)
                | Self::Op(Op::RParen | Op::Fact | Op::Percent | Op::Sup2 | Op::Sup3)
                | Self::Var(_)
                | Self::Const(_)
                | Self::Doc(_)
                | Self::Now
                | Self::Day(_)
                | Self::Time(_)
                | Self::Date(_)
        )
    }

    /// Whether a bare unit may follow this token (`in cm`, `/h`, `sq m`).
    fn expects_unit(&self) -> bool {
        matches!(self, Self::Convert(_) | Self::Op(Op::Div) | Self::Power(_))
    }

    /// Whether this token gives a line something to compute.
    pub(crate) fn is_value(&self) -> bool {
        matches!(
            self,
            Self::Num(_)
                | Self::Var(_)
                | Self::Const(_)
                | Self::Doc(_)
                | Self::Now
                | Self::Day(_)
                | Self::Time(_)
                | Self::Date(_)
        )
    }
}

/// A classified token with its byte range in the line and its highlight kind.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Item {
    pub(crate) tok: Tok,
    pub(crate) range: Range<usize>,
    pub(crate) kind: SpanKind,
}

/// What the classifier needs to know about the document and the engine.
pub(crate) struct Context<'a> {
    pub(crate) line: &'a str,
    pub(crate) custom: &'a Registry,
    pub(crate) is_variable: &'a dyn Fn(&str) -> bool,
    /// The current year, for dates written without one.
    pub(crate) year: i32,
    /// The input language's phrases on top of English.
    pub(crate) lexicon: Lexicon,
}

/// Classifies the raw tokens of a line.
pub(crate) fn classify(raws: &[Raw], ctx: &Context<'_>) -> Vec<Item> {
    let money_line = raws.iter().any(|raw| match raw.kind {
        RawKind::Sign(_) => true,
        RawKind::Word => {
            let text = &ctx.line[raw.range.clone()];
            currency::code(text).is_some()
                || currency::name(text).is_some_and(|_| !is_pound(text))
                || names_money(&ctx.lexicon.meanings(text))
        }
        _ => false,
    });
    let mut classifier = Classifier {
        raws,
        ctx,
        out: Vec::with_capacity(raws.len()),
        money_line,
        context: None,
    };
    let mut i = 0;
    while i < raws.len() {
        i += classifier.next(i);
    }
    classifier.out
}

/// Whether a phrase names a currency and nothing else (so `Pfund` does not make a money line).
fn names_money(meanings: &[Meaning]) -> bool {
    meanings.iter().any(|m| matches!(m, Meaning::Currency(_)))
        && !meanings.iter().any(|m| matches!(m, Meaning::Unit { .. }))
}

/// Keywords: words that cannot be variable names or extension function phrases.
const KEYWORDS: &[&str] = &[
    "in",
    "into",
    "to",
    "as",
    "plus",
    "and",
    "with",
    "minus",
    "subtract",
    "without",
    "times",
    "mul",
    "divide",
    "per",
    "mod",
    "xor",
    "of",
    "on",
    "off",
    "prev",
    "sum",
    "total",
    "avg",
    "average",
    "now",
    "time",
    "today",
    "tomorrow",
    "yesterday",
    "noon",
    "midnight",
    "pi",
    "e",
    "sq",
    "square",
    "cubic",
    "cu",
    "cb",
    "percent",
    "what",
    "is",
    "pct",
];

/// Whether `word` is taken by a keyword, scale, format, unit, currency or zone abbreviation.
pub(crate) fn is_reserved(word: &str) -> bool {
    KEYWORDS.contains(&word.to_lowercase().as_str())
        || scale(word).is_some()
        || Radix::from_word(word).is_some()
        || BUILTINS.lookup(word).is_some()
        || powered_length(word).is_some()
        || currency::code(word).is_some()
        || currency::name(word).is_some()
        || zones::abbreviation(word).is_some()
}

/// Whether English gives `word` a meaning of its own.
fn english_meaning(word: &str) -> bool {
    is_reserved(word) || Builtin::from_name(word).is_some()
}

/// Whether `text` (or `text` without a final `.`) is a short English unit symbol such as `m`,
/// `ft`, `kt` or `km`, which a language word may not redefine. Longer symbols and rarely
/// prefixed ones are units nobody writes (`Grad` gigaradians, `Tg` teragrams), so a language
/// word spelt like one keeps its own meaning (German `Grad` degrees, `Tg` days).
fn english_symbol(text: &str) -> bool {
    let bare = text.trim_end_matches('.');
    bare.chars().count() <= 2
        && BUILTINS.symbol(bare).is_some_and(|hit| {
            BUILTINS
                .def(hit.index)
                .prefix
                .is_none_or(|prefix| COMMON_PREFIXES.contains(&prefix))
        })
}

/// SI prefixes common enough that their two-letter symbols (`km`, `kg`, `ms`) beat a language
/// word; rarer ones (`Em` exametres, `Tg` teragrams) give way to it.
const COMMON_PREFIXES: &[&str] = &["k", "M", "G", "c", "m", "µ", "n"];

fn is_pound(text: &str) -> bool {
    matches!(text.to_lowercase().as_str(), "pound" | "pounds")
}

/// Most words in a built-in multi-word name (`Ho Chi Minh City`).
const MAX_WORDS_IN_NAMES: usize = zones::MAX_CITY_WORDS;

const MONTHS: &[(&str, u32)] = &[
    ("january", 1),
    ("jan", 1),
    ("february", 2),
    ("feb", 2),
    ("march", 3),
    ("mar", 3),
    ("april", 4),
    ("apr", 4),
    ("may", 5),
    ("june", 6),
    ("jun", 6),
    ("july", 7),
    ("jul", 7),
    ("august", 8),
    ("aug", 8),
    ("september", 9),
    ("sep", 9),
    ("sept", 9),
    ("october", 10),
    ("oct", 10),
    ("november", 11),
    ("nov", 11),
    ("december", 12),
    ("dec", 12),
];

fn scale(word: &str) -> Option<f64> {
    match word {
        "k" => return Some(1e3),
        "M" => return Some(1e6),
        _ => {}
    }
    Some(match word.to_lowercase().as_str() {
        "thousand" | "thousands" => 1e3,
        "million" | "millions" | "mio" => 1e6,
        "billion" | "billions" | "bn" => 1e9,
        "trillion" | "trillions" => 1e12,
        _ => return None,
    })
}

/// Markers of East Asian dates: `2026年3月12日`, `3月12号`, `2026년 3월 12일`.
const YEAR_MARKERS: &[&str] = &["年", "년"];
const MONTH_MARKERS: &[&str] = &["月", "월"];
const DAY_MARKERS: &[&str] = &["日", "号", "일"];

/// Whether a phrase has other meanings (in the language or in English) to give way to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rivals {
    None,
    Some,
}

/// Whether a phrase is spelt like a short English unit symbol (`m`, `ft`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spelling {
    Own,
    EnglishSymbol,
}

/// A unit a word names, before context decides whether it is one.
struct UnitWord {
    unit: Unit,
    kind: SpanKind,
    /// A currency code written in uppercase, which may also stand before its amount.
    prefix_code: Option<&'static str>,
}

struct Classifier<'a> {
    raws: &'a [Raw],
    ctx: &'a Context<'a>,
    out: Vec<Item>,
    money_line: bool,
    /// Index in `out` of the token context rules look at; articles after `as` are skipped.
    context: Option<usize>,
}

impl Classifier<'_> {
    fn text(&self, i: usize) -> &str {
        &self.ctx.line[self.raws[i].range.clone()]
    }

    fn prev(&self) -> Option<&Tok> {
        self.context.map(|index| &self.out[index].tok)
    }

    fn prev_ends_value(&self) -> bool {
        self.prev().is_some_and(Tok::ends_value)
    }

    /// Whether the previous token can be a percentage (`20% of`, `v2 of`, `(5%) of`), so `of`,
    /// `on` and `off` are percentage keywords rather than prose (`2 cups of flour`).
    fn prev_is_share(&self) -> bool {
        match self.prev() {
            Some(Tok::Op(Op::Percent | Op::RParen) | Tok::Var(_) | Tok::Doc(_)) => true,
            Some(Tok::Unit(unit)) => unit.is_percent(),
            _ => false,
        }
    }

    fn prev_expects_unit(&self) -> bool {
        self.prev().is_some_and(Tok::expects_unit)
    }

    /// Whether a unit may be read here: after a value or where one is expected.
    fn unit_context(&self) -> bool {
        self.prev_ends_value() || self.prev_expects_unit()
    }

    /// Whether only whitespace separates raw `i` from raw `i + 1`.
    fn spaced(&self, i: usize) -> bool {
        match (self.raws.get(i), self.raws.get(i + 1)) {
            (Some(a), Some(b)) => self.ctx.line[a.range.end..b.range.start]
                .chars()
                .all(char::is_whitespace),
            _ => false,
        }
    }

    /// Whether raw `i` and `i + 1` touch.
    fn touching(&self, i: usize) -> bool {
        match (self.raws.get(i), self.raws.get(i + 1)) {
            (Some(a), Some(b)) => a.range.end == b.range.start,
            _ => false,
        }
    }

    fn number_at(&self, i: usize) -> Option<f64> {
        match self.raws.get(i)?.kind {
            RawKind::Number(n) => Some(n),
            _ => None,
        }
    }

    fn word_at(&self, i: usize) -> Option<&str> {
        (self.raws.get(i)?.kind == RawKind::Word).then(|| self.text(i))
    }

    fn is_dot(&self, i: usize) -> bool {
        self.raws
            .get(i)
            .is_some_and(|r| r.kind == RawKind::Other && self.text(i) == ".")
    }

    fn push(&mut self, tok: Tok, range: Range<usize>, kind: SpanKind) {
        self.out.push(Item { tok, range, kind });
        self.context = Some(self.out.len() - 1);
    }

    fn span(&self, first: usize, last: usize) -> Range<usize> {
        self.raws[first].range.start..self.raws[last].range.end
    }

    /// The text of raws `i..=j` with whitespace collapsed, when only whitespace separates them.
    fn phrase_text(&self, i: usize, j: usize) -> Option<String> {
        if j >= self.raws.len() || !(i..j).all(|k| self.spaced(k)) {
            return None;
        }
        Some(custom::normalize(&self.ctx.line[self.span(i, j)]))
    }

    /// How many raws after `i` a language phrase starting at `i` can reach.
    fn language_window(&self, i: usize) -> usize {
        self.ctx.lexicon.window(self.text(i))
    }

    /// The longest language phrase starting at raw `i`: its meanings and last raw.
    fn language_phrase_at(&self, i: usize) -> Option<(Vec<Meaning>, usize)> {
        let window = self.language_window(i);
        if window == 0 {
            return None;
        }
        let last = (i + window).min(self.raws.len()) - 1;
        (i..=last).rev().find_map(|j| {
            let meanings = self.ctx.lexicon.meanings(&self.phrase_text(i, j)?);
            (!meanings.is_empty()).then_some((meanings, j))
        })
    }

    /// Classifies the token(s) starting at raw `i`; returns how many raws were used.
    fn next(&mut self, i: usize) -> usize {
        let range = self.raws[i].range.clone();
        match &self.raws[i].kind {
            RawKind::Number(n) => self.number(i, *n),
            RawKind::Word => self.word(i),
            RawKind::Sign(code) => {
                if let Some(used) = self.language_phrase(i, 2) {
                    return used;
                }
                self.sign(i, code);
                1
            }
            RawKind::Sym(sym) => {
                self.push(Tok::Op(symbol(*sym)), range, SpanKind::Operator);
                1
            }
            RawKind::Time(time) => {
                let time = *time;
                let used = if self.oclock_follows(i) { 2 } else { 1 };
                self.push(Tok::Time(time), self.span(i, i + used - 1), SpanKind::DateTime);
                used
            }
            RawKind::Date(date) => {
                self.push(Tok::Date(*date), range, SpanKind::DateTime);
                1
            }
            RawKind::Other => {
                if let Some(used) = self.language_phrase(i, 1) {
                    return used;
                }
                self.push(Tok::Text, range, SpanKind::Text);
                1
            }
        }
    }

    /// Classifies a language phrase of at least `min_raws` raws starting at `i`, trying the
    /// longest first; returns how many raws it used.
    fn language_phrase(&mut self, i: usize, min_raws: usize) -> Option<usize> {
        let window = self.language_window(i);
        if window < min_raws {
            return None;
        }
        let last = (i + window).min(self.raws.len()) - 1;
        for j in (i + min_raws - 1..=last).rev() {
            if let Some(text) = self.phrase_text(i, j)
                && self.language(i, j, &text)
            {
                return Some(j - i + 1);
            }
        }
        None
    }

    fn number(&mut self, i: usize, n: f64) -> usize {
        if let Some((date, used)) = self.east_asian_date(i, n).or_else(|| self.day_month_year(i, n)) {
            self.push(Tok::Date(date), self.span(i, i + used - 1), SpanKind::DateTime);
            return used;
        }
        if let Some(time) = self.oclock(i) {
            self.push(Tok::Time(time), self.span(i, i + 1), SpanKind::DateTime);
            return 2;
        }
        self.push(Tok::Num(n), self.raws[i].range.clone(), SpanKind::Number);
        1
    }

    /// Whether the phrase right after raw `i` is an o'clock word (`Uhr`).
    fn oclock_follows(&self, i: usize) -> bool {
        self.spaced(i)
            && self.raws.get(i + 1).is_some_and(|r| r.kind == RawKind::Word)
            && self.language_phrase_at(i + 1).is_some_and(|(meanings, last)| {
                last == i + 1 && meanings.contains(&Meaning::Keyword(Keyword::OClock))
            })
    }

    /// `14 Uhr`, `14.30 Uhr`: an hour (and minutes) followed by an o'clock word.
    fn oclock(&self, i: usize) -> Option<NaiveTime> {
        if !self.oclock_follows(i) {
            return None;
        }
        let text = self.text(i);
        let (hour, minute) = match text.split_once(['.', ',']) {
            Some((hour, minute)) if minute.len() == 2 => (hour.parse::<u32>().ok()?, minute.parse().ok()?),
            Some(_) => return None,
            None => (text.parse::<u32>().ok()?, 0),
        };
        NaiveTime::from_hms_opt(if hour == 24 { 0 } else { hour }, minute, 0)
    }

    /// `12 March [2026]`, `12. März [2026]`.
    fn day_month_year(&self, i: usize, day: f64) -> Option<(NaiveDate, usize)> {
        let day = small_int(day, 31)?;
        let mut month_at = i + 1;
        if self.is_dot(month_at) && self.touching(i) {
            month_at += 1;
        }
        if !self.spaced(month_at - 1) {
            return None;
        }
        let (month, month_len) = self.month_at(month_at)?;
        let after = month_at + month_len;
        let (year, end) = match self.year_at(after).filter(|_| self.spaced(after - 1)) {
            Some(year) => (year, after + 1),
            None => (self.ctx.year, after),
        };
        Some((NaiveDate::from_ymd_opt(year, month, day)?, end - i))
    }

    /// A month name at raw `k`, with a touching `.` after an abbreviation (`Okt.`).
    fn month_at(&self, k: usize) -> Option<(u32, usize)> {
        let month = self.month(self.word_at(k)?)?;
        let dot = usize::from(self.is_dot(k + 1) && self.touching(k));
        Some((month, 1 + dot))
    }

    fn month(&self, word: &str) -> Option<u32> {
        let lower = word.to_lowercase();
        MONTHS
            .iter()
            .find(|(name, _)| *name == lower)
            .map(|(_, m)| *m)
            .or_else(|| {
                self.ctx.lexicon.meanings(word).iter().find_map(|m| match m {
                    Meaning::Month(month) => Some(*month),
                    _ => None,
                })
            })
    }

    /// `March 12[,] [2026]`.
    fn month_day_year(&self, i: usize, month: u32) -> Option<(NaiveDate, usize)> {
        if !self.spaced(i) {
            return None;
        }
        let day = small_int(self.number_at(i + 1)?, 31)?;
        let comma = usize::from(
            self.raws
                .get(i + 2)
                .is_some_and(|r| r.kind == RawKind::Other && self.text(i + 2) == ","),
        );
        let year_at = i + 2 + comma;
        let (year, used) = match self
            .year_at(year_at)
            .filter(|_| self.spaced(year_at - 1) || comma == 1)
        {
            Some(year) => (year, 3 + comma),
            None => (self.ctx.year, 2),
        };
        Some((NaiveDate::from_ymd_opt(year, month, day)?, used))
    }

    /// `2026年3月12日`, `3月12日`, `2026년 3월 12일`.
    fn east_asian_date(&self, i: usize, first: f64) -> Option<(NaiveDate, usize)> {
        let marker = |k: usize, markers: &[&str]| {
            self.spaced(k - 1) && self.word_at(k).is_some_and(|w| markers.contains(&w))
        };
        let number = |k: usize| self.number_at(k).filter(|_| self.spaced(k - 1));
        let (year, month_at) = if marker(i + 1, YEAR_MARKERS) {
            let year = crate::num::to_i64(first).and_then(|y| i32::try_from(y).ok())?;
            (year, i + 2)
        } else {
            (self.ctx.year, i)
        };
        let month = small_int(if month_at == i { first } else { number(month_at)? }, 12)?;
        if !marker(month_at + 1, MONTH_MARKERS) {
            return None;
        }
        let day = small_int(number(month_at + 2)?, 31)?;
        if !marker(month_at + 3, DAY_MARKERS) {
            return None;
        }
        Some((NaiveDate::from_ymd_opt(year, month, day)?, month_at + 4 - i))
    }

    fn year_at(&self, i: usize) -> Option<i32> {
        let n = self.number_at(i)?;
        let digits = self.text(i).len() == 4;
        (digits && n.fract() == 0.0)
            .then(|| crate::num::to_i64(n).and_then(|y| i32::try_from(y).ok()))
            .flatten()
    }

    fn sign(&mut self, i: usize, code: &str) {
        let range = self.raws[i].range.clone();
        let before_amount = self.number_at(i + 1).is_some()
            || (self
                .raws
                .get(i + 1)
                .is_some_and(|r| r.kind == RawKind::Sym(Sym::Minus))
                && self.number_at(i + 2).is_some());
        let starts_amount = self.touching(i) && (i == 0 || !self.touching(i - 1));
        let tok = if before_amount && (!self.prev_ends_value() || starts_amount) {
            Tok::PrefixCurrency(code.to_owned())
        } else if self.unit_context() {
            Tok::Unit(Unit::simple(UnitRef::Currency(code.to_owned()), Form::Default))
        } else {
            Tok::Text
        };
        self.push(tok, range, SpanKind::Currency);
    }

    fn word(&mut self, i: usize) -> usize {
        if let Some(used) = self.iana_zone(i).or_else(|| self.utc_offset(i)) {
            return used;
        }
        if let Some(m) = self.month(self.text(i))
            && !(self.ctx.is_variable)(self.text(i))
            && let Some((date, used)) = self.month_day_year(i, m)
        {
            self.push(Tok::Date(date), self.span(i, i + used - 1), SpanKind::DateTime);
            return used;
        }
        let window = MAX_WORDS_IN_NAMES
            .max(self.ctx.custom.max_phrase_tokens)
            .max(self.language_window(i));
        let last = (i + window).min(self.raws.len()) - 1;
        for j in (i + 1..=last).rev() {
            if self.phrase(i, j) {
                return j - i + 1;
            }
        }
        self.single_word(i);
        1
    }

    /// `Europe/Berlin`, `America/Argentina/Buenos_Aires`.
    fn iana_zone(&mut self, i: usize) -> Option<usize> {
        let mut j = i;
        while self
            .raws
            .get(j + 1)
            .is_some_and(|r| r.kind == RawKind::Sym(Sym::Slash))
            && self.word_at(j + 2).is_some()
            && self.touching(j)
            && self.touching(j + 1)
        {
            j += 2;
        }
        if j == i {
            return None;
        }
        if let Some(offset_end) = self.offset_suffix(j) {
            j = offset_end;
        }
        let range = self.span(i, j);
        let tz = zones::iana(&self.ctx.line[range.clone()])?;
        self.push(Tok::Zone(tz), range, SpanKind::DateTime);
        Some(j - i + 1)
    }

    /// Tries raws `i..=j` as one multi-token phrase.
    fn phrase(&mut self, i: usize, j: usize) -> bool {
        let Some(text) = self.phrase_text(i, j) else {
            return false;
        };
        let unit_context = self.unit_context();
        if let Some(index) = self.ctx.custom.unit(&text)
            && unit_context
            && self.english_phrase_like(i, j)
        {
            self.push(
                Tok::Unit(Unit::simple(UnitRef::Custom(index), Form::Default)),
                self.span(i, j),
                SpanKind::Unit,
            );
            return true;
        }
        if (self.ctx.is_variable)(&text) && self.ideographic(i, j) {
            self.push(Tok::Var(text), self.span(i, j), SpanKind::Variable);
            return true;
        }
        if self.language(i, j, &text) {
            return true;
        }
        if !self.english_phrase_like(i, j) {
            return false;
        }
        let range = self.span(i, j);
        let raws = &self.raws[i..=j];
        let words_only = raws.iter().all(|r| r.kind == RawKind::Word);
        let words_and_dots = raws
            .iter()
            .enumerate()
            .all(|(k, r)| r.kind == RawKind::Word || (r.kind == RawKind::Other && self.text(i + k) == "."));
        if words_and_dots
            && let Some(tz) = zones::city(&custom::normalize(&text.replace('.', " ")))
            && self.zone_context(j)
        {
            self.push(Tok::Zone(tz), range, SpanKind::DateTime);
            return true;
        }
        if !words_only {
            return false;
        }
        if let Some(index) = self.ctx.custom.function(&text) {
            self.push(Tok::Func(Func::Custom(index)), range, SpanKind::Function);
            return true;
        }
        let lower = text.to_lowercase();
        if let Some(op) = match lower.as_str() {
            "multiplied by" => Some(Op::Mul),
            "divided by" | "divide by" => Some(Op::Div),
            _ => None,
        } {
            self.push(Tok::Op(op), range, SpanKind::Keyword);
            return true;
        }
        if unit_context && let Some(word) = self.unit_word(&text) {
            self.push(Tok::Unit(word.unit), range, word.kind);
            return true;
        }
        false
    }

    /// Whether raws `i..=j` look like an English multi-word name: words, punctuation and
    /// symbols, ending in a word or a dot.
    fn english_phrase_like(&self, i: usize, j: usize) -> bool {
        let phrase_like = self.raws[i..=j]
            .iter()
            .all(|r| matches!(r.kind, RawKind::Word | RawKind::Other | RawKind::Sym(_)));
        let ends_well = self.raws[j].kind == RawKind::Word || self.text(j) == ".";
        phrase_like && ends_well
    }

    /// Whether raws `i..=j` are touching Chinese or Japanese characters (a variable name such
    /// as `价格` is several raws).
    fn ideographic(&self, i: usize, j: usize) -> bool {
        (i..j).all(|k| self.touching(k)) && self.raws[i..=j].iter().all(|r| r.kind == RawKind::Word)
    }

    /// Classifies raws `i..=j` (whose text is `text`) by the input language, if one of the
    /// phrase's meanings applies here.
    fn language(&mut self, i: usize, j: usize, text: &str) -> bool {
        let mut meanings = self.ctx.lexicon.meanings(text);
        if meanings.is_empty() {
            return false;
        }
        let rivals = if meanings.len() > 1 || (i == j && english_meaning(text)) {
            Rivals::Some
        } else {
            Rivals::None
        };
        let spelling = if english_symbol(text) {
            Spelling::EnglishSymbol
        } else {
            Spelling::Own
        };
        // Percentage words first after a percentage; money words first on money lines.
        let rank = |m: &Meaning| match m {
            Meaning::Keyword(Keyword::Of | Keyword::On | Keyword::Off) | Meaning::PercentWhat(_) => 0,
            Meaning::Currency(_) if self.money_line => 1,
            _ => 2,
        };
        meanings.sort_by_key(rank);
        meanings
            .into_iter()
            .any(|meaning| self.apply(meaning, i, j, rivals, spelling))
    }

    /// Classifies raws `i..=j` as `meaning` when it applies in this context.
    fn apply(&mut self, meaning: Meaning, i: usize, j: usize, rivals: Rivals, spelling: Spelling) -> bool {
        let symbol = spelling == Spelling::EnglishSymbol;
        let item = match meaning {
            Meaning::Keyword(keyword) => self.language_keyword(keyword, j, rivals),
            Meaning::Assign | Meaning::Month(_) => None,
            Meaning::AsPercent(op) => self
                .prev_ends_value()
                .then_some((Tok::AsPercent(op), SpanKind::Keyword)),
            Meaning::PercentWhat(op) => self
                .prev_is_share()
                .then_some((Tok::PercentWhat(op), SpanKind::Keyword)),
            Meaning::Day(offset) => Some((Tok::Day(offset), SpanKind::DateTime)),
            Meaning::Scale(factor) => return !symbol && self.scale(i, j, factor),
            Meaning::Function(builtin) => Some((Tok::Func(Func::Builtin(builtin)), SpanKind::Function)),
            Meaning::Radix(radix) => {
                matches!(self.prev(), Some(Tok::Convert(_))).then_some((Tok::Radix(radix), SpanKind::Keyword))
            }
            Meaning::Unit { index, power } => (!symbol && self.unit_context()).then(|| {
                (
                    Tok::Unit(Unit::power_of(UnitRef::Builtin(index), power, Form::Default)),
                    SpanKind::Unit,
                )
            }),
            Meaning::Currency(code) => {
                if symbol {
                    None
                } else if self.unit_context() {
                    let unit = Unit::simple(UnitRef::Currency(code.to_owned()), Form::Default);
                    Some((Tok::Unit(unit), SpanKind::Currency))
                } else {
                    let sign_like = self.raws[i..=j]
                        .iter()
                        .any(|r| matches!(r.kind, RawKind::Sign(_) | RawKind::Other));
                    (sign_like && self.number_at(j + 1).is_some())
                        .then(|| (Tok::PrefixCurrency(code.to_owned()), SpanKind::Currency))
                }
            }
            Meaning::Zone(tz) => self
                .zone_context(j)
                .then_some((Tok::Zone(tz), SpanKind::DateTime)),
        };
        let Some((tok, kind)) = item else {
            return false;
        };
        self.push(tok, self.span(i, j), kind);
        true
    }

    fn language_keyword(&self, keyword: Keyword, j: usize, rivals: Rivals) -> Option<(Tok, SpanKind)> {
        let contested = rivals == Rivals::Some;
        let key = |tok| Some((tok, SpanKind::Keyword));
        let operator = |op| {
            (self.operand_follows(j) && (!contested || self.prev_ends_value()))
                .then_some((Tok::Op(op), SpanKind::Keyword))
        };
        match keyword {
            Keyword::Plus => operator(Op::Add),
            Keyword::Minus => operator(Op::Sub),
            Keyword::Times => operator(Op::Mul),
            Keyword::Divide => operator(Op::Div),
            Keyword::Modulo => operator(Op::Mod),
            Keyword::Per => {
                (!contested || self.prev_ends_value()).then_some((Tok::Op(Op::Div), SpanKind::Keyword))
            }
            Keyword::Convert => (!contested || self.target_follows(j))
                .then_some((Tok::Convert(ConvertWord::In), SpanKind::Keyword)),
            Keyword::Percent => self.unit_context().then(|| {
                (
                    Tok::Unit(Unit::simple(UnitRef::Percent, Form::Default)),
                    SpanKind::Unit,
                )
            }),
            Keyword::Of if self.prev_is_share() => key(Tok::Percent(PercentOp::Of)),
            Keyword::On if self.prev_is_share() => key(Tok::Percent(PercentOp::On)),
            Keyword::Off if self.prev_is_share() => key(Tok::Percent(PercentOp::Off)),
            Keyword::Of | Keyword::On | Keyword::Off => None,
            Keyword::Prev => Some((Tok::Doc(DocRef::Prev), SpanKind::Variable)),
            Keyword::Sum => Some((Tok::Doc(DocRef::Sum), SpanKind::Variable)),
            Keyword::Average => Some((Tok::Doc(DocRef::Average), SpanKind::Variable)),
            Keyword::Now | Keyword::OClock => Some((Tok::Now, SpanKind::DateTime)),
            Keyword::Local => self
                .zone_context(j)
                .then_some((Tok::LocalZone, SpanKind::DateTime)),
            Keyword::Noon => Some((Tok::Time(NaiveTime::from_hms_opt(12, 0, 0)?), SpanKind::DateTime)),
            Keyword::Midnight => Some((Tok::Time(NaiveTime::MIN), SpanKind::DateTime)),
        }
    }

    /// Applies a scale word to the number right before raw `i` (`5 Millionen`).
    fn scale(&mut self, i: usize, j: usize, factor: f64) -> bool {
        if i == 0 || self.number_at(i - 1).is_none() {
            return false;
        }
        let end = self.raws[j].range.end;
        let before = self.raws[i - 1].range.end;
        match self.out.last_mut() {
            Some(Item {
                tok: Tok::Num(n),
                range,
                ..
            }) if range.end == before => {
                *n *= factor;
                range.end = end;
                true
            }
            _ => false,
        }
    }

    /// Whether an operand starts right after raw `j` (so a word between is an operator).
    fn operand_follows(&self, j: usize) -> bool {
        let Some(next) = self.raws.get(j + 1) else {
            return false;
        };
        match &next.kind {
            RawKind::Number(_) | RawKind::Sign(_) | RawKind::Date(_) | RawKind::Time(_) => true,
            RawKind::Sym(sym) => matches!(sym, Sym::LParen | Sym::Minus | Sym::Plus),
            RawKind::Other => false,
            RawKind::Word => {
                let text = self.text(j + 1);
                let english = matches!(
                    text.to_lowercase().as_str(),
                    "pi" | "π"
                        | "e"
                        | "prev"
                        | "sum"
                        | "total"
                        | "avg"
                        | "average"
                        | "now"
                        | "time"
                        | "today"
                        | "tomorrow"
                        | "yesterday"
                        | "noon"
                        | "midnight"
                );
                let language = self.language_phrase_at(j + 1).is_some_and(|(meanings, _)| {
                    meanings.iter().any(|m| {
                        matches!(
                            m,
                            Meaning::Day(_)
                                | Meaning::Function(_)
                                | Meaning::Keyword(
                                    Keyword::Prev
                                        | Keyword::Sum
                                        | Keyword::Average
                                        | Keyword::Now
                                        | Keyword::Noon
                                        | Keyword::Midnight
                                )
                        )
                    })
                });
                english
                    || language
                    || (self.ctx.is_variable)(text)
                    || Builtin::from_name(text).is_some()
                    || self.ctx.custom.function(text).is_some()
                    || (text.len() == 3 && currency::code(text).is_some() && self.number_at(j + 2).is_some())
            }
        }
    }

    /// Whether a conversion target (unit, currency, zone, format, `%`) follows raw `j`.
    fn target_follows(&self, j: usize) -> bool {
        let k = j + 1;
        let Some(next) = self.raws.get(k) else {
            return false;
        };
        match &next.kind {
            RawKind::Sign(_) | RawKind::Sym(Sym::Percent) => true,
            RawKind::Word | RawKind::Other => {
                let text = self.text(k);
                let english = self.unit_word(text).is_some()
                    || Radix::from_word(text).is_some()
                    || zones::city(text).is_some()
                    || zones::abbreviation(text).is_some();
                english
                    || self.language_phrase_at(k).is_some_and(|(meanings, _)| {
                        meanings.iter().any(|m| {
                            matches!(
                                m,
                                Meaning::Unit { .. }
                                    | Meaning::Currency(_)
                                    | Meaning::Zone(_)
                                    | Meaning::Radix(_)
                                    | Meaning::Keyword(Keyword::Local | Keyword::Percent)
                            )
                        })
                    })
            }
            _ => false,
        }
    }

    /// `+2` or `-5` touching raw `j` (for `UTC+2`, `Etc/GMT+5`): the index of the number.
    fn offset_suffix(&self, j: usize) -> Option<usize> {
        let sign = matches!(self.raws.get(j + 1)?.kind, RawKind::Sym(Sym::Plus | Sym::Minus));
        let hours = self.number_at(j + 2)?;
        let whole = hours.fract() == 0.0 && (0.0..=14.0).contains(&hours);
        (sign && whole && self.touching(j) && self.touching(j + 1)).then_some(j + 2)
    }

    /// `UTC+2`, `GMT-5`: a fixed offset, used like a zone.
    fn utc_offset(&mut self, i: usize) -> Option<usize> {
        if !matches!(self.text(i), "UTC" | "GMT") {
            return None;
        }
        let end = self.offset_suffix(i)?;
        let east = self.raws[i + 1].kind == RawKind::Sym(Sym::Plus);
        let hours = crate::num::to_i64(self.number_at(end)?)?;
        // IANA's Etc zones count the other way round: UTC+2 is Etc/GMT-2.
        let name = match (hours, east) {
            (0, _) => "Etc/GMT".to_owned(),
            (_, true) => format!("Etc/GMT-{hours}"),
            (_, false) => format!("Etc/GMT+{hours}"),
        };
        let tz = zones::iana(&name)?;
        if !self.zone_context(end) {
            return None;
        }
        self.push(Tok::Zone(tz), self.span(i, end), SpanKind::DateTime);
        Some(end - i + 1)
    }

    /// Whether a zone name ending at raw `last` is used as one: `Berlin time`, `in Berlin`,
    /// `2:30 pm HKT`.
    fn zone_context(&self, last: usize) -> bool {
        let before = matches!(self.prev(), Some(Tok::Convert(_) | Tok::Time(_) | Tok::Date(_)));
        let after = self.spaced(last)
            && self.word_at(last + 1).is_some_and(|w| {
                matches!(w.to_lowercase().as_str(), "time" | "now")
                    || self.language_phrase_at(last + 1).is_some_and(|(meanings, _)| {
                        meanings.contains(&Meaning::Keyword(Keyword::Now))
                            || meanings.contains(&Meaning::Keyword(Keyword::OClock))
                    })
            });
        before || after
    }

    fn keyword(&self, i: usize, lower: &str) -> Option<(Tok, SpanKind)> {
        let keyword = |tok| Some((tok, SpanKind::Keyword));
        match lower {
            "in" | "into" | "to" => keyword(Tok::Convert(ConvertWord::In)),
            "as" => keyword(Tok::Convert(ConvertWord::As)),
            "plus" | "and" | "with" => keyword(Tok::Op(Op::Add)),
            "minus" | "subtract" | "without" => keyword(Tok::Op(Op::Sub)),
            "times" | "mul" => keyword(Tok::Op(Op::Mul)),
            "divide" | "per" => keyword(Tok::Op(Op::Div)),
            "mod" => keyword(Tok::Op(Op::Mod)),
            "xor" => keyword(Tok::Op(Op::Xor)),
            "of" if self.prev_is_share() => keyword(Tok::Percent(PercentOp::Of)),
            "on" if self.prev_is_share() => keyword(Tok::Percent(PercentOp::On)),
            "off" if self.prev_is_share() => keyword(Tok::Percent(PercentOp::Off)),
            "what" if matches!(self.prev(), Some(Tok::Percent(_))) => keyword(Tok::What),
            "is" if matches!(self.prev(), Some(Tok::What)) => keyword(Tok::Is),
            "sq" | "square" if self.length_unit_follows(i) => keyword(Tok::Power(2)),
            "cubic" | "cu" | "cb" if self.length_unit_follows(i) => keyword(Tok::Power(3)),
            "x" if self.prev_ends_value() && self.number_at(i + 1).is_some() => keyword(Tok::Op(Op::Mul)),
            "prev" => Some((Tok::Doc(DocRef::Prev), SpanKind::Variable)),
            "sum" | "total" => Some((Tok::Doc(DocRef::Sum), SpanKind::Variable)),
            "avg" | "average" => Some((Tok::Doc(DocRef::Average), SpanKind::Variable)),
            "now" | "time" => Some((Tok::Now, SpanKind::DateTime)),
            "today" => Some((Tok::Day(0), SpanKind::DateTime)),
            "tomorrow" => Some((Tok::Day(1), SpanKind::DateTime)),
            "yesterday" => Some((Tok::Day(-1), SpanKind::DateTime)),
            "noon" => Some((Tok::Time(NaiveTime::from_hms_opt(12, 0, 0)?), SpanKind::DateTime)),
            "midnight" => Some((Tok::Time(NaiveTime::MIN), SpanKind::DateTime)),
            "pi" | "π" => Some((Tok::Const(PI), SpanKind::Constant)),
            _ => None,
        }
    }

    fn length_unit_follows(&self, i: usize) -> bool {
        self.spaced(i)
            && self.word_at(i + 1).is_some_and(|w| {
                BUILTINS.lookup(w).is_some_and(|hit| is_length(hit.index)) || w.to_lowercase() == "inches"
            })
    }

    fn single_word(&mut self, i: usize) {
        let range = self.raws[i].range.clone();
        let text = self.text(i).to_owned();
        let lower = text.to_lowercase();
        if let Some(tz) = zones::abbreviation(&text).or_else(|| zones::city(&text))
            && self.zone_context(i)
        {
            self.push(Tok::Zone(tz), range, SpanKind::DateTime);
            return;
        }
        if matches!(lower.as_str(), "a" | "an") && self.prev() == Some(&Tok::Convert(ConvertWord::As)) {
            // `as a % of`: the article is prose and leaves the context alone.
            self.out.push(Item {
                tok: Tok::Text,
                range,
                kind: SpanKind::Text,
            });
            return;
        }
        if (self.ctx.is_variable)(&text) {
            self.push(Tok::Var(text), range, SpanKind::Variable);
            return;
        }
        if self.ctx.custom.unit(&text).is_none() && self.language(i, i, &text) {
            return;
        }
        if let Some((tok, kind)) = self.keyword(i, &lower) {
            self.push(tok, range, kind);
            return;
        }
        if text == "e" {
            self.push(Tok::Const(E), range, SpanKind::Constant);
            return;
        }
        if let Some(func) = self.ctx.custom.function(&text).map(Func::Custom).or_else(|| {
            Builtin::from_name(&text)
                .filter(|_| self.ctx.custom.unit(&text).is_none())
                .map(Func::Builtin)
        }) {
            self.push(Tok::Func(func), range, SpanKind::Function);
            return;
        }
        if matches!(self.prev(), Some(Tok::Convert(_)))
            && let Some(radix) = Radix::from_word(&text)
        {
            self.push(Tok::Radix(radix), range, SpanKind::Keyword);
            return;
        }
        if let Some(factor) = scale(&text)
            && self.scale(i, i, factor)
        {
            return;
        }
        self.unit_or_text(i, &text, range);
    }

    fn unit_or_text(&mut self, i: usize, text: &str, range: Range<usize>) {
        let Some(word) = self.unit_word(text) else {
            self.push(Tok::Text, range, SpanKind::Text);
            return;
        };
        if self.unit_context() {
            self.push(Tok::Unit(word.unit), range, word.kind);
        } else if let Some(code) = word.prefix_code
            && self.number_at(i + 1).is_some()
        {
            self.push(Tok::PrefixCurrency(code.to_owned()), range, SpanKind::Currency);
        } else {
            self.push(Tok::Text, range, SpanKind::Text);
        }
    }

    /// The unit an English word or phrase (or an extension unit) names, if any.
    fn unit_word(&self, text: &str) -> Option<UnitWord> {
        let currency_unit = |code: &str| Unit::simple(UnitRef::Currency(code.to_owned()), Form::Default);
        let uppercase = text.len() >= 2 && text.chars().all(|c| c.is_ascii_uppercase());
        if let Some(index) = self.ctx.custom.unit(text) {
            return Some(UnitWord {
                unit: Unit::simple(UnitRef::Custom(index), Form::Default),
                kind: SpanKind::Unit,
                prefix_code: None,
            });
        }
        if uppercase && let Some(code) = currency::code(text) {
            return Some(UnitWord {
                unit: currency_unit(code),
                kind: SpanKind::Currency,
                prefix_code: Some(code),
            });
        }
        let code = currency::code(text);
        if is_pound(text) && self.money_line {
            return Some(UnitWord {
                unit: currency_unit("GBP"),
                kind: SpanKind::Currency,
                prefix_code: None,
            });
        }
        if let Some(hit) = BUILTINS.lookup(text) {
            return Some(UnitWord {
                unit: Unit::simple(UnitRef::Builtin(hit.index), hit.form),
                kind: SpanKind::Unit,
                prefix_code: None,
            });
        }
        if let Some(code) = code {
            return Some(UnitWord {
                unit: currency_unit(code),
                kind: SpanKind::Currency,
                prefix_code: Some(code),
            });
        }
        if let Some(code) = currency::name(text) {
            return Some(UnitWord {
                unit: currency_unit(code),
                kind: SpanKind::Currency,
                prefix_code: None,
            });
        }
        if matches!(text.to_lowercase().as_str(), "percent" | "pct") {
            return Some(UnitWord {
                unit: Unit::simple(UnitRef::Percent, Form::Default),
                kind: SpanKind::Unit,
                prefix_code: None,
            });
        }
        powered_length(text).map(|unit| UnitWord {
            unit,
            kind: SpanKind::Unit,
            prefix_code: None,
        })
    }
}

/// `sqm`, `qm`, `sqft`, `cbm`, `km2`, `cm3`: a length unit with a power folded into the word.
fn powered_length(text: &str) -> Option<Unit> {
    let length = |stem: &str| BUILTINS.lookup(stem).filter(|hit| is_length(hit.index));
    let (stem, power) = if text == "qm" {
        ("m", 2)
    } else if let Some(stem) = text.strip_prefix("sq").or_else(|| text.strip_suffix('2')) {
        (stem, 2)
    } else {
        let stem = text
            .strip_prefix("cb")
            .or_else(|| text.strip_prefix("cu"))
            .or_else(|| text.strip_suffix('3'))?;
        (stem, 3)
    };
    let hit = length(stem)?;
    Some(Unit::power_of(UnitRef::Builtin(hit.index), power, hit.form))
}

fn small_int(n: f64, max: u32) -> Option<u32> {
    let int = crate::num::to_i64(n).filter(|_| n.fract() == 0.0)?;
    u32::try_from(int).ok().filter(|v| (1..=max).contains(v))
}

fn symbol(sym: Sym) -> Op {
    match sym {
        Sym::Plus => Op::Add,
        Sym::Minus => Op::Sub,
        Sym::Star => Op::Mul,
        Sym::Slash => Op::Div,
        Sym::Caret => Op::Pow,
        Sym::Amp => Op::BitAnd,
        Sym::Pipe => Op::BitOr,
        Sym::Shl => Op::Shl,
        Sym::Shr => Op::Shr,
        Sym::Bang => Op::Fact,
        Sym::Percent => Op::Percent,
        Sym::LParen => Op::LParen,
        Sym::RParen => Op::RParen,
        Sym::Semicolon => Op::Semi,
        Sym::Equals => Op::Equals,
        Sym::Sup2 => Op::Sup2,
        Sym::Sup3 => Op::Sup3,
    }
}
