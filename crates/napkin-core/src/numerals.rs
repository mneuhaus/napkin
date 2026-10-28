//! Numbers written without digits: Chinese and Japanese numerals (`三百五十`, `两千`,
//! `一万五千`, `五点五`), from the characters a language lists under `numeral.<value>`.
//!
//! The grammar, as numbers are spoken:
//!
//! - A digit before a multiplier counts that many of it (`三百` 300). Inside a group of four
//!   digits the multipliers fall (`三百五十`); a numeral ends before one that does not
//!   (`五十千米` is 50 and `千米`). A multiplier at the start counts one (`十二` 12, Japanese
//!   `千円` 1000 yen), and so does `十` at the start of a later group.
//! - Group multipliers (`万`, `亿`) close a group and fall too (`一亿五千万`); one right after a
//!   smaller one multiplies with it (`五万亿` is 5·10^12).
//! - A zero after a multiplier is a placeholder (`一百零五` 105).
//! - A last digit right after a multiplier of 100 or more counts one step below it, as spoken
//!   (`三百五` 350, `一万五` 15000).
//! - Three or more digits in a row are read place by place (`二〇二六` 2026); two in a row are a
//!   range (`三五个`, three or five), so the numeral ends after the first.
//! - The decimal point needs digits after it (`五点五` 5.5); `三点` stays three o'clock, and so
//!   does `三点五分` (5 past 3).

use std::borrow::Cow;

use crate::classify::Context;
use crate::custom;
use crate::document::MAX_NAME_RAWS;
use crate::lexer::{Raw, RawKind, is_ideographic};
use crate::units::BUILTINS;
use crate::vocab::{Lexicon, Meaning};

/// What a character is in a numeral system written without digits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Numeral {
    /// 0 to 9: `〇`, `三`, `两`.
    Digit(u8),
    /// 10, 100 or 1000 inside a group of four digits: `十`, `百`, `千`.
    Multiplier(u16),
    /// A power of ten from 10^4 that closes a group: `万`, `亿`.
    Group(f64),
    /// The decimal point: `点`.
    Point,
}

impl Numeral {
    /// The numeral a data file key stands for: `numeral.3`, `numeral.100`, `numeral.1e4`,
    /// `numeral.point`.
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        if key == "point" {
            return Some(Self::Point);
        }
        let value = key.parse::<f64>().ok()?;
        match crate::num::to_i64(value).filter(|_| value.fract() == 0.0)? {
            digit @ 0..=9 => u8::try_from(digit).ok().map(Self::Digit),
            multiplier @ (10 | 100 | 1000) => u16::try_from(multiplier).ok().map(Self::Multiplier),
            _ if value >= 1e4 && value.log10().fract() == 0.0 => Some(Self::Group(value)),
            _ => None,
        }
    }
}

/// What came before the current character of a numeral.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Last {
    Start,
    Digit,
    /// A zero placeholder after a multiplier.
    Zero,
    Multiplier(u16),
    Group(f64),
}

/// The longest numeral at the start of `chars`: its value and how many characters it takes.
/// Stops before a decimal point (see [`fraction`]).
pub(crate) fn parse(chars: &[Numeral]) -> Option<(f64, usize)> {
    place_value(chars).or_else(|| grouped(chars))
}

/// `二〇二六`: three or more digits in a row, read place by place.
fn place_value(chars: &[Numeral]) -> Option<(f64, usize)> {
    let digits: Vec<u8> = chars
        .iter()
        .map_while(|c| match c {
            Numeral::Digit(d) => Some(*d),
            _ => None,
        })
        .collect();
    (digits.len() >= 3).then(|| {
        let value = digits
            .iter()
            .fold(0.0, |acc, digit| acc * 10.0 + f64::from(*digit));
        (value, digits.len())
    })
}

/// `三百五十`, `一万五千`, `一百零五`: digits counting multipliers.
fn grouped(chars: &[Numeral]) -> Option<(f64, usize)> {
    let mut total = 0.0;
    let mut group = 0.0;
    // A digit not yet counted by a multiplier, and what it is worth if the numeral ends there.
    let mut pending: Option<(f64, f64)> = None;
    let mut multiplier: Option<u16> = None;
    let mut group_multiplier: Option<f64> = None;
    let mut last = Last::Start;
    let mut used = 0;
    for (at, &c) in chars.iter().enumerate() {
        match c {
            Numeral::Digit(_) if pending.is_some() => break,
            Numeral::Digit(0) if matches!(last, Last::Multiplier(_) | Last::Group(_)) => {
                last = Last::Zero;
                continue;
            }
            Numeral::Digit(digit) => {
                let worth = match last {
                    Last::Multiplier(m) if m >= 100 => f64::from(m / 10),
                    Last::Group(g) => g / 10.0,
                    Last::Start | Last::Digit | Last::Zero | Last::Multiplier(_) => 1.0,
                };
                let digit = f64::from(digit);
                pending = Some((digit, digit * worth));
                last = Last::Digit;
            }
            Numeral::Multiplier(m) => {
                if multiplier.is_some_and(|before| m >= before) {
                    break;
                }
                let count = match pending.take() {
                    Some((digit, _)) => digit,
                    None if last == Last::Start || (m == 10 && multiplier.is_none() && group == 0.0) => 1.0,
                    None => break,
                };
                group += count * f64::from(m);
                multiplier = Some(m);
                last = Last::Multiplier(m);
            }
            Numeral::Group(g) => {
                if let Last::Group(before) = last
                    && g > before
                {
                    // `五万亿`: 万亿 is one multiplier.
                    total *= g;
                    group_multiplier = Some(before * g);
                    last = Last::Group(before * g);
                } else {
                    // `万` at the start counts one (Korean `만원`).
                    let count = if last == Last::Start {
                        1.0
                    } else {
                        group + pending.map_or(0.0, |(digit, _)| digit)
                    };
                    if group_multiplier.is_some_and(|before| g >= before) || count == 0.0 {
                        break;
                    }
                    total += count * g;
                    group = 0.0;
                    pending = None;
                    multiplier = None;
                    group_multiplier = Some(g);
                    last = Last::Group(g);
                }
            }
            Numeral::Point => break,
        }
        used = at + 1;
    }
    let tail = pending.map_or(0.0, |(_, worth)| worth);
    (used > 0).then_some((total + group + tail, used))
}

/// `点五`, `点一四`: a decimal point and the digits after it, as a fraction and its length.
pub(crate) fn fraction(chars: &[Numeral]) -> Option<(f64, usize)> {
    if chars.first() != Some(&Numeral::Point) {
        return None;
    }
    let mut value = 0.0;
    let mut scale = 0.1;
    let mut len = 0;
    for c in &chars[1..] {
        let Numeral::Digit(digit) = c else { break };
        value += f64::from(*digit) * scale;
        scale /= 10.0;
        len += 1;
    }
    (len > 0).then_some((value, 1 + len))
}

/// Joins numerals written without digits into number raws (`三百五十` becomes 350), the
/// longest phrase first: a vocabulary phrase or variable that starts at the same character
/// and is at least as long keeps its characters (`一共` is the sum, `十六进制` hexadecimal,
/// `万一` prose), unless it only names a scale (`十万` alone is the numeral 100000). A
/// multiplier right after digits stays the scale word it is (`5十万`).
pub(crate) fn merge<'r>(raws: &'r [Raw], ctx: &Context<'_>) -> Cow<'r, [Raw]> {
    let numeral_char = |raw: &Raw| single_char(ctx.line, raw).and_then(|c| ctx.lexicon.numeral(c));
    if !ctx.lexicon.has_numerals() || !raws.iter().any(|raw| numeral_char(raw).is_some()) {
        return Cow::Borrowed(raws);
    }
    let mut out = Vec::with_capacity(raws.len());
    let mut p = 0;
    while p < raws.len() {
        if raws[p].kind != RawKind::Word {
            out.push(raws[p].clone());
            p += 1;
            continue;
        }
        let phrase = phrase_len(raws, p, ctx);
        match numeral_at(raws, p, ctx) {
            Some((value, len)) if len > phrase => {
                out.push(Raw {
                    kind: RawKind::Number(value),
                    range: raws[p].range.start..raws[p + len - 1].range.end,
                });
                p += len;
            }
            _ => {
                let len = phrase.max(1);
                out.extend_from_slice(&raws[p..p + len]);
                p += len;
            }
        }
    }
    Cow::Owned(out)
}

/// The character of a Chinese or Japanese word raw (one character each).
fn single_char(line: &str, raw: &Raw) -> Option<char> {
    if raw.kind != RawKind::Word {
        return None;
    }
    let mut chars = line[raw.range.clone()].chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if is_ideographic(c) => Some(c),
        _ => None,
    }
}

/// A Korean word that starts with a numeral (`삼백`, `오천원`, `이십분`, `만원`): its value, its
/// length in bytes and the byte offsets where the rest of the word splits further into known
/// words. Single syllables are particles and words as often as numbers (`이`, `일`), so the
/// numeral needs a multiplier (`이사`, `사이` stay words) and the rest must be known words; a
/// word that is only a numeral needs two syllables and must start with a digit or `십` (`천사`
/// is an angel, `백조` a swan).
pub(crate) fn korean(text: &str, lexicon: &Lexicon) -> Option<(f64, usize, Vec<usize>)> {
    let chars: Vec<Numeral> = text.chars().map_while(|c| lexicon.numeral(c)).collect();
    let (value, used) = grouped(&chars)?;
    let multiplied = chars[..used]
        .iter()
        .any(|c| matches!(c, Numeral::Multiplier(_) | Numeral::Group(_)));
    if !multiplied {
        return None;
    }
    let end = text.char_indices().nth(used).map_or(text.len(), |(at, _)| at);
    if end < text.len() {
        let cuts = crate::classify::decompose(&text[end..], lexicon)?;
        return Some((value, end, cuts.into_iter().map(|cut| cut + end).collect()));
    }
    let digit_first = matches!(chars.first(), Some(Numeral::Digit(_) | Numeral::Multiplier(10)));
    (used >= 2 && digit_first).then_some((value, end, Vec::new()))
}

fn touching(raws: &[Raw], k: usize) -> bool {
    raws[k].range.end == raws[k + 1].range.start
}

/// How many raws the longest vocabulary phrase (other than a bare scale) or variable starting
/// at raw `p` takes, 0 when none does.
fn phrase_len(raws: &[Raw], p: usize, ctx: &Context<'_>) -> usize {
    let line = ctx.line;
    let window = ctx
        .lexicon
        .window(&line[raws[p].range.clone()])
        .max(MAX_NAME_RAWS);
    let last = (p + window).min(raws.len()) - 1;
    // The last raw of the touching words from `p`: as far as a variable name reaches.
    let word_end = p
        + (p..last)
            .take_while(|&k| touching(raws, k) && raws[k + 1].kind == RawKind::Word)
            .count();
    (p..=last)
        .rev()
        .find_map(|j| {
            let spaced = (p..j).all(|k| line[raws[k].range.end..raws[k + 1].range.start].trim().is_empty());
            if !spaced {
                return None;
            }
            let text = custom::normalize(&line[raws[p].range.start..raws[j].range.end]);
            let phrase = ctx
                .lexicon
                .meanings(&text)
                .iter()
                .any(|m| !matches!(m, Meaning::Scale(_)));
            let variable = j <= word_end && (ctx.is_variable)(&text);
            (phrase || variable).then_some(j - p + 1)
        })
        .unwrap_or(0)
}

/// The numeral starting at raw `p`, if one starts there: its value and how many raws it takes.
fn numeral_at(raws: &[Raw], p: usize, ctx: &Context<'_>) -> Option<(f64, usize)> {
    let numeral = |k: usize| single_char(ctx.line, raws.get(k)?).and_then(|c| ctx.lexicon.numeral(c));
    let after_digits = p > 0 && matches!(raws[p - 1].kind, RawKind::Number(_)) && touching(raws, p - 1);
    match numeral(p)? {
        Numeral::Digit(_) => {}
        // `十二`, Japanese `千円`; after digits `5千` the multiplier is a scale word.
        Numeral::Multiplier(_) if !after_digits => {}
        Numeral::Multiplier(_) | Numeral::Group(_) | Numeral::Point => return None,
    }
    let mut run = Vec::new();
    while let Some(c) = numeral(p + run.len()).filter(|_| run.is_empty() || touching(raws, p + run.len() - 1))
    {
        run.push(c);
    }
    let (whole, used) = parse(&run)?;
    match fraction(&run[used..]) {
        Some((part, len)) if !minutes_after(raws, p + used + len, ctx) => Some((whole + part, used + len)),
        _ => Some((whole, used)),
    }
}

/// Whether raw `k` touches the one before it and is a minutes word (`三点五分` is 3:05).
fn minutes_after(raws: &[Raw], k: usize, ctx: &Context<'_>) -> bool {
    let Some(raw) = raws
        .get(k)
        .filter(|raw| raw.kind == RawKind::Word && touching(raws, k - 1))
    else {
        return false;
    };
    let minute = BUILTINS.by_id("min");
    ctx.lexicon
        .meanings(&ctx.line[raw.range.clone()])
        .iter()
        .any(|m| matches!(m, Meaning::Unit { index, power: 1 } if Some(*index) == minute))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chinese numerals, as `lang/zh-Hans.txt` lists them.
    fn chars(text: &str) -> Vec<Numeral> {
        text.chars()
            .map(|c| match c {
                '〇' | '零' => Numeral::Digit(0),
                '一' => Numeral::Digit(1),
                '二' | '两' => Numeral::Digit(2),
                '三' => Numeral::Digit(3),
                '四' => Numeral::Digit(4),
                '五' => Numeral::Digit(5),
                '六' => Numeral::Digit(6),
                '七' => Numeral::Digit(7),
                '八' => Numeral::Digit(8),
                '九' => Numeral::Digit(9),
                '十' => Numeral::Multiplier(10),
                '百' => Numeral::Multiplier(100),
                '千' => Numeral::Multiplier(1000),
                '万' => Numeral::Group(1e4),
                '亿' => Numeral::Group(1e8),
                '点' => Numeral::Point,
                other => panic!("not a numeral: {other}"),
            })
            .collect()
    }

    #[test]
    fn grammar() {
        let cases = [
            ("三百五十", 350.0, 4),
            ("两千", 2000.0, 2),
            ("一万五千", 15_000.0, 4),
            ("三亿", 3e8, 2),
            ("十二", 12.0, 2),
            ("十", 10.0, 1),
            ("二十", 20.0, 2),
            ("一百零五", 105.0, 4),
            ("一千零五十", 1050.0, 5),
            ("三百五", 350.0, 3),
            ("一万五", 15_000.0, 3),
            ("一亿五千万", 1.5e8, 5),
            ("五万亿", 5e12, 3),
            ("十万", 1e5, 2),
            ("三十万", 3e5, 3),
            ("二〇二六", 2026.0, 4),
            ("五十千", 50.0, 2),
            ("三五", 3.0, 1),
            ("一百零", 100.0, 2),
            ("五点五", 5.0, 1),
            ("千", 1000.0, 1),
            ("百五十", 150.0, 3),
            ("万", 1e4, 1),
        ];
        for (text, value, used) in cases {
            assert_eq!(parse(&chars(text)), Some((value, used)), "{text}");
        }
        assert_eq!(parse(&chars("点")), None);
    }

    #[test]
    fn fractions() {
        let (value, len) = fraction(&chars("点一四")).expect("a fraction");
        assert!((value - 0.14).abs() < 1e-12);
        assert_eq!(len, 3);
        assert_eq!(fraction(&chars("点")), None);
        assert_eq!(fraction(&chars("五")), None);
    }

    #[test]
    fn keys() {
        assert_eq!(Numeral::from_key("3"), Some(Numeral::Digit(3)));
        assert_eq!(Numeral::from_key("100"), Some(Numeral::Multiplier(100)));
        assert_eq!(Numeral::from_key("1e8"), Some(Numeral::Group(1e8)));
        assert_eq!(Numeral::from_key("point"), Some(Numeral::Point));
        assert_eq!(Numeral::from_key("12"), None);
        assert_eq!(Numeral::from_key("2e4"), None);
    }
}
