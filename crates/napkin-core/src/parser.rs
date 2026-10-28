//! A Pratt parser over classified tokens, with implicit multiplication, unit attachment,
//! conversions and Numi's percentage phrases.

use chrono::{NaiveDate, NaiveTime};
use chrono_tz::Tz;

use crate::EvalError;
use crate::classify::{ConvertWord, DocRef, Func, Op, PercentOp, Tok};
use crate::format::Radix;
use crate::units::{Form, Unit, UnitRef};

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinOp {
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
}

/// What `in`/`to`/`as` converts to.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Target {
    Unit(Unit),
    Radix(Radix),
    Zone(Tz),
}

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expr {
    Num(f64),
    Neg(Box<Expr>),
    /// A value with a unit attached (`20 cm`, `$30`, `$30 CAD`).
    WithUnit(Box<Expr>, Unit),
    /// A unit on its own, worth one of it (`h` in `km/h`).
    BareUnit(Unit),
    Percent(Box<Expr>),
    Fact(Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// Two operands next to each other: multiplied, or added when both are quantities of the
    /// same dimension (`1 meter 20 cm`).
    Juxtapose(Box<Expr>, Box<Expr>),
    Call(Func, Vec<Expr>),
    Var(String),
    Doc(DocRef),
    Now,
    Day(i64),
    /// A time of day, optionally in a named zone (`2:30 pm HKT`).
    TimeLit(NaiveTime, Option<Tz>),
    DateLit(NaiveDate, Option<Tz>),
    /// The current time in a zone (`Berlin time`).
    ZoneNow(Tz),
    Convert(Box<Expr>, Target),
    /// `20% of $10`, `5% on $30`, `6% off 40 EUR`.
    PercentOf(PercentOp, Box<Expr>, Box<Expr>),
    /// `5% of what is 6 EUR`.
    PercentWhat(PercentOp, Box<Expr>, Box<Expr>),
    /// `$50 as a % of $100`.
    AsPercent(PercentOp, Box<Expr>, Box<Expr>),
}

const CONVERT: u8 = 1;
const ADD: u8 = 6;
const MUL: u8 = 7;
const JUXTAPOSE: u8 = 8;
const POW: u8 = 9;
const PREFIX: u8 = 10;

/// Deepest nesting of parentheses, signs and operators a line may have; keeps parsing and
/// evaluation from exhausting the stack on pathological input.
const MAX_DEPTH: usize = 100;

/// Parses a whole token list.
pub(crate) fn parse(tokens: &[Tok]) -> Result<Expr, EvalError> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        depth: 0,
    };
    let expr = parser.expr(0)?;
    match parser.peek() {
        None => Ok(expr),
        Some(tok) => Err(EvalError::new(format!("unexpected {}", describe(tok)))),
    }
}

fn describe(tok: &Tok) -> &'static str {
    match tok {
        Tok::Op(Op::RParen) => "`)`",
        Tok::Op(_) => "operator",
        Tok::Unit(_) => "unit",
        Tok::Convert(_) => "conversion",
        _ => "token",
    }
}

struct Parser<'a> {
    tokens: &'a [Tok],
    pos: usize,
    /// Current nesting of `expr` calls, bounded by [`MAX_DEPTH`].
    depth: usize,
}

/// How an infix token combines its operands.
enum Infix {
    Binary(BinOp),
    Percent(PercentOp),
    Convert(ConvertWord),
    Juxtapose,
}

pub(crate) fn starts_operand(tok: &Tok) -> bool {
    matches!(
        tok,
        Tok::Num(_)
            | Tok::Op(Op::LParen)
            | Tok::Var(_)
            | Tok::Const(_)
            | Tok::Func(_)
            | Tok::Doc(_)
            | Tok::Now
            | Tok::Day(_)
            | Tok::Time(_)
            | Tok::Date(_)
            | Tok::PrefixCurrency(_)
    )
}

/// Left and right binding power of an infix token.
fn infix(tok: &Tok) -> Option<(u8, u8, Infix)> {
    let binary = |l, op| Some((l, l + 1, Infix::Binary(op)));
    match tok {
        Tok::Op(Op::Add) => binary(ADD, BinOp::Add),
        Tok::Op(Op::Sub) => binary(ADD, BinOp::Sub),
        Tok::Op(Op::Mul) => binary(MUL, BinOp::Mul),
        Tok::Op(Op::Div) => binary(MUL, BinOp::Div),
        Tok::Op(Op::Mod) => binary(MUL, BinOp::Mod),
        Tok::Op(Op::Pow) => Some((POW, POW, Infix::Binary(BinOp::Pow))),
        Tok::Op(Op::BitOr) => binary(2, BinOp::BitOr),
        Tok::Op(Op::Xor) => binary(3, BinOp::Xor),
        Tok::Op(Op::BitAnd) => binary(4, BinOp::BitAnd),
        Tok::Op(Op::Shl) => binary(5, BinOp::Shl),
        Tok::Op(Op::Shr) => binary(5, BinOp::Shr),
        Tok::Percent(op) => Some((MUL, MUL + 1, Infix::Percent(*op))),
        Tok::Convert(word) => Some((CONVERT, CONVERT + 1, Infix::Convert(*word))),
        tok if starts_operand(tok) => Some((JUXTAPOSE, POW, Infix::Juxtapose)),
        _ => None,
    }
}

fn boxed(expr: Expr) -> Box<Expr> {
    Box::new(expr)
}

/// Anchors date and time literals in `tz`; `None` when `expr` is not a literal.
fn anchor(expr: &Expr, tz: Tz) -> Option<Expr> {
    match expr {
        Expr::TimeLit(time, None) => Some(Expr::TimeLit(*time, Some(tz))),
        Expr::DateLit(date, None) => Some(Expr::DateLit(*date, Some(tz))),
        Expr::Juxtapose(a, b) => Some(Expr::Juxtapose(boxed(anchor(a, tz)?), boxed(anchor(b, tz)?))),
        _ => None,
    }
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a Tok> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, ahead: usize) -> Option<&'a Tok> {
        self.tokens.get(self.pos + ahead)
    }

    fn bump(&mut self) -> Option<&'a Tok> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    fn expr(&mut self, min_bp: u8) -> Result<Expr, EvalError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(EvalError::new("expression nested too deeply"));
        }
        let result = self.expr_inner(min_bp);
        self.depth -= 1;
        result
    }

    fn expr_inner(&mut self, min_bp: u8) -> Result<Expr, EvalError> {
        let mut lhs = self.prefix()?;
        while let Some(tok) = self.peek() {
            if let Some(next) = self.postfix(&lhs)? {
                lhs = next;
                continue;
            }
            let Some((left, right, kind)) = infix(tok) else {
                break;
            };
            if left < min_bp {
                break;
            }
            self.pos += usize::from(!matches!(kind, Infix::Juxtapose));
            lhs = match kind {
                Infix::Binary(op) => Expr::Binary(op, boxed(lhs), boxed(self.expr(right)?)),
                Infix::Juxtapose => Expr::Juxtapose(boxed(lhs), boxed(self.expr(right)?)),
                Infix::Percent(op) => self.percent_phrase(op, lhs)?,
                Infix::Convert(word) => self.conversion(word, lhs)?,
            };
        }
        Ok(lhs)
    }

    /// Applies a postfix token to `lhs`, if the next token is one.
    fn postfix(&mut self, lhs: &Expr) -> Result<Option<Expr>, EvalError> {
        let Some(tok) = self.peek() else { return Ok(None) };
        let lhs = || boxed(lhs.clone());
        Ok(Some(match tok {
            Tok::Op(Op::Fact) => {
                self.pos += 1;
                Expr::Fact(lhs())
            }
            Tok::Op(Op::Percent) => {
                self.pos += 1;
                Expr::Percent(lhs())
            }
            Tok::Op(op @ (Op::Sup2 | Op::Sup3)) => {
                let power = if *op == Op::Sup2 { 2.0 } else { 3.0 };
                self.pos += 1;
                Expr::Binary(BinOp::Pow, lhs(), boxed(Expr::Num(power)))
            }
            Tok::Unit(_) | Tok::Power(_) => {
                // `60 km/h` is one unit, so `30 km / 60 km/h` divides by a speed.
                let mut unit = self.unit_factor()?;
                while self.peek() == Some(&Tok::Op(Op::Div))
                    && matches!(self.peek_at(1), Some(Tok::Unit(_) | Tok::Power(_)))
                {
                    self.pos += 1;
                    unit = unit.times(&self.unit_factor()?.pow(-1));
                }
                Expr::WithUnit(lhs(), unit)
            }
            Tok::Zone(tz) => {
                let tz = *tz;
                self.pos += 1;
                let expr = lhs();
                anchor(&expr, tz).unwrap_or(Expr::Convert(expr, Target::Zone(tz)))
            }
            _ => return Ok(None),
        }))
    }

    fn prefix(&mut self) -> Result<Expr, EvalError> {
        let Some(tok) = self.bump().cloned() else {
            return Err(EvalError::new("incomplete expression"));
        };
        Ok(match tok {
            Tok::Num(n) | Tok::Const(n) => Expr::Num(n),
            Tok::Op(Op::Sub) => Expr::Neg(boxed(self.expr(PREFIX)?)),
            Tok::Op(Op::Add) => self.expr(PREFIX)?,
            Tok::Op(Op::LParen) => self.group()?,
            Tok::PrefixCurrency(code) => {
                let amount = if self.peek() == Some(&Tok::Op(Op::Sub)) {
                    self.pos += 1;
                    Expr::Neg(boxed(self.prefix()?))
                } else {
                    self.prefix()?
                };
                Expr::WithUnit(
                    boxed(amount),
                    Unit::simple(UnitRef::Currency(code), Form::Default),
                )
            }
            Tok::Unit(_) | Tok::Power(_) => {
                self.pos -= 1;
                Expr::BareUnit(self.unit_factor()?)
            }
            Tok::Func(func) => self.call(func)?,
            Tok::Var(name) => Expr::Var(name),
            Tok::Doc(doc) => Expr::Doc(doc),
            Tok::Now => Expr::Now,
            Tok::Day(offset) => Expr::Day(offset),
            Tok::Time(time) => Expr::TimeLit(time, None),
            Tok::Date(date) => Expr::DateLit(date, None),
            Tok::Zone(tz) => {
                if self.peek() == Some(&Tok::Now) {
                    self.pos += 1;
                }
                Expr::ZoneNow(tz)
            }
            other => return Err(EvalError::new(format!("unexpected {}", describe(&other)))),
        })
    }

    /// After `(`: an expression and its closing parenthesis (optional at the end of the line).
    fn group(&mut self) -> Result<Expr, EvalError> {
        if self.peek() == Some(&Tok::Op(Op::RParen)) {
            return Err(EvalError::new("empty parentheses"));
        }
        let inner = self.expr(0)?;
        match self.peek() {
            Some(Tok::Op(Op::RParen)) => self.pos += 1,
            None => {}
            Some(tok) => return Err(EvalError::new(format!("expected `)`, found {}", describe(tok)))),
        }
        Ok(inner)
    }

    /// A function call: `f(a; b)`, `f x`, or `log 2 (10)`.
    fn call(&mut self, func: Func) -> Result<Expr, EvalError> {
        if self.peek() == Some(&Tok::Op(Op::LParen)) {
            self.pos += 1;
            return Ok(Expr::Call(func, self.arguments()?));
        }
        let first = self.expr(POW)?;
        let leading = matches!(func, Func::Builtin(b) if b.takes_leading_argument());
        if leading && self.peek() == Some(&Tok::Op(Op::LParen)) {
            self.pos += 1;
            let mut args = vec![first];
            args.extend(self.arguments()?);
            return Ok(Expr::Call(func, args));
        }
        Ok(Expr::Call(func, vec![first]))
    }

    /// After `(`: arguments separated by `;` up to `)` (optional at the end of the line).
    fn arguments(&mut self) -> Result<Vec<Expr>, EvalError> {
        let mut args = Vec::new();
        if self.peek() == Some(&Tok::Op(Op::RParen)) {
            self.pos += 1;
            return Ok(args);
        }
        loop {
            args.push(self.expr(0)?);
            match self.bump() {
                Some(Tok::Op(Op::Semi)) => {}
                Some(Tok::Op(Op::RParen)) | None => return Ok(args),
                Some(tok) => {
                    return Err(EvalError::new(format!(
                        "expected `;` or `)`, found {}",
                        describe(tok)
                    )));
                }
            }
        }
    }

    /// After `of`/`on`/`off`: `P% of X` or `P% of what is X`.
    fn percent_phrase(&mut self, op: PercentOp, lhs: Expr) -> Result<Expr, EvalError> {
        if self.peek() == Some(&Tok::What) && self.peek_at(1) == Some(&Tok::Is) {
            self.pos += 2;
            return Ok(Expr::PercentWhat(op, boxed(lhs), boxed(self.expr(ADD)?)));
        }
        Ok(Expr::PercentOf(op, boxed(lhs), boxed(self.expr(MUL + 1)?)))
    }

    /// After `in`/`to`/`as`: a conversion target or `as a % of Y`.
    fn conversion(&mut self, word: ConvertWord, lhs: Expr) -> Result<Expr, EvalError> {
        let percent_sign = match self.peek() {
            Some(Tok::Op(Op::Percent)) => true,
            Some(Tok::Unit(unit)) => unit.is_percent(),
            _ => false,
        };
        if percent_sign {
            if word == ConvertWord::As
                && let Some(Tok::Percent(op)) = self.peek_at(1)
            {
                let op = *op;
                self.pos += 2;
                return Ok(Expr::AsPercent(op, boxed(lhs), boxed(self.expr(CONVERT + 1)?)));
            }
            self.pos += 1;
            let percent = Unit::simple(UnitRef::Percent, Form::Default);
            return Ok(Expr::Convert(boxed(lhs), Target::Unit(percent)));
        }
        let target = match self.peek() {
            Some(Tok::Radix(radix)) => {
                let radix = *radix;
                self.pos += 1;
                Target::Radix(radix)
            }
            Some(Tok::Zone(tz)) => {
                let tz = *tz;
                self.pos += 1;
                Target::Zone(tz)
            }
            Some(Tok::Unit(_) | Tok::Power(_)) => Target::Unit(self.unit_product()?),
            _ => {
                return Err(EvalError::new(
                    "expected a unit, format or time zone after the conversion",
                ));
            }
        };
        Ok(Expr::Convert(boxed(lhs), target))
    }

    /// A unit with an optional `sq`/`cubic` prefix and `²`, `³`, `^n` suffix.
    fn unit_factor(&mut self) -> Result<Unit, EvalError> {
        let power = match self.peek() {
            Some(Tok::Power(power)) => {
                let power = *power;
                self.pos += 1;
                power
            }
            _ => 1,
        };
        let Some(Tok::Unit(unit)) = self.bump() else {
            return Err(EvalError::new("expected a unit"));
        };
        let mut unit = unit.pow(power);
        loop {
            match (self.peek(), self.peek_at(1), self.peek_at(2)) {
                (Some(Tok::Op(Op::Sup2)), ..) => {
                    self.pos += 1;
                    unit.raise(2);
                }
                (Some(Tok::Op(Op::Sup3)), ..) => {
                    self.pos += 1;
                    unit.raise(3);
                }
                (Some(Tok::Op(Op::Pow)), Some(Tok::Num(n)), _) => {
                    let Some(power) = crate::num::to_power(*n) else {
                        break;
                    };
                    self.pos += 2;
                    unit.raise(power);
                }
                (Some(Tok::Op(Op::Pow)), Some(Tok::Op(Op::Sub)), Some(Tok::Num(n))) => {
                    let Some(power) = crate::num::to_power(-*n) else {
                        break;
                    };
                    self.pos += 3;
                    unit.raise(power);
                }
                _ => break,
            }
        }
        Ok(unit)
    }

    /// A conversion target unit: `km/h`, `m²`, `kW h`.
    fn unit_product(&mut self) -> Result<Unit, EvalError> {
        let mut unit = self.unit_factor()?;
        loop {
            let is_unit = |tok: Option<&Tok>| matches!(tok, Some(Tok::Unit(_) | Tok::Power(_)));
            match self.peek() {
                Some(Tok::Op(Op::Div)) if is_unit(self.peek_at(1)) => {
                    self.pos += 1;
                    unit = unit.times(&self.unit_factor()?.pow(-1));
                }
                Some(Tok::Op(Op::Mul)) if is_unit(self.peek_at(1)) => {
                    self.pos += 1;
                    unit = unit.times(&self.unit_factor()?);
                }
                Some(Tok::Unit(_) | Tok::Power(_)) => unit = unit.times(&self.unit_factor()?),
                _ => return Ok(unit),
            }
        }
    }
}
