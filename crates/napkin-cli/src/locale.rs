//! The input language for `--language auto`: `$NAPKIN_LANGUAGE`, else the system's preferred
//! language (the same choice as the app's "Automatic"), else English.

use napkin_core::InputLanguage;

/// Overrides the system language for `--language auto` (a tag or locale such as `de_DE`).
const OVERRIDE: &str = "NAPKIN_LANGUAGE";

/// The language `auto` stands for.
pub(crate) fn automatic() -> InputLanguage {
    std::env::var(OVERRIDE)
        .ok()
        .and_then(|tag| parse(&tag))
        .or_else(|| {
            preferred_languages()
                .iter()
                .find_map(|tag| InputLanguage::from_locale(tag))
        })
        .unwrap_or_default()
}

/// A tag (`de-CH`) or the closest language for a locale (`de_DE.UTF-8`).
pub(crate) fn parse(text: &str) -> Option<InputLanguage> {
    text.parse().ok().or_else(|| InputLanguage::from_locale(text))
}

/// The user's preferred languages, most preferred first (`de-DE`, `en-US`).
#[cfg(target_os = "macos")]
fn preferred_languages() -> Vec<String> {
    objc2_foundation::NSLocale::preferredLanguages()
        .iter()
        .map(|tag| tag.to_string())
        .collect()
}

/// The user's preferred languages from the POSIX locale variables.
#[cfg(not(target_os = "macos"))]
fn preferred_languages() -> Vec<String> {
    ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .flat_map(|value| value.split(':').map(str::to_owned).collect::<Vec<_>>())
        .filter(|tag| !tag.is_empty() && tag != "C" && tag != "POSIX")
        .collect()
}
