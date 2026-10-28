//! Document semantics: headers, comments, labels, assignments, `prev`, `sum` and `average`.

use std::collections::HashMap;
use std::ops::Range;

use chrono::{DateTime, Datelike, Utc};

use crate::classify::{self, Context, Item, Op, Tok};
use crate::eval::Env;
use crate::functions::Builtin;
use crate::lexer::{self, Raw, RawKind, Sym};
use crate::parser::{self, Expr};
use crate::units::{BUILTINS, Form, Unit, UnitRef};
use crate::value::{Units, Value};
use crate::vocab::{Lexicon, Meaning};
use crate::zones::Zone;
use crate::{Clock, Engine, EvalError, LineOutput, LineResult, Span, SpanKind};

/// CSS pixels per inch until the document assigns `ppi`.
const DEFAULT_PPI: f64 = 96.0;
/// CSS pixels per em until the document assigns `em`.
const DEFAULT_EM_PX: f64 = 16.0;

/// The evaluation state of one document, top to bottom.
pub(crate) struct Document<'a> {
    engine: &'a Engine,
    vars: HashMap<String, Value>,
    globals: HashMap<String, Result<Value, EvalError>>,
    prev: Option<Value>,
    block: Vec<Value>,
    ppi: f64,
    em_px: f64,
    now: DateTime<Utc>,
    local: Zone,
    lexicon: Lexicon,
}

/// Where an assignment's parts are in a line.
#[derive(Debug)]
struct Assignment {
    /// Byte range of the variable name.
    name: Range<usize>,
    /// Byte range of `=` or the assignment words.
    operator: Range<usize>,
    operator_kind: SpanKind,
    /// Index of the expression's first raw.
    expression: usize,
}

/// Most raws a variable name can span: names in Chinese or Japanese characters are one raw per
/// character.
const MAX_NAME_RAWS: usize = 8;

/// Where the label, comments and code of a line are.
#[derive(Debug, Default)]
struct Layout {
    label: Option<Range<usize>>,
    comments: Vec<Range<usize>>,
    code: Vec<Range<usize>>,
}

fn closing_quote(c: char) -> Option<char> {
    match c {
        '"' => Some('"'),
        '“' | '„' => Some('”'),
        '«' => Some('»'),
        _ => None,
    }
}

/// Whether a `"` between `prev` and `next` marks a Hebrew abbreviation (`ש"ח`, the ASCII stand-in
/// for gershayim) rather than opening a quoted comment.
fn hebrew_abbreviation(prev: Option<char>, next: Option<char>) -> bool {
    let hebrew = |c: Option<char>| c.is_some_and(|c| ('\u{05D0}'..='\u{05EA}').contains(&c));
    hebrew(prev) && hebrew(next)
}

/// Splits a line into label, comments and code.
fn layout(line: &str) -> Layout {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut quote = None;
    let mut slashes = None;
    let mut colon = None;
    for (k, &(i, c)) in chars.iter().enumerate() {
        if let Some(close) = quote {
            if c == close || (close == '”' && c == '“') {
                quote = None;
            }
            continue;
        }
        let prev = k.checked_sub(1).and_then(|p| chars.get(p)).map(|&(_, c)| c);
        let next = chars.get(k + 1).map(|&(_, c)| c);
        if let Some(close) = closing_quote(c).filter(|_| !hebrew_abbreviation(prev, next)) {
            quote = Some(close);
        } else if c == '/' && next == Some('/') {
            slashes = Some(i);
            break;
        } else if c == ':' && colon.is_none() {
            let ends_word = next.is_none_or(char::is_whitespace);
            let between_digits =
                prev.is_some_and(|p| p.is_ascii_digit()) && next.is_some_and(|n| n.is_ascii_digit());
            if ends_word && !between_digits {
                colon = Some(i);
            }
        }
    }
    let mut out = Layout::default();
    let content_start = line.len() - line.trim_start().len();
    let code_start = colon.map_or(0, |c| c + 1);
    if let Some(c) = colon {
        out.label = Some(content_start..c + 1);
    }
    let code_end = slashes.unwrap_or(line.len());
    if let Some(start) = slashes {
        out.comments.push(start..line.trim_end().len());
    }
    split_quotes(line, code_start..code_end, &mut out);
    out
}

/// Splits `region` into code and quoted comments.
fn split_quotes(line: &str, region: Range<usize>, out: &mut Layout) {
    let mut start = region.start;
    let mut quote: Option<(usize, char)> = None;
    for (i, c) in line[region.clone()]
        .char_indices()
        .map(|(i, c)| (i + region.start, c))
    {
        match quote {
            Some((open, close)) if c == close || (close == '”' && c == '“' && i > open) => {
                out.comments.push(open..i + c.len_utf8());
                start = i + c.len_utf8();
                quote = None;
            }
            Some(_) => {}
            None => {
                let prev = line[..i].chars().next_back();
                let next = line[i + c.len_utf8()..].chars().next();
                if let Some(close) = closing_quote(c).filter(|_| !hebrew_abbreviation(prev, next)) {
                    out.code.push(start..i);
                    quote = Some((i, close));
                }
            }
        }
    }
    match quote {
        Some((open, _)) => out
            .comments
            .push(open..line[..region.end].trim_end().len().max(open + 1)),
        None => out.code.push(start..region.end),
    }
    out.code.retain(|r| !line[r.clone()].trim().is_empty());
}

/// Most significant tokens a line may have; longer lines are refused rather than risking deep
/// recursion.
const MAX_TOKENS: usize = 300;

/// The first unknown word written right after a bare number that a conversion follows
/// (`10 XYZ in USD`, `1 furlongz in m`). Ignoring it would silently turn the number into the
/// target unit, so the line fails instead. Prose elsewhere stays ignored.
fn unknown_unit_before_conversion(items: &[Item]) -> Option<Range<usize>> {
    items.windows(2).enumerate().find_map(|(index, pair)| {
        let [number, word] = pair else { return None };
        if !matches!(number.tok, Tok::Num(_)) || word.tok != Tok::Text {
            return None;
        }
        let next = items[index + 2..].iter().find(|item| item.tok != Tok::Text)?;
        matches!(next.tok, Tok::Convert(_)).then(|| word.range.clone())
    })
}

/// The tokens the parser sees: prose dropped, everything after a stray `=` dropped, and a
/// [`Tok::Gap`] where prose separated two operands (so `3 apples 4 pears` does not multiply).
fn significant(items: &[Item]) -> Vec<Tok> {
    let mut tokens: Vec<Tok> = Vec::with_capacity(items.len());
    let mut after_prose = false;
    for item in items.iter().take_while(|item| item.tok != Tok::Op(Op::Equals)) {
        if item.tok == Tok::Text {
            after_prose = true;
            continue;
        }
        if after_prose && tokens.last().is_some_and(Tok::ends_value) && parser::starts_operand(&item.tok) {
            tokens.push(Tok::Gap);
        }
        after_prose = false;
        tokens.push(item.tok.clone());
    }
    tokens
}

/// Whether a token can end an expression.
fn can_end(tok: &Tok) -> bool {
    !matches!(
        tok,
        Tok::Op(
            Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Div
                | Op::Pow
                | Op::Mod
                | Op::BitAnd
                | Op::BitOr
                | Op::Xor
                | Op::Shl
                | Op::Shr
                | Op::LParen
                | Op::Semi
        ) | Tok::Convert(_)
            | Tok::Percent(_)
            | Tok::AsPercent(_)
            | Tok::PercentWhat(_)
            | Tok::What
            | Tok::Is
            | Tok::Power(_)
            | Tok::Func(_)
            | Tok::PrefixCurrency(_)
            | Tok::PercentBefore
            | Tok::Gap
    )
}

/// Whether a token can start an expression.
fn can_start(tok: &Tok) -> bool {
    !matches!(
        tok,
        Tok::Op(
            Op::Mul
                | Op::Div
                | Op::Pow
                | Op::Mod
                | Op::BitAnd
                | Op::BitOr
                | Op::Xor
                | Op::Shl
                | Op::Shr
                | Op::RParen
                | Op::Semi
                | Op::Fact
                | Op::Percent
                | Op::Sup2
                | Op::Sup3
        ) | Tok::Convert(_)
            | Tok::Percent(_)
            | Tok::AsPercent(_)
            | Tok::PercentWhat(_)
            | Tok::What
            | Tok::Is
            | Tok::Radix(_)
            | Tok::LocalZone
            | Tok::Gap
    )
}

/// Drops operators and keywords dangling at either end.
fn trim(tokens: &[Tok]) -> &[Tok] {
    let start = tokens.iter().position(can_start).unwrap_or(tokens.len());
    let end = tokens
        .iter()
        .rposition(can_end)
        .map_or(start, |e| (e + 1).max(start));
    &tokens[start..end]
}

/// Parses a line's tokens, falling back to the longest sensible prefix when the whole line does
/// not parse (prose tolerance).
fn parse_tolerant(tokens: &[Tok]) -> Result<Expr, EvalError> {
    let first_error = match parser::parse(tokens) {
        Ok(expr) => return Ok(expr),
        Err(error) => error,
    };
    let trimmed = trim(tokens);
    for end in (1..=trimmed.len()).rev() {
        let candidate = trim(&trimmed[..end]);
        if candidate.iter().any(Tok::is_value)
            && let Ok(expr) = parser::parse(candidate)
        {
            return Ok(expr);
        }
    }
    Err(first_error)
}

fn none(spans: Vec<Span>) -> LineOutput {
    LineOutput {
        spans,
        result: LineResult::None,
    }
}

impl<'a> Document<'a> {
    pub(crate) fn new(engine: &'a Engine) -> Self {
        let (now, local) = match engine.clock {
            Clock::System => (Utc::now(), Zone::Local),
            Clock::Fixed { now, time_zone } => (now, Zone::Named(time_zone)),
        };
        let mut document = Self {
            engine,
            vars: HashMap::new(),
            globals: HashMap::new(),
            prev: None,
            block: Vec::new(),
            ppi: DEFAULT_PPI,
            em_px: DEFAULT_EM_PX,
            now,
            local,
            lexicon: Lexicon::new(engine.settings.input_language),
        };
        let globals = engine
            .globals
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    document
                        .env(&|_| None)
                        .quantity_from_ext(value)
                        .map(Value::Quantity),
                )
            })
            .collect();
        document.globals = globals;
        document
    }

    fn units(&self) -> Units<'a> {
        Units {
            custom: &self.engine.custom,
            rates: &self.engine.rates,
            ppi: self.ppi,
            em_px: self.em_px,
        }
    }

    fn env<'s>(&'s self, lookup: crate::eval::Lookup<'s>) -> Env<'s> {
        Env {
            units: self.units(),
            settings: &self.engine.settings,
            now: self.now,
            local: self.local,
            functions: &self.engine.custom.functions,
            lookup,
            prev: self.prev.as_ref(),
            block: &self.block,
        }
    }

    fn is_variable(&self, name: &str) -> bool {
        self.vars.contains_key(name) || self.globals.contains_key(name)
    }

    /// Evaluates the next line of the document.
    pub(crate) fn line(&mut self, raw_line: &str) -> LineOutput {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let content = line.trim();
        let content_start = line.len() - line.trim_start().len();
        if content.is_empty() {
            self.block.clear();
            return none(Vec::new());
        }
        if content.starts_with(['#', '＃']) {
            self.block.clear();
            let range = content_start..content_start + content.len();
            return none(vec![Span {
                range,
                kind: SpanKind::Header,
            }]);
        }
        let layout = layout(line);
        let mut spans: Vec<Span> = layout
            .label
            .iter()
            .map(|r| Span {
                range: r.clone(),
                kind: SpanKind::Label,
            })
            .collect();
        spans.extend(layout.comments.iter().map(|r| Span {
            range: r.clone(),
            kind: SpanKind::Comment,
        }));
        let style = self.engine.settings.number_style;
        let raws: Vec<Raw> = layout
            .code
            .iter()
            .flat_map(|r| lexer::lex(&line[r.clone()], r.start, style))
            .collect();
        let raws = classify::segment(line, raws, &self.lexicon);
        let result = self.code(line, &raws, &mut spans);
        if let (LineResult::Value(_), Some(value)) = (&result.0, result.1) {
            self.block.push(value.clone());
            self.prev = Some(value);
        }
        spans.sort_by_key(|s| s.range.start);
        LineOutput {
            spans,
            result: result.0,
        }
    }

    /// Evaluates the code tokens of a line; returns the result and its value.
    fn code(&mut self, line: &str, raws: &[Raw], spans: &mut Vec<Span>) -> (LineResult, Option<Value>) {
        if let [digits, word, eq, ..] = raws
            && matches!(digits.kind, RawKind::Number(_))
            && word.kind == RawKind::Word
            && digits.range.end == word.range.start
            && eq.kind == RawKind::Sym(Sym::Equals)
        {
            let name = &line[digits.range.start..word.range.end];
            let items = self.classify(line, raws);
            spans.extend(items.iter().map(|item| Span {
                range: item.range.clone(),
                kind: item.kind,
            }));
            let error = EvalError::new(format!("`{name}` is not a valid variable name"));
            return (LineResult::Error(error), None);
        }
        let assignment = self.assignment(line, raws);
        let expression = assignment.as_ref().map_or(raws, |a| &raws[a.expression..]);
        let items = self.classify(line, expression);
        spans.extend(items.iter().map(|item| Span {
            range: item.range.clone(),
            kind: item.kind,
        }));
        let Some(assignment) = assignment else {
            return self.evaluate(line, &items);
        };
        spans.push(Span {
            range: assignment.name.clone(),
            kind: SpanKind::Variable,
        });
        spans.push(Span {
            range: assignment.operator.clone(),
            kind: assignment.operator_kind,
        });
        self.assign(line, &line[assignment.name], &items)
    }

    /// `name = ...`, `name is ...`, `x ist gleich ...`, `价格 = ...`: where the name, the
    /// assignment operator and the expression are. A word operator only assigns to a valid,
    /// unreserved name; `=` assigns to any name, so a bad name is reported.
    fn assignment(&self, line: &str, raws: &[Raw]) -> Option<Assignment> {
        let words = raws.iter().take_while(|raw| raw.kind == RawKind::Word).count();
        for name_raws in 1..=words.min(MAX_NAME_RAWS) {
            let touching = raws[..name_raws]
                .windows(2)
                .all(|pair| pair[0].range.end == pair[1].range.start);
            if !touching {
                break;
            }
            let name = raws[0].range.start..raws[name_raws - 1].range.end;
            let operator_at = raws.get(name_raws)?;
            if operator_at.kind == RawKind::Sym(Sym::Equals) {
                return Some(Assignment {
                    name,
                    operator: operator_at.range.clone(),
                    operator_kind: SpanKind::Operator,
                    expression: name_raws + 1,
                });
            }
            let Some(words) = self.assignment_words(line, &raws[name_raws..]) else {
                continue;
            };
            if self.check_name(&line[name.clone()]).is_ok() {
                return Some(Assignment {
                    name,
                    operator: operator_at.range.start..raws[name_raws + words - 1].range.end,
                    operator_kind: SpanKind::Keyword,
                    expression: name_raws + words,
                });
            }
        }
        None
    }

    /// How many raws at the start of `raws` form an assignment phrase (`is`, `ist gleich`).
    fn assignment_words(&self, line: &str, raws: &[Raw]) -> Option<usize> {
        let first = &line[raws.first()?.range.clone()];
        let window = self.lexicon.window(first).min(raws.len());
        (1..=window).rev().find(|&count| {
            let spaced = raws[..count]
                .windows(2)
                .all(|pair| line[pair[0].range.end..pair[1].range.start].trim().is_empty());
            let text = &line[raws[0].range.start..raws[count - 1].range.end];
            spaced && self.lexicon.meanings(text).contains(&Meaning::Assign)
        })
    }

    fn classify(&self, line: &str, raws: &[Raw]) -> Vec<Item> {
        let is_variable = |name: &str| self.is_variable(name);
        let ctx = Context {
            line,
            custom: &self.engine.custom,
            is_variable: &is_variable,
            year: self.local.naive(self.now).year(),
            lexicon: self.lexicon,
        };
        classify::classify(raws, &ctx)
    }

    fn evaluate(&self, line: &str, items: &[Item]) -> (LineResult, Option<Value>) {
        let tokens = significant(items);
        let missing_prev = self.prev.is_none() && tokens.contains(&Tok::Doc(classify::DocRef::Prev));
        if missing_prev || !tokens.iter().any(Tok::is_value) {
            return (LineResult::None, None);
        }
        if tokens.len() > MAX_TOKENS {
            return (
                LineResult::Error(EvalError::new("the line is too long to calculate")),
                None,
            );
        }
        if let Some(word) = unknown_unit_before_conversion(items) {
            let message = format!("unknown unit \"{}\"", &line[word]);
            return (LineResult::Error(EvalError::new(message)), None);
        }
        let lookup = |name: &str| {
            self.vars
                .get(name)
                .cloned()
                .map(Ok)
                .or_else(|| self.globals.get(name).cloned())
        };
        let env = self.env(&lookup);
        match parse_tolerant(&tokens).and_then(|expr| env.eval(&expr)) {
            Ok(value) => {
                let answer = env.units.answer(&value, &self.engine.settings, self.now);
                (LineResult::Value(answer), Some(value))
            }
            Err(error) => (LineResult::Error(error), None),
        }
    }

    fn assign(&mut self, line: &str, name: &str, items: &[Item]) -> (LineResult, Option<Value>) {
        let special = matches!(name, "em" | "ppi");
        if !special && let Err(error) = self.check_name(name) {
            return (LineResult::Error(error), None);
        }
        let (result, value) = self.evaluate(line, items);
        let Some(value) = value else {
            return (result, None);
        };
        if special && let Err(error) = self.set_css(name, &value) {
            return (LineResult::Error(error), None);
        }
        if !special {
            self.vars.insert(name.to_owned(), value.clone());
        }
        (result, Some(value))
    }

    fn set_css(&mut self, name: &str, value: &Value) -> Result<(), EvalError> {
        let Value::Quantity(q) = value else {
            return Err(EvalError::new(format!("`{name}` needs a number")));
        };
        if name == "ppi" && !q.is_plain() {
            return Err(EvalError::new("`ppi` needs a plain number"));
        }
        let size = if name == "ppi" || q.is_plain() {
            q.value
        } else {
            let px = BUILTINS.by_id("px").expect("px is built in");
            self.units()
                .convert(q, &Unit::simple(UnitRef::Builtin(px), Form::Default))?
        };
        if !size.is_finite() || size <= 0.0 {
            return Err(EvalError::new(format!("`{name}` must be positive")));
        }
        if name == "ppi" {
            self.ppi = size;
        } else {
            self.em_px = size;
        }
        Ok(())
    }

    fn check_name(&self, name: &str) -> Result<(), EvalError> {
        check_variable_name(name, &self.engine.custom).map_err(EvalError::new)
    }
}

/// Whether `name` may be a variable: letters, digits and `_`, not starting with a digit, and not
/// a unit, currency, keyword, function or constant.
pub(crate) fn check_variable_name(name: &str, custom: &crate::custom::Registry) -> Result<(), String> {
    let mut chars = name.chars();
    let valid_start = chars.next().is_some_and(|c| c.is_alphabetic() || c == '_');
    if !valid_start || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(format!("`{name}` is not a valid variable name"));
    }
    let taken = classify::is_reserved(name) || Builtin::from_name(name).is_some() || custom.names(name);
    if taken {
        return Err(format!(
            "`{name}` is a unit, keyword or function and cannot be a variable"
        ));
    }
    Ok(())
}

/// Evaluates every line of `text`.
pub(crate) fn evaluate(engine: &Engine, text: &str) -> Vec<LineOutput> {
    let mut document = Document::new(engine);
    text.split('\n').map(|line| document.line(line)).collect()
}

/// Evaluates one expression without document context.
pub(crate) fn evaluate_expression(engine: &Engine, input: &str) -> LineResult {
    let single_line = input.replace(['\n', '\r'], " ");
    Document::new(engine).line(&single_line).result
}
