//! Input vocabularies: the words of each [`InputLanguage`], read from the data files in `lang/`
//! and layered on top of English, plus the language's way of showing word units and dates.
//!
//! # Data files
//!
//! `lang/<tag>.txt` holds one entry per line, `key = phrase, phrase, ...`. Lines starting with
//! `#` are comments; an indented line continues the entry above it. In a phrase, `{a,b}` stands
//! for each alternative in turn (`Stunde{,n}` is `Stunde` and `Stunden`; several blocks
//! multiply). Phrases of one or two characters (`T`, `Wo`) match their exact case, longer ones
//! any case; whitespace inside a phrase matches any run of spaces and `’` matches `'`.
//!
//! | Key | Meaning |
//! |---|---|
//! | `base` | a tag whose entries come first (`base = de` in `de-CH`) |
//! | `plus`, `minus`, `times`, `divide`, `modulo` | operators between two operands |
//! | `per` | division that also takes a bare unit after it (`km pro Stunde`) |
//! | `convert` | conversion, like `in` |
//! | `assign` | assignment after a variable name: `x ist gleich 5` |
//! | `percent` | the word for `%` |
//! | `percent.before` | a percent word written before its number (`百分之20`) |
//! | `percent.of`, `percent.on`, `percent.off` | after a percentage: `20 % of 50`, `5 % on 30`, `6 % off 40` |
//! | `as-percent.of`, `as-percent.on`, `as-percent.off` | `50 as a % of 100` |
//! | `what-is.of`, `what-is.on`, `what-is.off` | `5 % of what is 6` |
//! | `prev`, `sum`, `average` | the line above, the block's sum and mean |
//! | `today`, `tomorrow`, `yesterday`, `day.<offset>` | days relative to today (`day.2 = übermorgen`) |
//! | `now`, `local`, `noon`, `midnight` | the current time, the local time zone, 12:00, 0:00 |
//! | `oclock` | after an hour a time of day (`14 Uhr`, `3点`); add the word to `now` too if it means now alone |
//! | `half-hour` | after an o'clock word: half past (`3点半`); minutes work as `3点20分` |
//! | `am`, `pm` | before the hour: morning, afternoon (`下午3点`, `오후 3시`) |
//! | `convert-after` | a conversion word after its target (`5인치 cm로`, `100ドルをユーロに`) |
//! | `unit-before.<id>` | a unit written before its number (`unit-before.°C = 摄氏`: `摄氏100度`) |
//! | `scale.<factor>` | after a number: `scale.1e3 = Tausend` |
//! | `function.<name>` | a built-in function: `function.sqrt = Quadratwurzel` |
//! | `format.<name>` | a conversion format: `format.hex = hexadezimal` |
//! | `unit.<id>` | a built-in unit by id (`unit.h`, `unit.km`); a trailing `2` or `3` squares or cubes it (`unit.cm3`) |
//! | `show.<id>` | how a word unit is shown, one form per plural category separated by `\|` (`Stunde \| Stunden`); the forms are inputs too |
//! | `prefix.<symbol>` | SI and IEC prefix words (`prefix.c = Zenti`), joined to every phrase of a unit that takes prefixes |
//! | `square`, `cubic` | squared and cubed lengths, `*` standing for any length phrase (`Quadrat*`) |
//! | `show.square`, `show.cubic` | how squared and cubed word units are shown, `*` standing for the unit word; plural forms separated by `\|` like `show.<id>` |
//! | `per.<id>` | one of a unit in the denominator: `per.h = в час` (`100 рублей в час` is RUB/h) |
//! | `filler` | a word between a number and its unit that adds nothing (Romanian `25 de kilograme`) |
//! | `date-word` | a word joining day, month and year in date literals (`12 de março de 2026`) |
//! | `ordinal` | a suffix written onto the day of a date literal (`1er mai`, `March 1st`) |
//! | `hour-mark` | a letter between hours and two-digit minutes in a time of day (`14h30`, `14 h 30`); alone after a number it stays the hour unit (`14h`) |
//! | `prose` | a word that means nothing to the calculator, listed so the numerals inside it stay prose (`万一`, `一些`) |
//! | `numeral.<value>` | the characters of a numeral system written without digits, each one character: digits (`numeral.3 = 三`), the multipliers 10, 100 and 1000 inside a group of four (`numeral.10 = 十`) and the group multipliers from 10^4 (`numeral.1e4 = 万`); `numeral.point` is the decimal point (`五点五`) |
//! | `currency.<CODE>` | currency names |
//! | `zone.<Area/City>` | city names for a time zone: `zone.Europe/Vienna = Wien` |
//! | `month.<1-12>` | month names in date literals (`12. März 2026`) |
//! | `weekdays`, `months`, `date` | display: seven short weekday names from Monday, twelve short month names, and the date pattern (`%a`, `%-d`, `%d`, `%b`, `%m`, `%Y`) |
//! | `money` | display: where a currency sign goes around the amount, `{amount}` and `{sign}` standing for the parts (`{amount} {sign}` gives `7,20 €`); the default is `{sign}{amount}` |
//!
//! Plural categories follow the language (CLDR): one form for Chinese, Japanese, Korean and
//! Turkish; `one | other` for most languages; `one | few | many | other` for Russian, Ukrainian,
//! Polish and Czech; `one | few | other` for Bosnian and Romanian; `one | two | other` for
//! Hebrew. A single form is used for every count.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::OnceLock;

use chrono::{Datelike, NaiveDate};
use chrono_tz::Tz;

use crate::classify::PercentOp;
use crate::format::Radix;
use crate::functions::Builtin;
use crate::numerals::Numeral;
use crate::units::{self, BUILTINS, Style, is_length};
use crate::{InputLanguage, NumberStyle, currency, lexer, zones};

/// A word with a fixed role, named after its English counterpart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keyword {
    Plus,
    Minus,
    Times,
    Divide,
    /// Division that also accepts a bare unit after it, like `per`: `km pro Stunde`.
    Per,
    Modulo,
    Convert,
    Percent,
    /// A percent word before its number: Chinese `百分之20`.
    PercentBefore,
    Of,
    On,
    Off,
    Prev,
    Sum,
    Average,
    Now,
    Local,
    Noon,
    Midnight,
    OClock,
    /// A conversion word written after its target (Korean `유로로`, Japanese `ユーロに`).
    ConvertAfter,
}

/// What a phrase of an input language can mean.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Meaning {
    Keyword(Keyword),
    /// Assignment after a variable name.
    Assign,
    /// A word between a number and its unit that means nothing (Romanian `25 de kilograme`).
    Filler,
    /// A word joining the parts of a date literal (`12 de março de 2026`).
    DateWord,
    /// A suffix written onto the day of a date literal (`1er mai`, `March 1st`).
    Ordinal,
    /// A letter between hours and minutes in a time of day (`14h30`).
    HourMark,
    /// A word that is prose; listed so the numerals inside it stay prose (`万一`).
    Prose,
    /// Before or after noon, written before the hour (`下午3点`, `오후 3시`).
    HalfDay(HalfDay),
    /// Half past, after an o'clock word (`3点半`).
    HalfHour,
    /// A unit written before its number (`摄氏100度`, `大さじ3`).
    UnitBefore(usize),
    /// `as a % of`, `as a % on`, `as a % off` in one phrase.
    AsPercent(PercentOp),
    /// `of what is`, `on what is`, `off what is` in one phrase.
    PercentWhat(PercentOp),
    /// A day relative to today: `heute` (0), `morgen` (1), `vorgestern` (-2).
    Day(i64),
    Scale(f64),
    Function(Builtin),
    Radix(Radix),
    /// A built-in unit, possibly squared or cubed.
    Unit {
        index: usize,
        power: i8,
    },
    Currency(&'static str),
    Zone(Tz),
    Month(u32),
}

/// Morning or afternoon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HalfDay {
    Am,
    Pm,
}

/// How a language picks the form of a word after a number (CLDR plural rules, cardinals).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Plural {
    /// One form for every count.
    Invariant,
    /// `one` for exactly 1.
    OneOther,
    /// `one` below 2 (`1,5 heure`): French and Brazilian Portuguese.
    ZeroOne,
    /// Danish: `one` for 1 and for fractions below 2 (`1,5 time`).
    Danish,
    /// Russian and Ukrainian: 1, 21 / 2-4, 22 / 5-20, 25 / fractions.
    EastSlavic,
    /// Polish: 1 / 2-4, 22 / 5-21, 25 / fractions.
    Polish,
    /// Czech: 1 / 2-4 / 5 and more / fractions (`many`, listed last).
    Czech,
    /// Bosnian: 1, 21 / 2-4, 22 / the rest.
    Bosnian,
    /// Romanian: 1 / 0, fractions and 2-19 after the hundreds / the rest (`20 de ore`).
    Romanian,
    /// Hebrew: 1 / 2 / the rest.
    Hebrew,
}

/// A plural category; data files list forms in [`Plural::categories`] order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Category {
    One,
    Two,
    Few,
    Many,
    Other,
}

impl Plural {
    fn of(language: InputLanguage) -> Self {
        use InputLanguage::*;
        match language {
            Japanese | Korean | ChineseSimplified | ChineseTraditional | Turkish => Self::Invariant,
            French | PortugueseBrazil => Self::ZeroOne,
            Danish => Self::Danish,
            Russian | Ukrainian => Self::EastSlavic,
            Polish => Self::Polish,
            Czech => Self::Czech,
            Bosnian => Self::Bosnian,
            Romanian => Self::Romanian,
            Hebrew => Self::Hebrew,
            _ => Self::OneOther,
        }
    }

    /// The categories in the order data files list their forms.
    pub(crate) fn categories(self) -> &'static [Category] {
        use Category::*;
        match self {
            Self::Invariant => &[Other],
            Self::OneOther | Self::ZeroOne | Self::Danish => &[One, Other],
            Self::EastSlavic | Self::Polish => &[One, Few, Many, Other],
            Self::Czech => &[One, Few, Other, Many],
            Self::Bosnian | Self::Romanian => &[One, Few, Other],
            Self::Hebrew => &[One, Two, Other],
        }
    }

    /// The category of a formatted number (`"1"`, `"1,5"`, `"1,234.5"`).
    pub(crate) fn category(self, number: &str, style: NumberStyle) -> Category {
        let decimal = match style {
            NumberStyle::German | NumberStyle::French => ',',
            NumberStyle::English | NumberStyle::Swiss | NumberStyle::Plain => '.',
        };
        let digits = number.trim_start_matches('-');
        let (int, frac) = digits.split_once(decimal).unwrap_or((digits, ""));
        let plain = int
            .chars()
            .all(|c| c.is_ascii_digit() || ",.' \u{a0}\u{202f}".contains(c))
            && frac.chars().all(|c| c.is_ascii_digit());
        let i: u64 = int
            .chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap_or(u64::MAX);
        let digits = if !frac.is_empty() || !plain {
            Digits::Fraction
        } else {
            Digits::Whole
        };
        self.category_of(i, digits)
    }

    fn category_of(self, i: u64, digits: Digits) -> Category {
        use Category::*;
        let (last, last_two) = (i % 10, i % 100);
        let teen = (12..=14).contains(&last_two);
        let whole = digits == Digits::Whole;
        match self {
            Self::ZeroOne if i <= 1 => One,
            Self::Danish if i == 1 || (!whole && i == 0) => One,
            Self::EastSlavic | Self::Bosnian if whole && last == 1 && last_two != 11 => One,
            Self::OneOther | Self::Polish | Self::Czech | Self::Romanian | Self::Hebrew
                if whole && i == 1 =>
            {
                One
            }
            Self::Hebrew if whole && i == 2 => Two,
            Self::EastSlavic | Self::Polish | Self::Bosnian if whole && (2..=4).contains(&last) && !teen => {
                Few
            }
            Self::Czech if whole && (2..=4).contains(&i) => Few,
            Self::Romanian if !whole || i == 0 || (1..=19).contains(&last_two) => Few,
            Self::EastSlavic | Self::Polish if whole => Many,
            Self::Czech if !whole => Many,
            _ => Other,
        }
    }
}

/// Whether a formatted number shows decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Digits {
    Whole,
    Fraction,
}

/// The vocabulary of one language, ready for lookups.
#[derive(Debug, Default)]
pub(crate) struct Vocab {
    /// Phrases of one or two characters, exact case.
    exact: HashMap<String, Vec<Meaning>>,
    /// Longer phrases, lowercase.
    folded: HashMap<String, Vec<Meaning>>,
    /// The most raw tokens of a phrase starting with a given (lowercase) token.
    starts: HashMap<String, usize>,
    /// Characters of the language's numerals written without digits (`三`, `百`, `万`).
    numerals: HashMap<char, Numeral>,
    plural: Option<Plural>,
    /// Display forms of word units by unit index, in plural category order.
    shows: HashMap<usize, Vec<String>>,
    show_square: Option<Vec<String>>,
    show_cubic: Option<Vec<String>>,
    weekdays: Option<Vec<String>>,
    months: Option<Vec<String>>,
    date: Option<String>,
    /// Where the currency sign goes, `{amount}` and `{sign}` standing for the parts.
    money: Option<String>,
}

/// Collapses whitespace and unifies apostrophes, the way phrases are compared.
fn normalize(phrase: &str) -> String {
    phrase
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('’', "'")
}

/// `phrase` as [`normalize`] would make it, borrowed when it already is (the common case).
fn normal(phrase: &str) -> Cow<'_, str> {
    let already = !phrase.contains(['’', '\t', '\n', '\r', '\u{a0}'])
        && !phrase.contains("  ")
        && !phrase.starts_with(' ')
        && !phrase.ends_with(' ');
    if already {
        Cow::Borrowed(phrase)
    } else {
        Cow::Owned(normalize(phrase))
    }
}

/// `text` in lowercase, borrowed when it has no uppercase letters.
fn lower(text: &str) -> Cow<'_, str> {
    if text.chars().any(char::is_uppercase) {
        Cow::Owned(text.to_lowercase())
    } else {
        Cow::Borrowed(text)
    }
}

/// Whether a normalized phrase matches its exact case only.
fn exact_case(phrase: &str) -> bool {
    phrase.chars().count() <= 2
}

impl Vocab {
    /// What a phrase (any whitespace, any case for longer phrases) means, in priority order.
    pub(crate) fn get(&self, phrase: &str) -> &[Meaning] {
        let key = normal(phrase);
        let found = if exact_case(&key) {
            self.exact.get(key.as_ref())
        } else {
            self.folded.get(lower(&key).as_ref())
        };
        found.map_or(&[], Vec::as_slice)
    }

    /// How many raw tokens the longest phrase starting with `first` has (0 when none does).
    pub(crate) fn window(&self, first: &str) -> usize {
        self.starts.get(lower(first).as_ref()).copied().unwrap_or(0)
    }

    /// What `c` is in the language's numerals written without digits.
    pub(crate) fn numeral(&self, c: char) -> Option<Numeral> {
        self.numerals.get(&c).copied()
    }

    /// The form for the formatted `number` among forms listed in plural category order.
    fn form<'a>(&self, forms: &'a [String], number: &str, style: NumberStyle) -> Option<&'a str> {
        let plural = self.plural?;
        let category = plural.category(number, style);
        let position = plural
            .categories()
            .iter()
            .position(|c| *c == category)
            .unwrap_or(0);
        forms.get(position).or_else(|| forms.first()).map(String::as_str)
    }

    /// The word for unit `index` after the formatted `number`, when the language has one.
    pub(crate) fn unit_word(&self, index: usize, number: &str, style: NumberStyle) -> Option<&str> {
        self.form(self.shows.get(&index)?, number, style)
    }

    /// How squared (2) or cubed (3) word units are shown after `number`, `*` standing for the
    /// word (`* quadrado` / `* quadrados`).
    pub(crate) fn power_template(&self, power: i8, number: &str, style: NumberStyle) -> Option<&str> {
        let forms = match power {
            2 => self.show_square.as_deref()?,
            3 => self.show_cubic.as_deref()?,
            _ => return None,
        };
        self.form(forms, number, style)
    }

    /// The short weekday name of `date`.
    pub(crate) fn weekday(&self, date: NaiveDate) -> Option<&str> {
        let names = self.weekdays.as_ref()?;
        let index = usize::try_from(date.weekday().num_days_from_monday()).ok()?;
        names.get(index).map(String::as_str)
    }

    /// The language's placement of a currency sign around a formatted amount (`{amount} {sign}`).
    pub(crate) fn money(&self) -> Option<&str> {
        self.money.as_deref()
    }

    /// `date` in the language's pattern, when it has one.
    pub(crate) fn date(&self, date: NaiveDate) -> Option<String> {
        let pattern = self.date.as_deref()?;
        let month = usize::try_from(date.month0()).ok()?;
        let month_name = self.months.as_ref().and_then(|names| names.get(month));
        let weekday = self.weekday(date);
        let mut out = String::with_capacity(pattern.len() + 16);
        let mut chars = pattern.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            let unpadded = chars.peek() == Some(&'-');
            if unpadded {
                chars.next();
            }
            match chars.next() {
                Some('a') => out.push_str(weekday?),
                Some('b') => out.push_str(month_name?),
                Some('d') if unpadded => out.push_str(&date.day().to_string()),
                Some('d') => {
                    let _ = write!(out, "{:02}", date.day());
                }
                Some('m') if unpadded => out.push_str(&date.month().to_string()),
                Some('m') => {
                    let _ = write!(out, "{:02}", date.month());
                }
                Some('Y') => out.push_str(&date.year().to_string()),
                Some(other) => {
                    out.push('%');
                    out.push(other);
                }
                None => out.push('%'),
            }
        }
        Some(out)
    }
}

/// The selected language's vocabulary on top of English's extra phrases: what the classifier
/// consults.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lexicon {
    primary: &'static Vocab,
    english: Option<&'static Vocab>,
}

impl Lexicon {
    pub(crate) fn new(language: InputLanguage) -> Self {
        let english = vocab(InputLanguage::English);
        if language == InputLanguage::English {
            Self {
                primary: english,
                english: None,
            }
        } else {
            Self {
                primary: vocab(language),
                english: Some(english),
            }
        }
    }

    /// What a phrase means: the selected language's meanings first, then English's.
    pub(crate) fn meanings(&self, phrase: &str) -> Cow<'static, [Meaning]> {
        let primary: &'static [Meaning] = self.primary.get(phrase);
        let english: &'static [Meaning] = self.english.map_or(&[], |vocab| vocab.get(phrase));
        if english.is_empty() {
            return Cow::Borrowed(primary);
        }
        if primary.is_empty() {
            return Cow::Borrowed(english);
        }
        let mut out = primary.to_vec();
        out.extend(english.iter().filter(|m| !primary.contains(m)));
        Cow::Owned(out)
    }

    /// The most raw tokens a phrase starting with `first` can have.
    pub(crate) fn window(&self, first: &str) -> usize {
        let english = self.english.map_or(0, |vocab| vocab.window(first));
        self.primary.window(first).max(english)
    }

    /// Whether the language writes numbers without digits too (`三百五十`).
    pub(crate) fn has_numerals(&self) -> bool {
        !self.primary.numerals.is_empty()
    }

    /// What `c` is in the language's numerals written without digits; English has none.
    pub(crate) fn numeral(&self, c: char) -> Option<Numeral> {
        self.primary.numeral(c)
    }
}

/// The data file and sample note of every language, in [`InputLanguage::ALL`] order.
const SOURCES: [(&str, &str, &str); 28] = [
    (
        "en",
        include_str!("../lang/en.txt"),
        include_str!("../lang/en.numi"),
    ),
    (
        "af",
        include_str!("../lang/af.txt"),
        include_str!("../lang/af.numi"),
    ),
    (
        "bs-BA",
        include_str!("../lang/bs-BA.txt"),
        include_str!("../lang/bs-BA.numi"),
    ),
    (
        "cs",
        include_str!("../lang/cs.txt"),
        include_str!("../lang/cs.numi"),
    ),
    (
        "da",
        include_str!("../lang/da.txt"),
        include_str!("../lang/da.numi"),
    ),
    (
        "de",
        include_str!("../lang/de.txt"),
        include_str!("../lang/de.numi"),
    ),
    (
        "de-AT",
        include_str!("../lang/de-AT.txt"),
        include_str!("../lang/de-AT.numi"),
    ),
    (
        "de-CH",
        include_str!("../lang/de-CH.txt"),
        include_str!("../lang/de-CH.numi"),
    ),
    (
        "es",
        include_str!("../lang/es.txt"),
        include_str!("../lang/es.numi"),
    ),
    (
        "fi",
        include_str!("../lang/fi.txt"),
        include_str!("../lang/fi.numi"),
    ),
    (
        "fr",
        include_str!("../lang/fr.txt"),
        include_str!("../lang/fr.numi"),
    ),
    (
        "he",
        include_str!("../lang/he.txt"),
        include_str!("../lang/he.numi"),
    ),
    (
        "it",
        include_str!("../lang/it.txt"),
        include_str!("../lang/it.numi"),
    ),
    (
        "ja",
        include_str!("../lang/ja.txt"),
        include_str!("../lang/ja.numi"),
    ),
    (
        "ko",
        include_str!("../lang/ko.txt"),
        include_str!("../lang/ko.numi"),
    ),
    (
        "nl",
        include_str!("../lang/nl.txt"),
        include_str!("../lang/nl.numi"),
    ),
    (
        "nl-BE",
        include_str!("../lang/nl-BE.txt"),
        include_str!("../lang/nl-BE.numi"),
    ),
    (
        "no",
        include_str!("../lang/no.txt"),
        include_str!("../lang/no.numi"),
    ),
    (
        "pl",
        include_str!("../lang/pl.txt"),
        include_str!("../lang/pl.numi"),
    ),
    (
        "pt-BR",
        include_str!("../lang/pt-BR.txt"),
        include_str!("../lang/pt-BR.numi"),
    ),
    (
        "pt-PT",
        include_str!("../lang/pt-PT.txt"),
        include_str!("../lang/pt-PT.numi"),
    ),
    (
        "ro",
        include_str!("../lang/ro.txt"),
        include_str!("../lang/ro.numi"),
    ),
    (
        "ru",
        include_str!("../lang/ru.txt"),
        include_str!("../lang/ru.numi"),
    ),
    (
        "sv",
        include_str!("../lang/sv.txt"),
        include_str!("../lang/sv.numi"),
    ),
    (
        "tr",
        include_str!("../lang/tr.txt"),
        include_str!("../lang/tr.numi"),
    ),
    (
        "uk",
        include_str!("../lang/uk.txt"),
        include_str!("../lang/uk.numi"),
    ),
    (
        "zh-Hans",
        include_str!("../lang/zh-Hans.txt"),
        include_str!("../lang/zh-Hans.numi"),
    ),
    (
        "zh-Hant",
        include_str!("../lang/zh-Hant.txt"),
        include_str!("../lang/zh-Hant.numi"),
    ),
];

static VOCABS: [OnceLock<Vocab>; 28] = [const { OnceLock::new() }; 28];

/// The vocabulary of `language`, built on first use.
///
/// # Panics
///
/// When the language's data file is malformed, which the test suite rules out.
pub(crate) fn vocab(language: InputLanguage) -> &'static Vocab {
    VOCABS[language.index()].get_or_init(|| {
        build(language).unwrap_or_else(|error| panic!("lang/{}.txt: {error}", language.tag()))
    })
}

/// Builds a vocabulary, reporting the first malformed entry.
pub(crate) fn build(language: InputLanguage) -> Result<Vocab, String> {
    let mut builder = Builder::default();
    builder.load(language.tag(), 0)?;
    let mut vocab = builder.finish();
    vocab.plural = Some(Plural::of(language));
    let categories = Plural::of(language).categories().len();
    for forms in vocab.show_square.iter().chain(&vocab.show_cubic) {
        if forms.len() != 1 && forms.len() != categories {
            return Err(format!(
                "show.square or show.cubic has {} forms, the language has {categories} plural categories",
                forms.len()
            ));
        }
    }
    for (index, forms) in &vocab.shows {
        if forms.len() != 1 && forms.len() != categories {
            return Err(format!(
                "show.{} has {} forms, the language has {categories} plural categories",
                BUILTINS.def(*index).id,
                forms.len()
            ));
        }
    }
    Ok(vocab)
}

/// The sample note of `language`.
pub(crate) fn sample(language: InputLanguage) -> &'static str {
    SOURCES[language.index()].2
}

/// Splits `value` at commas outside `{}` and expands every phrase.
fn phrases(value: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut depth = 0_u32;
    let mut current = String::new();
    for c in value.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.checked_sub(1).ok_or("unbalanced `}`")?,
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if depth != 0 {
        return Err("unbalanced `{`".to_owned());
    }
    parts.push(current);
    let mut out = Vec::new();
    for part in parts {
        for phrase in expand(&part) {
            let phrase = normalize(&phrase);
            if !phrase.is_empty() && !out.contains(&phrase) {
                out.push(phrase);
            }
        }
    }
    Ok(out)
}

/// Expands `{a,b}` blocks (`Stunde{,n}` → `Stunde`, `Stunden`).
fn expand(phrase: &str) -> Vec<String> {
    let Some(open) = phrase.find('{') else {
        return vec![phrase.to_owned()];
    };
    let Some(close) = phrase[open..].find('}').map(|c| c + open) else {
        return vec![phrase.to_owned()];
    };
    let (head, tail) = (&phrase[..open], &phrase[close + 1..]);
    let tails = expand(tail);
    phrase[open + 1..close]
        .split(',')
        .flat_map(|alternative| tails.iter().map(move |rest| format!("{head}{alternative}{rest}")))
        .collect()
}

/// A built-in unit id, optionally with a power suffix: `km` → (km, 1), `cm3` → (cm, 3).
fn unit_ref(id: &str) -> Option<(usize, i8)> {
    if let Some(index) = BUILTINS.by_id(id) {
        return Some((index, 1));
    }
    let power = match id.chars().last()? {
        '2' => 2,
        '3' => 3,
        _ => return None,
    };
    BUILTINS.by_id(&id[..id.len() - 1]).map(|index| (index, power))
}

/// Collects a language's entries before they become lookup tables.
#[derive(Debug, Default)]
struct Builder {
    phrases: Vec<(String, Meaning)>,
    /// Unit phrases at power 1, for prefixes and templates.
    unit_phrases: Vec<(usize, String)>,
    prefixes: Vec<(&'static str, String)>,
    square: Vec<String>,
    cubic: Vec<String>,
    shows: HashMap<usize, Vec<String>>,
    show_square: Option<Vec<String>>,
    show_cubic: Option<Vec<String>>,
    weekdays: Option<Vec<String>>,
    months: Option<Vec<String>>,
    date: Option<String>,
    money: Option<String>,
    numerals: HashMap<char, Numeral>,
}

/// The farthest day word from today (`day.2 = übermorgen`).
const MAX_DAY_OFFSET: i64 = 7;

/// The deepest chain of `base` entries.
const MAX_BASE_DEPTH: usize = 3;

impl Builder {
    fn load(&mut self, tag: &str, depth: usize) -> Result<(), String> {
        if depth > MAX_BASE_DEPTH {
            return Err("`base` entries form a loop".to_owned());
        }
        let source = SOURCES
            .iter()
            .find(|(t, ..)| *t == tag)
            .map(|(_, data, _)| *data)
            .ok_or_else(|| format!("no data file for `{tag}`"))?;
        let mut entries: Vec<(usize, String, String)> = Vec::new();
        for (number, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if line.starts_with(char::is_whitespace) {
                let (_, _, value) = entries
                    .last_mut()
                    .ok_or_else(|| format!("line {}: continuation without an entry", number + 1))?;
                value.push(' ');
                value.push_str(trimmed);
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| format!("line {}: expected `key = value`", number + 1))?;
            entries.push((number + 1, key.trim().to_owned(), value.trim().to_owned()));
        }
        for (line, key, value) in entries {
            if key == "base" {
                self.load(&value, depth + 1)?;
                continue;
            }
            self.entry(&key, &value)
                .map_err(|error| format!("line {line} (`{key}`): {error}"))?;
        }
        Ok(())
    }

    fn add(&mut self, value: &str, meaning: Meaning) -> Result<(), String> {
        for phrase in phrases(value)? {
            self.phrases.push((phrase, meaning));
        }
        Ok(())
    }

    fn entry(&mut self, key: &str, value: &str) -> Result<(), String> {
        let keyword = |k| Meaning::Keyword(k);
        let simple = match key {
            "plus" => Some(keyword(Keyword::Plus)),
            "minus" => Some(keyword(Keyword::Minus)),
            "times" => Some(keyword(Keyword::Times)),
            "divide" => Some(keyword(Keyword::Divide)),
            "per" => Some(keyword(Keyword::Per)),
            "modulo" => Some(keyword(Keyword::Modulo)),
            "convert" => Some(keyword(Keyword::Convert)),
            "assign" => Some(Meaning::Assign),
            "percent" => Some(keyword(Keyword::Percent)),
            "percent.before" => Some(keyword(Keyword::PercentBefore)),
            "prev" => Some(keyword(Keyword::Prev)),
            "sum" => Some(keyword(Keyword::Sum)),
            "average" => Some(keyword(Keyword::Average)),
            "today" => Some(Meaning::Day(0)),
            "tomorrow" => Some(Meaning::Day(1)),
            "yesterday" => Some(Meaning::Day(-1)),
            "now" => Some(keyword(Keyword::Now)),
            "local" => Some(keyword(Keyword::Local)),
            "noon" => Some(keyword(Keyword::Noon)),
            "midnight" => Some(keyword(Keyword::Midnight)),
            "oclock" => Some(keyword(Keyword::OClock)),
            "convert-after" => Some(keyword(Keyword::ConvertAfter)),
            "am" => Some(Meaning::HalfDay(HalfDay::Am)),
            "pm" => Some(Meaning::HalfDay(HalfDay::Pm)),
            "half-hour" => Some(Meaning::HalfHour),
            "ordinal" => Some(Meaning::Ordinal),
            "hour-mark" => Some(Meaning::HourMark),
            "prose" => Some(Meaning::Prose),
            _ => None,
        };
        if let Some(meaning) = simple {
            return self.add(value, meaning);
        }
        match key {
            "square" => self.square.extend(phrases(value)?),
            "cubic" => self.cubic.extend(phrases(value)?),
            "show.square" => self.show_square = Some(forms(value)?),
            "show.cubic" => self.show_cubic = Some(forms(value)?),
            "filler" => self.add(value, Meaning::Filler)?,
            "date-word" => self.add(value, Meaning::DateWord)?,
            "weekdays" => self.weekdays = Some(names(value, 7)?),
            "months" => self.months = Some(names(value, 12)?),
            "date" => self.date = Some(value.to_owned()),
            "money" if value.contains("{amount}") && value.contains("{sign}") => {
                self.money = Some(value.to_owned());
            }
            _ => return self.keyed(key, value),
        }
        Ok(())
    }

    /// `kind.argument = ...` entries.
    fn keyed(&mut self, key: &str, value: &str) -> Result<(), String> {
        let (kind, argument) = key.split_once('.').ok_or("unknown key")?;
        let percent_op = || match argument {
            "of" => Ok(PercentOp::Of),
            "on" => Ok(PercentOp::On),
            "off" => Ok(PercentOp::Off),
            _ => Err(format!("expected `of`, `on` or `off` after `{kind}.`")),
        };
        let meaning = match kind {
            "percent" => Meaning::Keyword(match percent_op()? {
                PercentOp::Of => Keyword::Of,
                PercentOp::On => Keyword::On,
                PercentOp::Off => Keyword::Off,
            }),
            "as-percent" => Meaning::AsPercent(percent_op()?),
            "what-is" => Meaning::PercentWhat(percent_op()?),
            "scale" => Meaning::Scale(
                argument
                    .parse::<f64>()
                    .ok()
                    .filter(|f| f.is_finite() && *f > 1.0)
                    .ok_or("expected a factor such as `1e3`")?,
            ),
            "day" => Meaning::Day(
                argument
                    .parse::<i64>()
                    .ok()
                    .filter(|d| d.abs() <= MAX_DAY_OFFSET)
                    .ok_or("expected a day offset such as `2` or `-2`")?,
            ),
            "function" => Meaning::Function(Builtin::from_name(argument).ok_or("unknown function")?),
            "format" => Meaning::Radix(Radix::from_word(argument).ok_or("unknown format")?),
            "currency" => Meaning::Currency(currency::canonical(argument).ok_or("unknown currency code")?),
            "zone" => Meaning::Zone(zones::iana(argument).ok_or("unknown time zone")?),
            "month" => Meaning::Month(
                argument
                    .parse::<u32>()
                    .ok()
                    .filter(|m| (1..=12).contains(m))
                    .ok_or("expected a month number from 1 to 12")?,
            ),
            "unit" => {
                let (index, power) = unit_ref(argument).ok_or("unknown unit id")?;
                for phrase in phrases(value)? {
                    if power == 1 {
                        self.unit_phrases.push((index, phrase.clone()));
                    }
                    self.phrases.push((phrase, Meaning::Unit { index, power }));
                }
                return Ok(());
            }
            "show" => return self.show(argument, value),
            "numeral" => return self.numeral(argument, value),
            "unit-before" => Meaning::UnitBefore(BUILTINS.by_id(argument).ok_or("unknown unit id")?),
            "per" => {
                let index = BUILTINS.by_id(argument).ok_or("unknown unit id")?;
                Meaning::Unit { index, power: -1 }
            }
            "prefix" => {
                let (symbol, _) = units::prefix_names()
                    .find(|(symbol, _)| *symbol == argument)
                    .ok_or("unknown prefix symbol")?;
                for phrase in phrases(value)? {
                    self.prefixes.push((symbol, phrase));
                }
                return Ok(());
            }
            _ => return Err("unknown key".to_owned()),
        };
        self.add(value, meaning)
    }

    /// `numeral.3 = 三`, `numeral.1e4 = 万`, `numeral.point = 点`.
    fn numeral(&mut self, argument: &str, value: &str) -> Result<(), String> {
        let numeral = Numeral::from_key(argument)
            .ok_or("expected a digit, 10, 100, 1000, a power of ten from 1e4 or `point`")?;
        for phrase in phrases(value)? {
            let mut chars = phrase.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else {
                return Err(format!("`{phrase}` is not one character"));
            };
            self.numerals.insert(c, numeral);
        }
        Ok(())
    }

    fn show(&mut self, id: &str, value: &str) -> Result<(), String> {
        let index = BUILTINS.by_id(id).ok_or("unknown unit id")?;
        if BUILTINS.def(index).style != Style::Word {
            return Err("only word units (time, imperial) are shown as words".to_owned());
        }
        let forms = forms(value)?;
        for form in &forms {
            self.unit_phrases.push((index, form.clone()));
            self.phrases
                .push((form.clone(), Meaning::Unit { index, power: 1 }));
        }
        self.shows.insert(index, forms);
        Ok(())
    }

    fn finish(mut self) -> Vocab {
        self.expand_prefixes();
        self.expand_templates();
        let mut vocab = Vocab {
            shows: self.shows,
            show_square: self.show_square,
            show_cubic: self.show_cubic,
            weekdays: self.weekdays,
            months: self.months,
            date: self.date,
            money: self.money,
            numerals: self.numerals,
            ..Vocab::default()
        };
        for (phrase, meaning) in self.phrases {
            let raws = lexer::lex(&phrase, 0, NumberStyle::English, lexer::DashedDates::DayFirst);
            if let Some(first) = raws.first() {
                let start = phrase[first.range.clone()].to_lowercase();
                let count = vocab.starts.entry(start).or_insert(0);
                *count = (*count).max(raws.len());
            }
            let meanings = if exact_case(&phrase) {
                vocab.exact.entry(phrase).or_default()
            } else {
                vocab.folded.entry(phrase.to_lowercase()).or_default()
            };
            if !meanings.contains(&meaning) {
                meanings.push(meaning);
            }
        }
        vocab
    }

    /// `Zenti` + `Meter`: every prefix word with every phrase of a unit that takes prefixes.
    fn expand_prefixes(&mut self) {
        let mut added = Vec::new();
        for (symbol, english) in units::prefix_names() {
            let words: Vec<&str> = english
                .iter()
                .copied()
                .chain(
                    self.prefixes
                        .iter()
                        .filter(|(s, _)| *s == symbol)
                        .map(|(_, word)| word.as_str()),
                )
                .collect();
            for (base, phrase) in &self.unit_phrases {
                let Some(index) = BUILTINS.prefixed(*base, symbol) else {
                    continue;
                };
                for word in &words {
                    added.push((index, format!("{word}{phrase}")));
                }
            }
        }
        for (index, phrase) in added {
            self.phrases
                .push((phrase.clone(), Meaning::Unit { index, power: 1 }));
            self.unit_phrases.push((index, phrase));
        }
    }

    /// `Quadrat*`, `Kubik*` over every length phrase and length symbol.
    fn expand_templates(&mut self) {
        if self.square.is_empty() && self.cubic.is_empty() {
            return;
        }
        let lengths: Vec<(usize, String)> = self
            .unit_phrases
            .iter()
            .filter(|(index, _)| is_length(*index))
            .cloned()
            .chain(
                BUILTINS
                    .length_symbols()
                    .map(|(symbol, index)| (index, symbol.to_owned())),
            )
            .collect();
        for (templates, power) in [(&self.square, 2), (&self.cubic, 3)] {
            for template in templates {
                for (index, length) in &lengths {
                    self.phrases.push((
                        normalize(&template.replace('*', length)),
                        Meaning::Unit { index: *index, power },
                    ));
                }
            }
        }
    }
}

/// Display forms separated by `|`, one per plural category (or one for all).
fn forms(value: &str) -> Result<Vec<String>, String> {
    let forms: Vec<String> = value.split('|').map(normalize).collect();
    if forms.iter().any(String::is_empty) {
        return Err("empty form".to_owned());
    }
    Ok(forms)
}

/// A fixed-size list of display names, separated by commas.
fn names(value: &str, count: usize) -> Result<Vec<String>, String> {
    let list: Vec<String> = value.split(',').map(normalize).collect();
    if list.len() != count || list.iter().any(String::is_empty) {
        return Err(format!("expected {count} names"));
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_loads() {
        for language in InputLanguage::ALL {
            assert_eq!(SOURCES[language.index()].0, language.tag());
            if let Err(error) = build(language) {
                panic!("lang/{}.txt: {error}", language.tag());
            }
        }
    }

    #[test]
    fn braces_expand_and_commas_split() {
        assert_eq!(
            phrases("Stunde{,n}, Std{,.}").expect("valid"),
            ["Stunde", "Stunden", "Std", "Std."]
        );
        assert_eq!(
            phrases("a{b,c} d{e,f}").expect("valid"),
            ["ab de", "ab df", "ac de", "ac df"]
        );
        assert!(phrases("a{b").is_err());
    }

    #[test]
    fn plural_categories() {
        use Category::*;
        let ru = Plural::EastSlavic;
        let cases = [
            ("1", One),
            ("2", Few),
            ("5", Many),
            ("11", Many),
            ("21", One),
            ("1,5", Other),
        ];
        for (number, want) in cases {
            assert_eq!(ru.category(number, NumberStyle::German), want, "{number}");
        }
        assert_eq!(Plural::OneOther.category("1", NumberStyle::English), One);
        assert_eq!(Plural::OneOther.category("1.5", NumberStyle::English), Other);
        assert_eq!(Plural::OneOther.category("1,000", NumberStyle::English), Other);
        assert_eq!(Plural::ZeroOne.category("1,5", NumberStyle::German), One);
        assert_eq!(Plural::Polish.category("22", NumberStyle::English), Few);
        assert_eq!(Plural::Polish.category("12", NumberStyle::English), Many);
        assert_eq!(Plural::Czech.category("2.5", NumberStyle::English), Many);
        assert_eq!(Plural::Romanian.category("20", NumberStyle::English), Other);
        assert_eq!(Plural::Romanian.category("19", NumberStyle::English), Few);
    }
}
