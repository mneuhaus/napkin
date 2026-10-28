//! Splits the code part of a line into raw tokens: numbers, dates, times, words and symbols.

use std::ops::Range;

use chrono::{NaiveDate, NaiveTime};

use crate::NumberStyle;
use crate::currency;

/// An operator or punctuation symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sym {
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Amp,
    Pipe,
    Shl,
    Shr,
    Bang,
    Percent,
    LParen,
    RParen,
    Semicolon,
    Equals,
    Sup2,
    Sup3,
}

/// What a raw token is.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RawKind {
    Number(f64),
    Word,
    /// A currency sign with its code.
    Sign(&'static str),
    Sym(Sym),
    Time(NaiveTime),
    Date(NaiveDate),
    /// Punctuation the engine ignores.
    Other,
}

/// A token with its byte range in the line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Raw {
    pub(crate) kind: RawKind,
    pub(crate) range: Range<usize>,
}

/// Lexes `text`, which starts at byte `offset` of its line.
pub(crate) fn lex(text: &str, offset: usize, style: NumberStyle) -> Vec<Raw> {
    let mut lexer = Lexer {
        s: text,
        i: 0,
        style,
        out: Vec::new(),
    };
    lexer.run();
    for raw in &mut lexer.out {
        raw.range = raw.range.start + offset..raw.range.end + offset;
    }
    lexer.out
}

struct Lexer<'src> {
    s: &'src str,
    i: usize,
    style: NumberStyle,
    out: Vec<Raw>,
}

fn is_word_start(c: char) -> bool {
    c.is_alphabetic() || matches!(c, '_' | '°' | '℃' | '℉')
}

/// Before or after noon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Meridiem {
    Am,
    Pm,
}

/// Whether a byte separates digits inside a number-like run.
fn is_separator(byte: Option<u8>) -> bool {
    matches!(byte, Some(b'.' | b',' | b':' | b'\''))
}

/// `am`/`pm` in all lowercase or all uppercase (so `Pm` stays petametres).
fn meridiem(word: &str) -> Option<Meridiem> {
    if word != word.to_ascii_lowercase() && word != word.to_ascii_uppercase() {
        return None;
    }
    match word.to_ascii_lowercase().as_str() {
        "am" | "a.m." => Some(Meridiem::Am),
        "pm" | "p.m." => Some(Meridiem::Pm),
        _ => None,
    }
}

impl Lexer<'_> {
    fn peek(&self, at: usize) -> Option<char> {
        self.s.get(at..).and_then(|rest| rest.chars().next())
    }

    fn byte(&self, at: usize) -> Option<u8> {
        self.s.as_bytes().get(at).copied()
    }

    fn digit_at(&self, at: usize) -> bool {
        self.byte(at).is_some_and(|b| b.is_ascii_digit())
    }

    /// Number of ASCII digits starting at `at`.
    fn digits_from(&self, at: usize) -> usize {
        self.s
            .as_bytes()
            .get(at..)
            .map_or(0, |rest| rest.iter().take_while(|b| b.is_ascii_digit()).count())
    }

    fn push(&mut self, kind: RawKind, start: usize) {
        self.out.push(Raw {
            kind,
            range: start..self.i,
        });
    }

    fn run(&mut self) {
        while let Some(c) = self.peek(self.i) {
            let start = self.i;
            if c.is_whitespace() {
                self.i += c.len_utf8();
            } else if c.is_ascii_digit() || (c == '.' && self.digit_at(start + 1)) {
                self.number(start);
            } else if is_word_start(c) {
                self.word(start);
            } else if let Some(code) = currency::sign(c) {
                self.i += c.len_utf8();
                self.push(RawKind::Sign(code), start);
            } else {
                self.symbol(c, start);
            }
        }
    }

    fn word(&mut self, start: usize) {
        let mut chars = self.s[start..].char_indices().peekable();
        let mut end = start;
        while let Some((at, c)) = chars.next() {
            let continues = if at == 0 {
                true
            } else if matches!(c, '\'' | '’') {
                chars.peek().is_some_and(|(_, next)| next.is_alphabetic())
            } else {
                c.is_alphabetic() || c.is_ascii_digit() || c == '_'
            };
            if !continues {
                break;
            }
            end = start + at + c.len_utf8();
        }
        self.i = end;
        self.push(RawKind::Word, start);
    }

    fn symbol(&mut self, c: char, start: usize) {
        let next = self.peek(start + c.len_utf8());
        let (sym, len) = match (c, next) {
            ('*', Some('*')) => (Some(Sym::Caret), 2),
            ('<', Some('<')) => (Some(Sym::Shl), 2),
            ('>', Some('>')) => (Some(Sym::Shr), 2),
            ('+', _) => (Some(Sym::Plus), 1),
            ('-' | '−' | '–' | '—', _) => (Some(Sym::Minus), 1),
            ('*' | '×' | '·' | '⋅', _) => (Some(Sym::Star), 1),
            ('/' | '÷' | '∕', _) => (Some(Sym::Slash), 1),
            ('^', _) => (Some(Sym::Caret), 1),
            ('&', _) => (Some(Sym::Amp), 1),
            ('|', _) => (Some(Sym::Pipe), 1),
            ('!', _) => (Some(Sym::Bang), 1),
            ('%', _) => (Some(Sym::Percent), 1),
            ('(' | '[' | '{', _) => (Some(Sym::LParen), 1),
            (')' | ']' | '}', _) => (Some(Sym::RParen), 1),
            (';', _) => (Some(Sym::Semicolon), 1),
            ('=', _) => (Some(Sym::Equals), 1),
            ('²', _) => (Some(Sym::Sup2), 1),
            ('³', _) => (Some(Sym::Sup3), 1),
            _ => (None, 1),
        };
        self.i = if len == 2 { start + 2 } else { start + c.len_utf8() };
        self.push(sym.map_or(RawKind::Other, RawKind::Sym), start);
    }

    fn number(&mut self, start: usize) {
        if let Some((kind, end)) = self
            .radix_number(start)
            .or_else(|| self.date(start))
            .or_else(|| self.clock(start))
        {
            let iso_time_follows = self.byte(end) == Some(b'T') && self.digit_at(end + 1);
            self.i = end;
            self.push(kind, start);
            if iso_time_follows {
                self.i = end + 1;
            }
            return;
        }
        let (value, end, integer) = self.decimal(start);
        self.i = end;
        if is_separator(self.byte(end)) && self.digit_at(end + 1) {
            // `3:4`, `1.2.3`, `31.02.2026`: not a number we can read, so not a calculation.
            self.i = self.run_end(end);
            self.push(RawKind::Other, start);
            return;
        }
        if let Some((time, end)) = integer.and_then(|hour| self.hour_ampm(hour, end)) {
            self.i = end;
            self.push(RawKind::Time(time), start);
            return;
        }
        self.push(RawKind::Number(value), start);
    }

    /// `0x1F`, `0b101`, `0o17`.
    fn radix_number(&self, start: usize) -> Option<(RawKind, usize)> {
        if self.byte(start)? != b'0' {
            return None;
        }
        let radix = match self.byte(start + 1)? {
            b'x' | b'X' => 16,
            b'b' | b'B' => 2,
            b'o' | b'O' => 8,
            _ => return None,
        };
        let body = &self.s[start + 2..];
        let len = body
            .bytes()
            .take_while(|b| char::from(*b).is_digit(radix))
            .count();
        if len == 0 || body[len..].chars().next().is_some_and(char::is_alphanumeric) {
            return None;
        }
        let value = body[..len]
            .chars()
            .filter_map(|c| c.to_digit(radix))
            .fold(0.0, |acc: f64, d| acc * f64::from(radix) + f64::from(d));
        Some((RawKind::Number(value), start + 2 + len))
    }

    /// `2026-03-12`, `12.03.2026`, `3/12/2026` (month first in English style).
    fn date(&self, start: usize) -> Option<(RawKind, usize)> {
        let first = self.digits_from(start);
        let sep_byte = self.byte(start + first)?;
        let second_at = start + first + 1;
        let second = self.digits_from(second_at);
        if !b"-./".contains(&sep_byte) || self.byte(second_at + second)? != sep_byte {
            return None;
        }
        let sep = char::from(sep_byte);
        let third_at = second_at + second + 1;
        let third = self.digits_from(third_at);
        let end = third_at + third;
        let iso_time = self.byte(end) == Some(b'T') && self.digit_at(end + 1);
        if self.peek(end).is_some_and(char::is_alphanumeric) && !iso_time {
            return None;
        }
        let part = |at: usize, len: usize| self.s[at..at + len].parse::<u32>().ok();
        let (a, b, c) = (
            part(start, first)?,
            part(second_at, second)?,
            part(third_at, third)?,
        );
        let year = |y: u32| i32::try_from(y).ok();
        let date = match (sep, first, second, third) {
            ('-', 4, 1..=2, 1..=2) => NaiveDate::from_ymd_opt(year(a)?, b, c),
            ('/', 1..=2, 1..=2, 4) if self.style == NumberStyle::English => {
                NaiveDate::from_ymd_opt(year(c)?, a, b)
            }
            ('.' | '/', 1..=2, 1..=2, 4) => NaiveDate::from_ymd_opt(year(c)?, b, a),
            _ => None,
        }?;
        Some((RawKind::Date(date), end))
    }

    /// `14:30`, `2:30 pm`, `14:30:15`.
    fn clock(&self, start: usize) -> Option<(RawKind, usize)> {
        let hour_len = self.digits_from(start);
        if !(1..=2).contains(&hour_len) || self.byte(start + hour_len)? != b':' {
            return None;
        }
        let minute_at = start + hour_len + 1;
        if self.digits_from(minute_at) != 2 {
            return None;
        }
        let hour: u32 = self.s[start..start + hour_len].parse().ok()?;
        let minute: u32 = self.s[minute_at..minute_at + 2].parse().ok()?;
        let mut end = minute_at + 2;
        let mut second = 0;
        if self.byte(end) == Some(b':') && self.digits_from(end + 1) == 2 {
            second = self.s[end + 1..end + 3].parse().ok()?;
            end += 3;
        }
        if let Some((meridiem, after)) = self.ampm_after(end) {
            let time = NaiveTime::from_hms_opt(twelve_hour(hour, meridiem)?, minute, second)?;
            return Some((RawKind::Time(time), after));
        }
        if self.peek(end).is_some_and(char::is_alphanumeric) {
            return None;
        }
        let hour = if (hour, minute, second) == (24, 0, 0) {
            0
        } else {
            hour
        };
        Some((RawKind::Time(NaiveTime::from_hms_opt(hour, minute, second)?), end))
    }

    /// `2pm`, `11 am`: an integer hour followed by am/pm.
    fn hour_ampm(&self, hour: u32, end: usize) -> Option<(NaiveTime, usize)> {
        let (meridiem, after) = self.ampm_after(end)?;
        Some((
            NaiveTime::from_hms_opt(twelve_hour(hour, meridiem)?, 0, 0)?,
            after,
        ))
    }

    /// An `am`/`pm` marker after optional spaces at `at`, and where it ends.
    fn ampm_after(&self, at: usize) -> Option<(Meridiem, usize)> {
        let rest = &self.s[at..];
        let trimmed = rest.trim_start_matches(' ');
        let word_at = at + rest.len() - trimmed.len();
        let len = trimmed
            .bytes()
            .take_while(|b| b.is_ascii_alphabetic() || *b == b'.')
            .count();
        let word = trimmed[..len].trim_end_matches('.');
        let word_len = if trimmed[..len].to_ascii_lowercase().starts_with("a.m.")
            || trimmed[..len].to_ascii_lowercase().starts_with("p.m.")
        {
            4
        } else {
            word.len()
        };
        let meridiem = meridiem(&trimmed[..word_len])?;
        if trimmed[word_len..]
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
        {
            return None;
        }
        Some((meridiem, word_at + word_len))
    }

    /// A decimal number per the number style: (value, end, the integer value if it is a plain
    /// integer without grouping, decimals or exponent).
    ///
    /// A grouping separator that is not followed by a group of three digits is read as a decimal
    /// separator (`3,14` in English style, `1.5` in German style), apostrophes excepted.
    fn decimal(&self, start: usize) -> (f64, usize, Option<u32>) {
        let (group_seps, decimal_sep): (&[u8], u8) = match self.style {
            NumberStyle::English | NumberStyle::Plain => (b",", b'.'),
            NumberStyle::Swiss => (b"',", b'.'),
            NumberStyle::German | NumberStyle::French => (b".", b','),
        };
        let mut digits = String::new();
        let mut i = start;
        let first = self.digits_from(i);
        digits.push_str(&self.s[i..i + first]);
        i += first;
        let mut plain = first > 0;
        if (1..=3).contains(&first) {
            let (grouped, end) = self.groups(i, group_seps, &mut digits);
            plain &= !grouped;
            i = end;
        }
        let decimal_at = |i: usize| {
            let b = self.byte(i)?;
            let lenient = b != b'\'' && group_seps.contains(&b);
            ((b == decimal_sep || lenient) && self.digit_at(i + 1)).then_some(i + 1)
        };
        if let Some(frac_at) = decimal_at(i) {
            let len = self.digits_from(frac_at);
            digits.push('.');
            digits.push_str(&self.s[frac_at..frac_at + len]);
            i = frac_at + len;
            plain = false;
        }
        if matches!(self.byte(i), Some(b'e' | b'E')) {
            let sign = usize::from(matches!(self.byte(i + 1), Some(b'+' | b'-')));
            let len = self.digits_from(i + 1 + sign);
            if len > 0 {
                digits.push('e');
                digits.push_str(&self.s[i + 1..i + 1 + sign + len]);
                i += 1 + sign + len;
                plain = false;
            }
        }
        let value = digits.parse::<f64>().unwrap_or_default();
        let integer = if plain { digits.parse::<u32>().ok() } else { None };
        (value, i, integer)
    }

    /// Digit groups of exactly three after the first group: `1,000,000`, `5 300` (also with a
    /// no-break space).
    fn groups(&self, mut i: usize, group_seps: &[u8], digits: &mut String) -> (bool, usize) {
        let mut grouped = false;
        while let Some(c) = self.peek(i) {
            let is_separator = matches!(c, ' ' | '\u{a0}' | '\u{202f}')
                || u8::try_from(c).is_ok_and(|b| group_seps.contains(&b));
            let group_at = i + c.len_utf8();
            let group_ok = self.digits_from(group_at) == 3 && !matches!(self.byte(group_at + 3), Some(b':'));
            if !is_separator || !group_ok {
                break;
            }
            digits.push_str(&self.s[group_at..group_at + 3]);
            i = group_at + 3;
            grouped = true;
        }
        (grouped, i)
    }

    /// The end of a run of digits and separators (`1.2.3`, `3:4`).
    fn run_end(&self, mut i: usize) -> usize {
        loop {
            let continues = self.digit_at(i) || (is_separator(self.byte(i)) && self.digit_at(i + 1));
            if !continues {
                return i;
            }
            i += 1;
        }
    }
}

fn twelve_hour(hour: u32, meridiem: Meridiem) -> Option<u32> {
    if !(1..=12).contains(&hour) {
        return None;
    }
    Some(hour % 12 + if meridiem == Meridiem::Pm { 12 } else { 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str, style: NumberStyle) -> Vec<RawKind> {
        lex(text, 0, style).into_iter().map(|r| r.kind).collect()
    }

    fn numbers(text: &str, style: NumberStyle) -> Vec<f64> {
        kinds(text, style)
            .into_iter()
            .filter_map(|k| if let RawKind::Number(n) = k { Some(n) } else { None })
            .collect()
    }

    #[test]
    fn number_styles() {
        assert_eq!(numbers("1,000,000", NumberStyle::English), [1e6]);
        assert_eq!(numbers("5 300", NumberStyle::English), [5300.0]);
        assert_eq!(numbers("1 2", NumberStyle::English), [1.0, 2.0]);
        assert_eq!(numbers("3,25", NumberStyle::English), [3.25]);
        assert_eq!(numbers("1,000", NumberStyle::Plain), [1000.0]);
        assert_eq!(numbers("5\u{a0}300", NumberStyle::English), [5300.0]);
        assert_eq!(kinds("3:4", NumberStyle::English), [RawKind::Other]);
        assert_eq!(kinds("1.2.3", NumberStyle::English), [RawKind::Other]);
        assert_eq!(numbers("2.5E-4", NumberStyle::English), [2.5e-4]);
        assert_eq!(numbers("1.234,5", NumberStyle::German), [1234.5]);
        assert_eq!(numbers("1,5", NumberStyle::German), [1.5]);
        assert_eq!(numbers("1'234.5", NumberStyle::Swiss), [1234.5]);
        assert_eq!(numbers("1 234,5", NumberStyle::French), [1234.5]);
        assert_eq!(
            numbers("0x1F 0b101 0o17", NumberStyle::English),
            [31.0, 5.0, 15.0]
        );
    }

    #[test]
    fn dates_and_times() {
        let t = |h, m| RawKind::Time(NaiveTime::from_hms_opt(h, m, 0).expect("valid"));
        assert_eq!(kinds("2:30 pm", NumberStyle::English), [t(14, 30)]);
        assert_eq!(kinds("2pm", NumberStyle::English), [t(14, 0)]);
        assert_eq!(kinds("12am", NumberStyle::English), [t(0, 0)]);
        assert_eq!(kinds("14:30", NumberStyle::English), [t(14, 30)]);
        assert_eq!(kinds("24:00", NumberStyle::English), [t(0, 0)]);
        assert_eq!(
            kinds("2 Pm", NumberStyle::English).len(),
            2,
            "Pm is petametres, not pm"
        );
        let d = |y, m, day| RawKind::Date(NaiveDate::from_ymd_opt(y, m, day).expect("valid"));
        assert_eq!(kinds("2026-03-12", NumberStyle::English), [d(2026, 3, 12)]);
        assert_eq!(kinds("12.03.2026", NumberStyle::German), [d(2026, 3, 12)]);
        assert_eq!(kinds("3/12/2026", NumberStyle::English), [d(2026, 3, 12)]);
        assert_eq!(kinds("3/12/2026", NumberStyle::German), [d(2026, 12, 3)]);
        assert_eq!(
            kinds("2026-03-12T14:30", NumberStyle::English),
            [d(2026, 3, 12), t(14, 30)]
        );
    }

    #[test]
    fn words_and_symbols() {
        let raws = lex("20px °C don't 2**3", 0, NumberStyle::English);
        let texts: Vec<_> = raws
            .iter()
            .map(|r| &"20px °C don't 2**3"[r.range.clone()])
            .collect();
        assert_eq!(texts, ["20", "px", "°C", "don't", "2", "**", "3"]);
        assert_eq!(raws[5].kind, RawKind::Sym(Sym::Caret));
    }
}
