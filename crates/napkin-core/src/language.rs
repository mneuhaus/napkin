//! The languages Napkin reads calculations in, besides English, which is always understood.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The language whose words Napkin understands on top of English (`5 Zoll in cm`,
/// `20 Prozent von 50`) and in which it shows word units and dates (`2 Stunden`).
///
/// The set matches the languages of Numi's engine. Serialized as its tag (`"de-CH"`).
///
/// ```
/// use napkin_core::InputLanguage;
///
/// let swiss: InputLanguage = "de-CH".parse()?;
/// assert_eq!(swiss.native_name(), "Deutsch (Schweiz)");
/// assert_eq!(InputLanguage::from_locale("de_DE"), Some(InputLanguage::German));
/// # Ok::<(), napkin_core::ParseInputLanguageError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputLanguage {
    /// English (`en`), the base vocabulary every language builds on.
    #[default]
    #[serde(rename = "en")]
    English,
    /// Afrikaans (`af`).
    #[serde(rename = "af")]
    Afrikaans,
    /// Bosnian (`bs-BA`).
    #[serde(rename = "bs-BA")]
    Bosnian,
    /// Czech (`cs`).
    #[serde(rename = "cs")]
    Czech,
    /// Danish (`da`).
    #[serde(rename = "da")]
    Danish,
    /// German (`de`).
    #[serde(rename = "de")]
    German,
    /// German as written in Austria (`de-AT`).
    #[serde(rename = "de-AT")]
    GermanAustria,
    /// German as written in Switzerland (`de-CH`).
    #[serde(rename = "de-CH")]
    GermanSwitzerland,
    /// Spanish (`es`).
    #[serde(rename = "es")]
    Spanish,
    /// Finnish (`fi`).
    #[serde(rename = "fi")]
    Finnish,
    /// French (`fr`).
    #[serde(rename = "fr")]
    French,
    /// Hebrew (`he`).
    #[serde(rename = "he")]
    Hebrew,
    /// Italian (`it`).
    #[serde(rename = "it")]
    Italian,
    /// Japanese (`ja`).
    #[serde(rename = "ja")]
    Japanese,
    /// Korean (`ko`).
    #[serde(rename = "ko")]
    Korean,
    /// Dutch (`nl`).
    #[serde(rename = "nl")]
    Dutch,
    /// Dutch as written in Belgium (`nl-BE`).
    #[serde(rename = "nl-BE")]
    DutchBelgium,
    /// Norwegian (`no`).
    #[serde(rename = "no")]
    Norwegian,
    /// Polish (`pl`).
    #[serde(rename = "pl")]
    Polish,
    /// Portuguese as written in Brazil (`pt-BR`).
    #[serde(rename = "pt-BR")]
    PortugueseBrazil,
    /// Portuguese as written in Portugal (`pt-PT`).
    #[serde(rename = "pt-PT")]
    PortuguesePortugal,
    /// Romanian (`ro`).
    #[serde(rename = "ro")]
    Romanian,
    /// Russian (`ru`).
    #[serde(rename = "ru")]
    Russian,
    /// Swedish (`sv`).
    #[serde(rename = "sv")]
    Swedish,
    /// Turkish (`tr`).
    #[serde(rename = "tr")]
    Turkish,
    /// Ukrainian (`uk`).
    #[serde(rename = "uk")]
    Ukrainian,
    /// Chinese in simplified characters (`zh-Hans`).
    #[serde(rename = "zh-Hans")]
    ChineseSimplified,
    /// Chinese in traditional characters (`zh-Hant`).
    #[serde(rename = "zh-Hant")]
    ChineseTraditional,
}

/// Tag, native name and English name of every language, in [`InputLanguage::ALL`] order.
const NAMES: [(InputLanguage, &str, &str, &str); 28] = [
    (InputLanguage::English, "en", "English", "English"),
    (InputLanguage::Afrikaans, "af", "Afrikaans", "Afrikaans"),
    (InputLanguage::Bosnian, "bs-BA", "Bosanski", "Bosnian"),
    (InputLanguage::Czech, "cs", "Čeština", "Czech"),
    (InputLanguage::Danish, "da", "Dansk", "Danish"),
    (InputLanguage::German, "de", "Deutsch", "German"),
    (
        InputLanguage::GermanAustria,
        "de-AT",
        "Deutsch (Österreich)",
        "German (Austria)",
    ),
    (
        InputLanguage::GermanSwitzerland,
        "de-CH",
        "Deutsch (Schweiz)",
        "German (Switzerland)",
    ),
    (InputLanguage::Spanish, "es", "Español", "Spanish"),
    (InputLanguage::Finnish, "fi", "Suomi", "Finnish"),
    (InputLanguage::French, "fr", "Français", "French"),
    (InputLanguage::Hebrew, "he", "עברית", "Hebrew"),
    (InputLanguage::Italian, "it", "Italiano", "Italian"),
    (InputLanguage::Japanese, "ja", "日本語", "Japanese"),
    (InputLanguage::Korean, "ko", "한국어", "Korean"),
    (InputLanguage::Dutch, "nl", "Nederlands", "Dutch"),
    (
        InputLanguage::DutchBelgium,
        "nl-BE",
        "Nederlands (België)",
        "Dutch (Belgium)",
    ),
    (InputLanguage::Norwegian, "no", "Norsk", "Norwegian"),
    (InputLanguage::Polish, "pl", "Polski", "Polish"),
    (
        InputLanguage::PortugueseBrazil,
        "pt-BR",
        "Português (Brasil)",
        "Portuguese (Brazil)",
    ),
    (
        InputLanguage::PortuguesePortugal,
        "pt-PT",
        "Português (Portugal)",
        "Portuguese (Portugal)",
    ),
    (InputLanguage::Romanian, "ro", "Română", "Romanian"),
    (InputLanguage::Russian, "ru", "Русский", "Russian"),
    (InputLanguage::Swedish, "sv", "Svenska", "Swedish"),
    (InputLanguage::Turkish, "tr", "Türkçe", "Turkish"),
    (InputLanguage::Ukrainian, "uk", "Українська", "Ukrainian"),
    (
        InputLanguage::ChineseSimplified,
        "zh-Hans",
        "简体中文",
        "Chinese (Simplified)",
    ),
    (
        InputLanguage::ChineseTraditional,
        "zh-Hant",
        "繁體中文",
        "Chinese (Traditional)",
    ),
];

impl InputLanguage {
    /// Every language, English first, then by tag.
    pub const ALL: [Self; 28] = {
        let mut all = [Self::English; 28];
        let mut i = 0;
        while i < NAMES.len() {
            all[i] = NAMES[i].0;
            i += 1;
        }
        all
    };

    fn names(self) -> (&'static str, &'static str, &'static str) {
        let (_, tag, native, english) = NAMES[self.index()];
        (tag, native, english)
    }

    /// The position in [`Self::ALL`].
    pub(crate) fn index(self) -> usize {
        NAMES
            .iter()
            .position(|(language, ..)| *language == self)
            .expect("every language has a row in NAMES")
    }

    /// The BCP 47 tag: `"de"`, `"de-CH"`, `"zh-Hans"`.
    pub fn tag(self) -> &'static str {
        self.names().0
    }

    /// The language's name in itself: `"Deutsch (Schweiz)"`, `"日本語"`.
    pub fn native_name(self) -> &'static str {
        self.names().1
    }

    /// The language's name in English: `"German (Switzerland)"`.
    pub fn english_name(self) -> &'static str {
        self.names().2
    }

    /// The closest language for a system locale or language tag (`de_DE`, `de-AT`,
    /// `zh-Hant-TW`, `nb`, `pt`), or `None` when Napkin has no vocabulary for it.
    ///
    /// Regional variants are picked by region (`de-AT`, `de-CH`/`de-LI`, `nl-BE`, `pt-PT`),
    /// Chinese by script or region (`Hant`, `TW`, `HK`, `MO`); Norwegian Bokmål and Nynorsk map
    /// to `no`, Croatian to Bosnian. A `.UTF-8` or `@modifier` suffix is ignored.
    pub fn from_locale(locale: &str) -> Option<Self> {
        let locale = locale.split(['.', '@']).next().unwrap_or_default();
        let lower = locale.to_ascii_lowercase().replace('_', "-");
        let mut parts = lower.split('-').filter(|part| !part.is_empty());
        let language = parts.next()?;
        let rest: Vec<&str> = parts.collect();
        let has = |part: &str| rest.contains(&part);
        Some(match language {
            "en" => Self::English,
            "af" => Self::Afrikaans,
            "bs" | "hr" => Self::Bosnian,
            "cs" => Self::Czech,
            "da" => Self::Danish,
            "de" if has("at") => Self::GermanAustria,
            "de" if has("ch") || has("li") => Self::GermanSwitzerland,
            "de" => Self::German,
            "es" => Self::Spanish,
            "fi" => Self::Finnish,
            "fr" => Self::French,
            "he" | "iw" => Self::Hebrew,
            "it" => Self::Italian,
            "ja" => Self::Japanese,
            "ko" => Self::Korean,
            "nl" if has("be") => Self::DutchBelgium,
            "nl" => Self::Dutch,
            "no" | "nb" | "nn" => Self::Norwegian,
            "pl" => Self::Polish,
            "pt" if has("pt") => Self::PortuguesePortugal,
            "pt" => Self::PortugueseBrazil,
            "ro" => Self::Romanian,
            "ru" => Self::Russian,
            "sv" => Self::Swedish,
            "tr" => Self::Turkish,
            "uk" => Self::Ukrainian,
            "zh" if has("hant") || has("tw") || has("hk") || has("mo") => Self::ChineseTraditional,
            "zh" => Self::ChineseSimplified,
            _ => return None,
        })
    }

    /// A short welcome note in this language whose calculation lines all evaluate.
    pub fn sample_note(self) -> &'static str {
        crate::vocab::sample(self)
    }
}

impl fmt::Display for InputLanguage {
    /// Writes the tag, which [`FromStr`] reads back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

/// The error of parsing an [`InputLanguage`] from a string that is not one of its tags.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown input language `{input}`; expected a tag such as `de` or `zh-Hans`")]
pub struct ParseInputLanguageError {
    input: String,
}

impl FromStr for InputLanguage {
    type Err = ParseInputLanguageError;

    /// Reads a tag (`de-CH`), in any case and with `_` or `-`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let wanted = text.trim().replace('_', "-");
        Self::ALL
            .into_iter()
            .find(|language| language.tag().eq_ignore_ascii_case(&wanted))
            .ok_or_else(|| ParseInputLanguageError {
                input: text.to_owned(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_round_trip() {
        for language in InputLanguage::ALL {
            assert_eq!(language.tag().parse::<InputLanguage>(), Ok(language));
            assert_eq!(language.to_string(), language.tag());
            let json = serde_json::to_string(&language).expect("serializes");
            assert_eq!(json, format!("\"{}\"", language.tag()));
            assert_eq!(serde_json::from_str::<InputLanguage>(&json).ok(), Some(language));
        }
        assert_eq!(
            "DE_ch".parse::<InputLanguage>(),
            Ok(InputLanguage::GermanSwitzerland)
        );
        assert!("xx".parse::<InputLanguage>().is_err());
    }

    #[test]
    fn locales_map_to_the_closest_language() {
        let cases = [
            ("de_DE.UTF-8", Some(InputLanguage::German)),
            ("de-AT", Some(InputLanguage::GermanAustria)),
            ("de_CH", Some(InputLanguage::GermanSwitzerland)),
            ("en-GB", Some(InputLanguage::English)),
            ("pt", Some(InputLanguage::PortugueseBrazil)),
            ("pt-PT", Some(InputLanguage::PortuguesePortugal)),
            ("zh-Hans-CN", Some(InputLanguage::ChineseSimplified)),
            ("zh-Hant-TW", Some(InputLanguage::ChineseTraditional)),
            ("zh_HK", Some(InputLanguage::ChineseTraditional)),
            ("nb-NO", Some(InputLanguage::Norwegian)),
            ("nl-BE", Some(InputLanguage::DutchBelgium)),
            ("ja-JP", Some(InputLanguage::Japanese)),
            ("hu-HU", None),
            ("", None),
        ];
        for (locale, want) in cases {
            assert_eq!(InputLanguage::from_locale(locale), want, "{locale}");
        }
    }

    #[test]
    fn all_lists_every_language_once() {
        let mut tags: Vec<&str> = InputLanguage::ALL.iter().map(|l| l.tag()).collect();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), InputLanguage::ALL.len());
        for (index, language) in InputLanguage::ALL.into_iter().enumerate() {
            assert_eq!(language.index(), index);
        }
    }
}
