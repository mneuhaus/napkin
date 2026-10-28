//! Turns raw tokens into semantic tokens, deciding what each word means from its context, and
//! assigns every token its highlight kind.

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
    /// `sq`/`square` (2) or `cubic`/`cu`/`cb` (3) before a length unit.
    Power(i8),
    Func(Func),
    Const(f64),
    Var(String),
    Doc(DocRef),
    /// `now`, `time`.
    Now,
    /// `today` (0), `tomorrow` (1), `yesterday` (-1).
    Day(i64),
    Time(NaiveTime),
    Date(NaiveDate),
    Zone(Tz),
    Radix(Radix),
    /// Ignored prose.
    Text,
}

impl Tok {
    /// Whether a value can end right before this token's position (so a unit can attach).
    fn ends_value(&self) -> bool {
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
}

/// Classifies the raw tokens of a line.
pub(crate) fn classify(raws: &[Raw], ctx: &Context<'_>) -> Vec<Item> {
    let money_line = raws.iter().any(|raw| match raw.kind {
        RawKind::Sign(_) => true,
        RawKind::Word => {
            let text = &ctx.line[raw.range.clone()];
            currency::code(text).is_some() || currency::name(text).is_some_and(|_| !is_pound(text))
        }
        _ => false,
    });
    let mut classifier = Classifier {
        raws,
        ctx,
        out: Vec::with_capacity(raws.len()),
        money_line,
    };
    let mut i = 0;
    while i < raws.len() {
        i += classifier.next(i);
    }
    classifier.out
}

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

fn month(word: &str) -> Option<u32> {
    let lower = word.to_lowercase();
    MONTHS.iter().find(|(name, _)| *name == lower).map(|(_, m)| *m)
}

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
}

impl Classifier<'_> {
    fn text(&self, i: usize) -> &str {
        &self.ctx.line[self.raws[i].range.clone()]
    }

    fn prev(&self) -> Option<&Tok> {
        self.out.last().map(|item| &item.tok)
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

    fn push(&mut self, tok: Tok, range: Range<usize>, kind: SpanKind) {
        self.out.push(Item { tok, range, kind });
    }

    fn span(&self, first: usize, last: usize) -> Range<usize> {
        self.raws[first].range.start..self.raws[last].range.end
    }

    /// Classifies the token(s) starting at raw `i`; returns how many raws were used.
    fn next(&mut self, i: usize) -> usize {
        let range = self.raws[i].range.clone();
        match &self.raws[i].kind {
            RawKind::Number(n) => self.number(i, *n),
            RawKind::Word => self.word(i),
            RawKind::Sign(code) => {
                self.sign(i, code);
                1
            }
            RawKind::Sym(sym) => {
                self.push(Tok::Op(symbol(*sym)), range, SpanKind::Operator);
                1
            }
            RawKind::Time(time) => {
                self.push(Tok::Time(*time), range, SpanKind::DateTime);
                1
            }
            RawKind::Date(date) => {
                self.push(Tok::Date(*date), range, SpanKind::DateTime);
                1
            }
            RawKind::Other => {
                self.push(Tok::Text, range, SpanKind::Text);
                1
            }
        }
    }

    fn number(&mut self, i: usize, n: f64) -> usize {
        if let Some((date, used)) = self.day_month_year(i, n) {
            self.push(Tok::Date(date), self.span(i, i + used - 1), SpanKind::DateTime);
            return used;
        }
        self.push(Tok::Num(n), self.raws[i].range.clone(), SpanKind::Number);
        1
    }

    /// `12 March [2026]`.
    fn day_month_year(&self, i: usize, day: f64) -> Option<(NaiveDate, usize)> {
        let day = small_int(day, 31)?;
        let month = self
            .spaced(i)
            .then(|| self.word_at(i + 1).and_then(month))
            .flatten()?;
        let (year, used) = match self.year_at(i + 2).filter(|_| self.spaced(i + 1)) {
            Some(year) => (year, 3),
            None => (self.ctx.year, 2),
        };
        Some((NaiveDate::from_ymd_opt(year, month, day)?, used))
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
        let tok = if before_amount && !self.prev_ends_value() {
            Tok::PrefixCurrency(code.to_owned())
        } else if self.prev_ends_value() || self.prev_expects_unit() {
            Tok::Unit(Unit::simple(UnitRef::Currency(code.to_owned()), Form::Default))
        } else {
            Tok::Text
        };
        self.push(tok, range, SpanKind::Currency);
    }

    fn word(&mut self, i: usize) -> usize {
        if let Some(used) = self.iana_zone(i) {
            return used;
        }
        if let Some(m) = month(self.text(i))
            && !(self.ctx.is_variable)(self.text(i))
            && let Some((date, used)) = self.month_day_year(i, m)
        {
            self.push(Tok::Date(date), self.span(i, i + used - 1), SpanKind::DateTime);
            return used;
        }
        let window = MAX_WORDS_IN_NAMES.max(self.ctx.custom.max_phrase_tokens);
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
        let range = self.span(i, j);
        let tz = zones::iana(&self.ctx.line[range.clone()])?;
        self.push(Tok::Zone(tz), range, SpanKind::DateTime);
        Some(j - i + 1)
    }

    /// Tries raws `i..=j` as one multi-token phrase.
    fn phrase(&mut self, i: usize, j: usize) -> bool {
        let raws = &self.raws[i..=j];
        let word_like = raws
            .iter()
            .all(|r| matches!(r.kind, RawKind::Word | RawKind::Other));
        let ends_well = self.raws[j].kind == RawKind::Word || self.text(j) == ".";
        if !word_like || !ends_well || !(i..j).all(|k| self.spaced(k)) {
            return false;
        }
        let range = self.span(i, j);
        let text = custom::normalize(&self.ctx.line[range.clone()]);
        let words_only = raws.iter().all(|r| r.kind == RawKind::Word);
        let unit_context = self.prev_ends_value() || self.prev_expects_unit();
        if let Some(index) = self.ctx.custom.unit(&text)
            && unit_context
        {
            self.push(
                Tok::Unit(Unit::simple(UnitRef::Custom(index), Form::Default)),
                range,
                SpanKind::Unit,
            );
            return true;
        }
        if !words_only {
            return false;
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
        if let Some(tz) = zones::city(&text)
            && self.zone_context(j)
        {
            self.push(Tok::Zone(tz), range, SpanKind::DateTime);
            return true;
        }
        false
    }

    /// Whether a zone name ending at raw `last` is used as one: `Berlin time`, `in Berlin`,
    /// `2:30 pm HKT`.
    fn zone_context(&self, last: usize) -> bool {
        let before = matches!(self.prev(), Some(Tok::Convert(_) | Tok::Time(_) | Tok::Date(_)));
        let after = self.spaced(last)
            && self
                .word_at(last + 1)
                .is_some_and(|w| matches!(w.to_lowercase().as_str(), "time" | "now"));
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
        if (self.ctx.is_variable)(&text) {
            self.push(Tok::Var(text), range, SpanKind::Variable);
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
            && i > 0
            && self.number_at(i - 1).is_some()
            && let Some(Item {
                tok: Tok::Num(n),
                range: num_range,
                ..
            }) = self.out.last_mut()
            && num_range.end == self.raws[i - 1].range.end
        {
            *n *= factor;
            num_range.end = range.end;
            return;
        }
        if let Some(tz) = zones::abbreviation(&text).or_else(|| zones::city(&text))
            && self.zone_context(i)
        {
            self.push(Tok::Zone(tz), range, SpanKind::DateTime);
            return;
        }
        self.unit_or_text(i, &text, range);
    }

    fn unit_or_text(&mut self, i: usize, text: &str, range: Range<usize>) {
        let Some(word) = self.unit_word(text) else {
            self.push(Tok::Text, range, SpanKind::Text);
            return;
        };
        if self.prev_ends_value() || self.prev_expects_unit() {
            self.push(Tok::Unit(word.unit), range, word.kind);
        } else if let Some(code) = word.prefix_code
            && self.number_at(i + 1).is_some()
        {
            self.push(Tok::PrefixCurrency(code.to_owned()), range, SpanKind::Currency);
        } else {
            self.push(Tok::Text, range, SpanKind::Text);
        }
    }

    /// The unit a word or phrase names, if any.
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
