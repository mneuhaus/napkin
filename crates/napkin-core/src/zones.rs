//! Time zones: IANA names, city names, curated aliases and abbreviations.

use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{DateTime, Local, NaiveDateTime, TimeDelta, TimeZone, Utc};
use chrono_tz::{TZ_VARIANTS, Tz};

/// A time zone a moment is shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Zone {
    /// A named IANA zone.
    Named(Tz),
    /// The system's local zone ([`crate::Clock::System`]).
    Local,
}

impl Zone {
    /// The wall-clock date and time of `instant` in this zone.
    pub(crate) fn naive(self, instant: DateTime<Utc>) -> NaiveDateTime {
        match self {
            Self::Named(tz) => instant.with_timezone(&tz).naive_local(),
            Self::Local => instant.with_timezone(&Local).naive_local(),
        }
    }

    /// The instant a wall-clock time in this zone names (the earlier one when ambiguous, an hour
    /// later when the time falls into a daylight-saving gap).
    pub(crate) fn instant(self, naive: NaiveDateTime) -> Option<DateTime<Utc>> {
        let resolve = |naive: NaiveDateTime| match self {
            Self::Named(tz) => tz
                .from_local_datetime(&naive)
                .earliest()
                .map(|t| t.with_timezone(&Utc)),
            Self::Local => Local
                .from_local_datetime(&naive)
                .earliest()
                .map(|t| t.with_timezone(&Utc)),
        };
        resolve(naive).or_else(|| resolve(naive + TimeDelta::hours(1)))
    }
}

/// Areas whose zone names end in a city.
const CITY_AREAS: &[&str] = &[
    "Africa",
    "America",
    "Antarctica",
    "Asia",
    "Atlantic",
    "Australia",
    "Europe",
    "Indian",
    "Pacific",
];

/// Major cities and countries that are not IANA zone names, with the zone they use.
const ALIASES: &[(&str, &str)] = &[
    ("san francisco", "America/Los_Angeles"),
    ("seattle", "America/Los_Angeles"),
    ("portland", "America/Los_Angeles"),
    ("las vegas", "America/Los_Angeles"),
    ("san diego", "America/Los_Angeles"),
    ("san jose", "America/Los_Angeles"),
    ("sacramento", "America/Los_Angeles"),
    ("silicon valley", "America/Los_Angeles"),
    ("california", "America/Los_Angeles"),
    ("washington", "America/New_York"),
    ("washington dc", "America/New_York"),
    ("boston", "America/New_York"),
    ("miami", "America/New_York"),
    ("atlanta", "America/New_York"),
    ("philadelphia", "America/New_York"),
    ("pittsburgh", "America/New_York"),
    ("baltimore", "America/New_York"),
    ("orlando", "America/New_York"),
    ("charlotte", "America/New_York"),
    ("cleveland", "America/New_York"),
    ("nyc", "America/New_York"),
    ("dallas", "America/Chicago"),
    ("houston", "America/Chicago"),
    ("austin", "America/Chicago"),
    ("san antonio", "America/Chicago"),
    ("new orleans", "America/Chicago"),
    ("minneapolis", "America/Chicago"),
    ("st louis", "America/Chicago"),
    ("saint louis", "America/Chicago"),
    ("kansas city", "America/Chicago"),
    ("nashville", "America/Chicago"),
    ("milwaukee", "America/Chicago"),
    ("salt lake city", "America/Denver"),
    ("albuquerque", "America/Denver"),
    ("montreal", "America/Toronto"),
    ("ottawa", "America/Toronto"),
    ("quebec", "America/Toronto"),
    ("calgary", "America/Edmonton"),
    ("munich", "Europe/Berlin"),
    ("münchen", "Europe/Berlin"),
    ("frankfurt", "Europe/Berlin"),
    ("hamburg", "Europe/Berlin"),
    ("cologne", "Europe/Berlin"),
    ("köln", "Europe/Berlin"),
    ("düsseldorf", "Europe/Berlin"),
    ("dusseldorf", "Europe/Berlin"),
    ("stuttgart", "Europe/Berlin"),
    ("leipzig", "Europe/Berlin"),
    ("dresden", "Europe/Berlin"),
    ("hanover", "Europe/Berlin"),
    ("hannover", "Europe/Berlin"),
    ("nuremberg", "Europe/Berlin"),
    ("bonn", "Europe/Berlin"),
    ("bremen", "Europe/Berlin"),
    ("dortmund", "Europe/Berlin"),
    ("essen", "Europe/Berlin"),
    ("bielefeld", "Europe/Berlin"),
    ("gütersloh", "Europe/Berlin"),
    ("germany", "Europe/Berlin"),
    ("barcelona", "Europe/Madrid"),
    ("valencia", "Europe/Madrid"),
    ("seville", "Europe/Madrid"),
    ("spain", "Europe/Madrid"),
    ("milan", "Europe/Rome"),
    ("florence", "Europe/Rome"),
    ("venice", "Europe/Rome"),
    ("naples", "Europe/Rome"),
    ("turin", "Europe/Rome"),
    ("italy", "Europe/Rome"),
    ("geneva", "Europe/Zurich"),
    ("bern", "Europe/Zurich"),
    ("basel", "Europe/Zurich"),
    ("lausanne", "Europe/Zurich"),
    ("switzerland", "Europe/Zurich"),
    ("lyon", "Europe/Paris"),
    ("marseille", "Europe/Paris"),
    ("toulouse", "Europe/Paris"),
    ("bordeaux", "Europe/Paris"),
    ("france", "Europe/Paris"),
    ("manchester", "Europe/London"),
    ("edinburgh", "Europe/London"),
    ("glasgow", "Europe/London"),
    ("birmingham", "Europe/London"),
    ("liverpool", "Europe/London"),
    ("bristol", "Europe/London"),
    ("leeds", "Europe/London"),
    ("cardiff", "Europe/London"),
    ("uk", "Europe/London"),
    ("england", "Europe/London"),
    ("britain", "Europe/London"),
    ("ireland", "Europe/Dublin"),
    ("rotterdam", "Europe/Amsterdam"),
    ("the hague", "Europe/Amsterdam"),
    ("utrecht", "Europe/Amsterdam"),
    ("netherlands", "Europe/Amsterdam"),
    ("antwerp", "Europe/Brussels"),
    ("belgium", "Europe/Brussels"),
    ("porto", "Europe/Lisbon"),
    ("portugal", "Europe/Lisbon"),
    ("krakow", "Europe/Warsaw"),
    ("kraków", "Europe/Warsaw"),
    ("gdansk", "Europe/Warsaw"),
    ("poland", "Europe/Warsaw"),
    ("austria", "Europe/Vienna"),
    ("czechia", "Europe/Prague"),
    ("hungary", "Europe/Budapest"),
    ("greece", "Europe/Athens"),
    ("sweden", "Europe/Stockholm"),
    ("norway", "Europe/Oslo"),
    ("denmark", "Europe/Copenhagen"),
    ("finland", "Europe/Helsinki"),
    ("iceland", "Atlantic/Reykjavik"),
    ("st petersburg", "Europe/Moscow"),
    ("saint petersburg", "Europe/Moscow"),
    ("russia", "Europe/Moscow"),
    ("ukraine", "Europe/Kyiv"),
    ("turkey", "Europe/Istanbul"),
    ("ankara", "Europe/Istanbul"),
    ("mumbai", "Asia/Kolkata"),
    ("bombay", "Asia/Kolkata"),
    ("delhi", "Asia/Kolkata"),
    ("new delhi", "Asia/Kolkata"),
    ("bangalore", "Asia/Kolkata"),
    ("bengaluru", "Asia/Kolkata"),
    ("chennai", "Asia/Kolkata"),
    ("hyderabad", "Asia/Kolkata"),
    ("pune", "Asia/Kolkata"),
    ("india", "Asia/Kolkata"),
    ("beijing", "Asia/Shanghai"),
    ("peking", "Asia/Shanghai"),
    ("guangzhou", "Asia/Shanghai"),
    ("shenzhen", "Asia/Shanghai"),
    ("chengdu", "Asia/Shanghai"),
    ("hangzhou", "Asia/Shanghai"),
    ("wuhan", "Asia/Shanghai"),
    ("china", "Asia/Shanghai"),
    ("osaka", "Asia/Tokyo"),
    ("kyoto", "Asia/Tokyo"),
    ("yokohama", "Asia/Tokyo"),
    ("nagoya", "Asia/Tokyo"),
    ("sapporo", "Asia/Tokyo"),
    ("japan", "Asia/Tokyo"),
    ("busan", "Asia/Seoul"),
    ("korea", "Asia/Seoul"),
    ("south korea", "Asia/Seoul"),
    ("thailand", "Asia/Bangkok"),
    ("hanoi", "Asia/Ho_Chi_Minh"),
    ("saigon", "Asia/Ho_Chi_Minh"),
    ("ho chi minh city", "Asia/Ho_Chi_Minh"),
    ("vietnam", "Asia/Ho_Chi_Minh"),
    ("indonesia", "Asia/Jakarta"),
    ("bali", "Asia/Makassar"),
    ("philippines", "Asia/Manila"),
    ("tel aviv", "Asia/Jerusalem"),
    ("israel", "Asia/Jerusalem"),
    ("abu dhabi", "Asia/Dubai"),
    ("uae", "Asia/Dubai"),
    ("islamabad", "Asia/Karachi"),
    ("lahore", "Asia/Karachi"),
    ("pakistan", "Asia/Karachi"),
    ("egypt", "Africa/Cairo"),
    ("cape town", "Africa/Johannesburg"),
    ("durban", "Africa/Johannesburg"),
    ("south africa", "Africa/Johannesburg"),
    ("canberra", "Australia/Sydney"),
    ("wellington", "Pacific/Auckland"),
    ("christchurch", "Pacific/Auckland"),
    ("new zealand", "Pacific/Auckland"),
    ("rio de janeiro", "America/Sao_Paulo"),
    ("rio", "America/Sao_Paulo"),
    ("brasilia", "America/Sao_Paulo"),
    ("brazil", "America/Sao_Paulo"),
    ("argentina", "America/Argentina/Buenos_Aires"),
    ("buenos aires", "America/Argentina/Buenos_Aires"),
    ("mexico", "America/Mexico_City"),
    ("hawaii", "Pacific/Honolulu"),
    ("alaska", "America/Anchorage"),
];

/// Abbreviations (uppercase only) and the zone whose daylight-saving rules they follow.
const ABBREVIATIONS: &[(&str, &str)] = &[
    ("PST", "America/Los_Angeles"),
    ("PDT", "America/Los_Angeles"),
    ("PT", "America/Los_Angeles"),
    ("MST", "America/Denver"),
    ("MDT", "America/Denver"),
    ("MT", "America/Denver"),
    ("CST", "America/Chicago"),
    ("CDT", "America/Chicago"),
    ("CT", "America/Chicago"),
    ("EST", "America/New_York"),
    ("EDT", "America/New_York"),
    ("ET", "America/New_York"),
    ("AKST", "America/Anchorage"),
    ("AKDT", "America/Anchorage"),
    ("HST", "Pacific/Honolulu"),
    ("AST", "America/Halifax"),
    ("ADT", "America/Halifax"),
    ("NST", "America/St_Johns"),
    ("NDT", "America/St_Johns"),
    ("GMT", "Etc/GMT"),
    ("UTC", "UTC"),
    ("UCT", "UTC"),
    ("WET", "Europe/Lisbon"),
    ("WEST", "Europe/Lisbon"),
    ("BST", "Europe/London"),
    ("CET", "Europe/Berlin"),
    ("CEST", "Europe/Berlin"),
    ("MEZ", "Europe/Berlin"),
    ("MESZ", "Europe/Berlin"),
    ("EET", "Europe/Athens"),
    ("EEST", "Europe/Athens"),
    ("MSK", "Europe/Moscow"),
    ("GST", "Asia/Dubai"),
    ("PKT", "Asia/Karachi"),
    ("IST", "Asia/Kolkata"),
    ("ICT", "Asia/Bangkok"),
    ("WIB", "Asia/Jakarta"),
    ("SGT", "Asia/Singapore"),
    ("HKT", "Asia/Hong_Kong"),
    ("PHT", "Asia/Manila"),
    ("JST", "Asia/Tokyo"),
    ("KST", "Asia/Seoul"),
    ("AWST", "Australia/Perth"),
    ("ACST", "Australia/Adelaide"),
    ("ACDT", "Australia/Adelaide"),
    ("AEST", "Australia/Sydney"),
    ("AEDT", "Australia/Sydney"),
    ("NZST", "Pacific/Auckland"),
    ("NZDT", "Pacific/Auckland"),
    ("BRT", "America/Sao_Paulo"),
    ("ART", "America/Argentina/Buenos_Aires"),
    ("SAST", "Africa/Johannesburg"),
    ("EAT", "Africa/Nairobi"),
    ("WAT", "Africa/Lagos"),
    ("CAT", "Africa/Maputo"),
];

struct Tables {
    /// Lowercase city or alias, words separated by single spaces.
    cities: HashMap<String, Tz>,
    /// Lowercase full IANA name.
    names: HashMap<String, Tz>,
    abbreviations: HashMap<&'static str, Tz>,
}

static TABLES: LazyLock<Tables> = LazyLock::new(|| {
    let mut cities = HashMap::new();
    let mut names = HashMap::new();
    for tz in TZ_VARIANTS {
        let name = tz.name();
        names.insert(name.to_lowercase(), tz);
        let mut parts = name.split('/');
        let area = parts.next().unwrap_or_default();
        if CITY_AREAS.contains(&area)
            && let Some(city) = name.rsplit('/').next()
        {
            cities.entry(fold(&city.replace('_', " "))).or_insert(tz);
        }
    }
    for (alias, zone) in ALIASES {
        if let Ok(tz) = zone.parse::<Tz>() {
            cities.entry(fold(alias)).or_insert(tz);
        }
    }
    let abbreviations = ABBREVIATIONS
        .iter()
        .filter_map(|(abbr, zone)| zone.parse::<Tz>().ok().map(|tz| (*abbr, tz)))
        .collect();
    Tables {
        cities,
        names,
        abbreviations,
    }
});

/// The most words a city name or alias has (`Ho Chi Minh City`, `Salt Lake City`).
pub(crate) const MAX_CITY_WORDS: usize = 4;

/// A city or alias (`Berlin`, `New York`, `São Paulo`), any case and with or without
/// diacritics, words separated by spaces.
pub(crate) fn city(words: &str) -> Option<Tz> {
    TABLES.cities.get(&fold(words)).copied()
}

/// Lowercase without diacritics, so `São Paulo` finds `Sao_Paulo`.
fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => 'a',
            'ç' | 'č' | 'ć' => 'c',
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ě' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'ī' => 'i',
            'ñ' | 'ń' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' => 'u',
            'ý' | 'ÿ' => 'y',
            'ś' | 'š' => 's',
            'ź' | 'ż' | 'ž' => 'z',
            'ł' => 'l',
            other => other,
        })
        .collect()
}

/// A full IANA name (`Europe/Berlin`), any case.
pub(crate) fn iana(name: &str) -> Option<Tz> {
    TABLES.names.get(&name.to_lowercase()).copied()
}

/// An uppercase abbreviation (`PST`, `CET`, `HKT`).
pub(crate) fn abbreviation(word: &str) -> Option<Tz> {
    TABLES.abbreviations.get(word).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_alias_and_abbreviation_names_a_zone() {
        for (alias, zone) in ALIASES.iter().chain(ABBREVIATIONS) {
            assert!(zone.parse::<Tz>().is_ok(), "{alias} -> {zone}");
        }
    }

    #[test]
    fn cities_aliases_and_abbreviations() {
        assert_eq!(city("New York"), Some(Tz::America__New_York));
        assert_eq!(city("hong kong"), Some(Tz::Asia__Hong_Kong));
        assert_eq!(city("San Francisco"), Some(Tz::America__Los_Angeles));
        assert_eq!(city("São Paulo"), Some(Tz::America__Sao_Paulo));
        assert_eq!(city("Köln"), Some(Tz::Europe__Berlin));
        assert_eq!(city("Buenos Aires"), Some(Tz::America__Argentina__Buenos_Aires));
        assert_eq!(iana("europe/berlin"), Some(Tz::Europe__Berlin));
        assert_eq!(abbreviation("PST"), Some(Tz::America__Los_Angeles));
        assert_eq!(abbreviation("pst"), None);
    }

    #[test]
    fn names_fit_the_phrase_window() {
        let longest = TABLES
            .cities
            .keys()
            .map(|k| k.split(' ').count())
            .max()
            .unwrap_or(1);
        assert!(longest <= MAX_CITY_WORDS, "{longest}");
    }
}
