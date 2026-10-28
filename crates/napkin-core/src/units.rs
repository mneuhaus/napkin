//! Built-in units, SI and IEC prefixes, dimensions and compound units.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::LazyLock;

/// A base dimension quantities are measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum BaseDim {
    Length,
    Mass,
    Time,
    Temperature,
    Data,
    Angle,
    Currency,
    /// A dimension introduced by an extension unit that is its own base unit.
    Custom(usize),
}

/// A product of base dimensions with integer exponents, sorted and free of zero exponents.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub(crate) struct Dimension(Vec<(BaseDim, i8)>);

impl Dimension {
    /// A single base dimension raised to `exp`.
    pub(crate) fn of(base: BaseDim, exp: i8) -> Self {
        Self(vec![(base, exp)])
    }

    /// Whether this is the dimension of a plain number.
    pub(crate) fn is_none(&self) -> bool {
        self.0.is_empty()
    }

    /// `self * other^power`.
    pub(crate) fn mul_pow(&self, other: &Self, power: i8) -> Self {
        let mut exps = self.0.clone();
        for &(base, exp) in &other.0 {
            let add = exp.saturating_mul(power);
            if let Some(entry) = exps.iter_mut().find(|(b, _)| *b == base) {
                entry.1 = entry.1.saturating_add(add);
            } else {
                exps.push((base, add));
            }
        }
        exps.retain(|&(_, exp)| exp != 0);
        exps.sort_unstable();
        Self(exps)
    }
}

/// How a unit converts to its SI base unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Scale {
    /// Multiply by the factor to get the base unit.
    Factor(f64),
    /// An absolute temperature scale: `kelvin = (value + offset) * factor`.
    Temperature { factor: f64, offset: f64 },
    /// CSS pixel: `1/ppi` inch.
    Pixel,
    /// CSS em: the document's `em` size in pixels.
    Em,
}

/// How a unit is written after a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
    /// `5 cm`
    Symbol,
    /// `45°`: the symbol follows the number without a space.
    Attached,
    /// `5 inches`: a word, pluralised by value.
    Word,
}

/// A built-in unit.
#[derive(Debug, Clone)]
pub(crate) struct UnitDef {
    /// Stable id, reported as [`crate::Answer::unit`].
    pub(crate) id: String,
    /// Symbol used for symbol-style display and inside compound units.
    pub(crate) symbol: String,
    /// Singular display word for word-style units.
    pub(crate) singular: &'static str,
    /// Plural display word for word-style units.
    pub(crate) plural: &'static str,
    pub(crate) style: Style,
    pub(crate) dim: Dimension,
    pub(crate) scale: Scale,
    /// Whether prefixed forms (`km`, `kB`) exist for this unit.
    pub(crate) prefixable: bool,
    /// The SI or IEC prefix symbol of a prefixed unit (`k` for `km`).
    pub(crate) prefix: Option<&'static str>,
}

/// How a unit reference wants to be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Form {
    /// The unit's own style.
    #[default]
    Default,
    /// The abbreviation the user typed for a word-style unit (`min`, `ft`), kept as written.
    Abbrev(&'static str),
}

/// A single unit, built-in, from an extension, a currency or percent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UnitRef {
    Builtin(usize),
    Custom(usize),
    Currency(String),
    Percent,
}

/// One factor of a compound unit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Factor {
    pub(crate) unit: UnitRef,
    pub(crate) power: i8,
    pub(crate) form: Form,
}

/// A product of units with integer powers; empty for plain numbers.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Unit {
    pub(crate) factors: Vec<Factor>,
}

impl Unit {
    /// A single unit to the first power.
    pub(crate) fn simple(unit: UnitRef, form: Form) -> Self {
        Self {
            factors: vec![Factor { unit, power: 1, form }],
        }
    }

    /// A single unit raised to `power`.
    pub(crate) fn power_of(unit: UnitRef, power: i8, form: Form) -> Self {
        Self {
            factors: vec![Factor { unit, power, form }],
        }
    }

    /// Whether this is a plain number.
    pub(crate) fn is_empty(&self) -> bool {
        self.factors.is_empty()
    }

    /// The single unit to the first power, if that is all this is.
    pub(crate) fn as_simple(&self) -> Option<&UnitRef> {
        match self.factors.as_slice() {
            [Factor { unit, power: 1, .. }] => Some(unit),
            _ => None,
        }
    }

    /// Whether this is exactly percent.
    pub(crate) fn is_percent(&self) -> bool {
        matches!(self.as_simple(), Some(UnitRef::Percent))
    }

    /// The currency code when this is exactly one currency.
    pub(crate) fn currency(&self) -> Option<&str> {
        match self.as_simple() {
            Some(UnitRef::Currency(code)) => Some(code),
            _ => None,
        }
    }

    /// This unit raised to `power`.
    pub(crate) fn pow(&self, power: i8) -> Self {
        let factors = self
            .factors
            .iter()
            .map(|f| Factor {
                power: f.power.saturating_mul(power),
                ..f.clone()
            })
            .filter(|f| f.power != 0)
            .collect();
        Self { factors }
    }

    /// The product of two units, merging equal units.
    pub(crate) fn times(&self, other: &Self) -> Self {
        let mut factors = self.factors.clone();
        for factor in &other.factors {
            if let Some(existing) = factors.iter_mut().find(|f| f.unit == factor.unit) {
                existing.power = existing.power.saturating_add(factor.power);
            } else {
                factors.push(factor.clone());
            }
        }
        factors.retain(|f| f.power != 0);
        Self { factors }
    }

    /// Raises every power by `power` in place.
    pub(crate) fn raise(&mut self, power: i8) {
        *self = self.pow(power);
    }
}

/// An SI or IEC prefix.
struct Prefix {
    /// Symbols; the first one is used for display.
    symbols: &'static [&'static str],
    /// Long names; the first one identifies the prefix.
    names: &'static [&'static str],
    factor: f64,
}

const SI_PREFIXES: &[Prefix] = &[
    Prefix {
        symbols: &["Q"],
        names: &["quetta"],
        factor: 1e30,
    },
    Prefix {
        symbols: &["R"],
        names: &["ronna"],
        factor: 1e27,
    },
    Prefix {
        symbols: &["Y"],
        names: &["yotta"],
        factor: 1e24,
    },
    Prefix {
        symbols: &["Z"],
        names: &["zetta"],
        factor: 1e21,
    },
    Prefix {
        symbols: &["E"],
        names: &["exa"],
        factor: 1e18,
    },
    Prefix {
        symbols: &["P"],
        names: &["peta"],
        factor: 1e15,
    },
    Prefix {
        symbols: &["T"],
        names: &["tera"],
        factor: 1e12,
    },
    Prefix {
        symbols: &["G"],
        names: &["giga"],
        factor: 1e9,
    },
    Prefix {
        symbols: &["M"],
        names: &["mega"],
        factor: 1e6,
    },
    Prefix {
        symbols: &["k"],
        names: &["kilo"],
        factor: 1e3,
    },
    Prefix {
        symbols: &["h"],
        names: &["hecto"],
        factor: 1e2,
    },
    Prefix {
        symbols: &["da"],
        names: &["deca", "deka"],
        factor: 1e1,
    },
    Prefix {
        symbols: &["d"],
        names: &["deci"],
        factor: 1e-1,
    },
    Prefix {
        symbols: &["c"],
        names: &["centi"],
        factor: 1e-2,
    },
    Prefix {
        symbols: &["m"],
        names: &["milli"],
        factor: 1e-3,
    },
    Prefix {
        symbols: &["µ", "μ", "u"],
        names: &["micro"],
        factor: 1e-6,
    },
    Prefix {
        symbols: &["n"],
        names: &["nano"],
        factor: 1e-9,
    },
    Prefix {
        symbols: &["p"],
        names: &["pico"],
        factor: 1e-12,
    },
    Prefix {
        symbols: &["f"],
        names: &["femto"],
        factor: 1e-15,
    },
    Prefix {
        symbols: &["a"],
        names: &["atto"],
        factor: 1e-18,
    },
    Prefix {
        symbols: &["z"],
        names: &["zepto"],
        factor: 1e-21,
    },
    Prefix {
        symbols: &["y"],
        names: &["yocto"],
        factor: 1e-24,
    },
    Prefix {
        symbols: &["r"],
        names: &["ronto"],
        factor: 1e-27,
    },
    Prefix {
        symbols: &["q"],
        names: &["quecto"],
        factor: 1e-30,
    },
];

const IEC_PREFIXES: &[Prefix] = &[
    Prefix {
        symbols: &["Ki"],
        names: &["kibi"],
        factor: 1024.0,
    },
    Prefix {
        symbols: &["Mi"],
        names: &["mebi"],
        factor: 1_048_576.0,
    },
    Prefix {
        symbols: &["Gi"],
        names: &["gibi"],
        factor: 1_073_741_824.0,
    },
    Prefix {
        symbols: &["Ti"],
        names: &["tebi"],
        factor: 1_099_511_627_776.0,
    },
    Prefix {
        symbols: &["Pi"],
        names: &["pebi"],
        factor: 1_125_899_906_842_624.0,
    },
    Prefix {
        symbols: &["Ei"],
        names: &["exbi"],
        factor: 1_152_921_504_606_846_976.0,
    },
];

/// Which prefixes a unit accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prefixes {
    None,
    /// Every SI prefix.
    All,
    /// Kilo and up, plus the binary IEC prefixes (`kB`, `GiB`).
    Data,
}

/// A built-in unit definition before prefixes are expanded.
struct Spec {
    id: &'static str,
    symbol: &'static str,
    words: (&'static str, &'static str),
    style: Style,
    dim: &'static [(BaseDim, i8)],
    scale: Scale,
    /// Case-sensitive symbols; these take short prefixes.
    symbols: &'static [&'static str],
    /// Case-insensitive names (lowercase); these take long prefixes.
    names: &'static [&'static str],
    /// Case-sensitive abbreviations without prefixes; word-style units keep them in display.
    abbrevs: &'static [&'static str],
    prefixes: Prefixes,
}

const LENGTH: &[(BaseDim, i8)] = &[(BaseDim::Length, 1)];
const AREA: &[(BaseDim, i8)] = &[(BaseDim::Length, 2)];
const VOLUME: &[(BaseDim, i8)] = &[(BaseDim::Length, 3)];
const MASS: &[(BaseDim, i8)] = &[(BaseDim::Mass, 1)];
const TIME: &[(BaseDim, i8)] = &[(BaseDim::Time, 1)];
const FREQUENCY: &[(BaseDim, i8)] = &[(BaseDim::Time, -1)];
const SPEED: &[(BaseDim, i8)] = &[(BaseDim::Length, 1), (BaseDim::Time, -1)];
const TEMPERATURE: &[(BaseDim, i8)] = &[(BaseDim::Temperature, 1)];
const ANGLE: &[(BaseDim, i8)] = &[(BaseDim::Angle, 1)];
const DATA: &[(BaseDim, i8)] = &[(BaseDim::Data, 1)];

/// Metres per inch.
const INCH: f64 = 0.0254;
/// Seconds per day.
const DAY: f64 = 86_400.0;
/// Cubic metres per US liquid gallon.
const GALLON: f64 = 3.785_411_784e-3;

const NO_WORDS: (&str, &str) = ("", "");

#[expect(clippy::too_many_lines, reason = "a flat data table reads best in one place")]
fn specs() -> Vec<Spec> {
    use Style::{Attached, Symbol, Word};
    let symbol = |id, symbol, dim, factor, symbols, names, prefixes| Spec {
        id,
        symbol,
        words: NO_WORDS,
        style: Symbol,
        dim,
        scale: Scale::Factor(factor),
        symbols,
        names,
        abbrevs: &[],
        prefixes,
    };
    let word = |id, symbol, words, dim, factor, names, abbrevs| Spec {
        id,
        symbol,
        words,
        style: Word,
        dim,
        scale: Scale::Factor(factor),
        symbols: &[],
        names,
        abbrevs,
        prefixes: Prefixes::None,
    };
    vec![
        // Length.
        symbol(
            "m",
            "m",
            LENGTH,
            1.0,
            &["m"],
            &["meter", "meters", "metre", "metres"],
            Prefixes::All,
        ),
        word(
            "mil",
            "mil",
            ("mil", "mils"),
            LENGTH,
            INCH / 1000.0,
            &["mil", "mils", "thou"],
            &[],
        ),
        symbol(
            "pt",
            "pt",
            LENGTH,
            INCH / 72.0,
            &["pt"],
            &["point", "points"],
            Prefixes::None,
        ),
        word(
            "line",
            "line",
            ("line", "lines"),
            LENGTH,
            INCH / 12.0,
            &["line", "lines"],
            &[],
        ),
        word(
            "inch",
            "in",
            ("inch", "inches"),
            LENGTH,
            INCH,
            &["inch", "inches"],
            &[],
        ),
        word(
            "hand",
            "hand",
            ("hand", "hands"),
            LENGTH,
            4.0 * INCH,
            &["hand", "hands"],
            &[],
        ),
        word(
            "foot",
            "ft",
            ("foot", "feet"),
            LENGTH,
            12.0 * INCH,
            &["foot", "feet"],
            &["ft"],
        ),
        word(
            "yard",
            "yd",
            ("yard", "yards"),
            LENGTH,
            36.0 * INCH,
            &["yard", "yards"],
            &["yd", "yds"],
        ),
        word(
            "rod",
            "rod",
            ("rod", "rods"),
            LENGTH,
            5.0292,
            &["rod", "rods"],
            &[],
        ),
        word(
            "chain",
            "ch",
            ("chain", "chains"),
            LENGTH,
            20.1168,
            &["chain", "chains"],
            &[],
        ),
        word(
            "furlong",
            "fur",
            ("furlong", "furlongs"),
            LENGTH,
            201.168,
            &["furlong", "furlongs"],
            &[],
        ),
        word(
            "mile",
            "mi",
            ("mile", "miles"),
            LENGTH,
            1609.344,
            &["mile", "miles"],
            &["mi"],
        ),
        word(
            "cable",
            "cable",
            ("cable", "cables"),
            LENGTH,
            185.2,
            &["cable", "cables"],
            &[],
        ),
        word(
            "nautical mile",
            "nmi",
            ("nautical mile", "nautical miles"),
            LENGTH,
            1852.0,
            &["nautical mile", "nautical miles"],
            &["nmi"],
        ),
        word(
            "league",
            "lea",
            ("league", "leagues"),
            LENGTH,
            4828.032,
            &["league", "leagues"],
            &[],
        ),
        word(
            "light year",
            "ly",
            ("light year", "light years"),
            LENGTH,
            9.460_730_472_580_8e15,
            &["light year", "light years", "lightyear", "lightyears"],
            &["ly"],
        ),
        symbol(
            "Å",
            "Å",
            LENGTH,
            1e-10,
            &["Å"],
            &["angstrom", "angstroms"],
            Prefixes::None,
        ),
        Spec {
            id: "px",
            symbol: "px",
            words: NO_WORDS,
            style: Symbol,
            dim: LENGTH,
            scale: Scale::Pixel,
            symbols: &["px"],
            names: &["pixel", "pixels"],
            abbrevs: &[],
            prefixes: Prefixes::None,
        },
        Spec {
            id: "em",
            symbol: "em",
            words: NO_WORDS,
            style: Symbol,
            dim: LENGTH,
            scale: Scale::Em,
            symbols: &["em"],
            names: &[],
            abbrevs: &[],
            prefixes: Prefixes::None,
        },
        // Area.
        symbol(
            "ha",
            "ha",
            AREA,
            1e4,
            &["ha"],
            &["hectare", "hectares"],
            Prefixes::None,
        ),
        word("are", "a", ("are", "ares"), AREA, 100.0, &["are", "ares"], &[]),
        word(
            "acre",
            "ac",
            ("acre", "acres"),
            AREA,
            4_046.856_422_4,
            &["acre", "acres"],
            &["ac"],
        ),
        // The East Asian 坪 (tsubo, ping): 400/121 m², a 1.82 m square.
        word(
            "ping",
            "ping",
            ("ping", "pings"),
            AREA,
            400.0 / 121.0,
            &["ping", "pings", "píng", "píngs"],
            &[],
        ),
        // Volume.
        symbol(
            "l",
            "l",
            VOLUME,
            1e-3,
            &["l", "L"],
            &["liter", "liters", "litre", "litres"],
            Prefixes::All,
        ),
        word(
            "pint",
            "pint",
            ("pint", "pints"),
            VOLUME,
            GALLON / 8.0,
            &["pint", "pints"],
            &[],
        ),
        word(
            "quart",
            "qt",
            ("quart", "quarts"),
            VOLUME,
            GALLON / 4.0,
            &["quart", "quarts"],
            &["qt"],
        ),
        word(
            "gallon",
            "gal",
            ("gallon", "gallons"),
            VOLUME,
            GALLON,
            &["gallon", "gallons"],
            &["gal"],
        ),
        word(
            "teaspoon",
            "tsp",
            ("teaspoon", "teaspoons"),
            VOLUME,
            GALLON / 768.0,
            &["teaspoon", "teaspoons", "tea spoon", "tea spoons"],
            &["tsp"],
        ),
        word(
            "tablespoon",
            "tbsp",
            ("tablespoon", "tablespoons"),
            VOLUME,
            GALLON / 256.0,
            &["tablespoon", "tablespoons", "table spoon", "table spoons"],
            &["tbsp", "tbs"],
        ),
        word(
            "cup",
            "cup",
            ("cup", "cups"),
            VOLUME,
            GALLON / 16.0,
            &["cup", "cups"],
            &[],
        ),
        word(
            "fluid ounce",
            "fl oz",
            ("fluid ounce", "fluid ounces"),
            VOLUME,
            GALLON / 128.0,
            &["fluid ounce", "fluid ounces"],
            &["fl oz", "floz"],
        ),
        // Mass.
        symbol(
            "g",
            "g",
            MASS,
            1.0,
            &["g"],
            &["gram", "grams", "gramme", "grammes"],
            Prefixes::All,
        ),
        symbol(
            "t",
            "t",
            MASS,
            1e6,
            &["t"],
            &["tonne", "tonnes", "ton", "tons", "metric ton"],
            Prefixes::None,
        ),
        word(
            "carat",
            "ct",
            ("carat", "carats"),
            MASS,
            0.2,
            &["carat", "carats"],
            &["ct"],
        ),
        word(
            "centner",
            "centner",
            ("centner", "centners"),
            MASS,
            1e5,
            &["centner", "centners"],
            &[],
        ),
        word(
            "pound",
            "lb",
            ("pound", "pounds"),
            MASS,
            453.592_37,
            &["pound", "pounds"],
            &["lb", "lbs"],
        ),
        word(
            "stone",
            "st",
            ("stone", "stones"),
            MASS,
            6_350.293_18,
            &["stone", "stones"],
            &["st"],
        ),
        word(
            "ounce",
            "oz",
            ("ounce", "ounces"),
            MASS,
            28.349_523_125,
            &["ounce", "ounces"],
            &["oz"],
        ),
        // Time.
        Spec {
            id: "s",
            symbol: "s",
            words: ("second", "seconds"),
            style: Word,
            dim: TIME,
            scale: Scale::Factor(1.0),
            symbols: &["s"],
            names: &["second", "seconds"],
            abbrevs: &["sec", "secs"],
            prefixes: Prefixes::All,
        },
        word(
            "min",
            "min",
            ("minute", "minutes"),
            TIME,
            60.0,
            &["minute", "minutes"],
            &["min", "mins"],
        ),
        word(
            "h",
            "h",
            ("hour", "hours"),
            TIME,
            3600.0,
            &["hour", "hours"],
            &["h", "hr", "hrs"],
        ),
        word("day", "d", ("day", "days"), TIME, DAY, &["day", "days"], &["d"]),
        word(
            "week",
            "wk",
            ("week", "weeks"),
            TIME,
            7.0 * DAY,
            &["week", "weeks"],
            &["wk", "wks"],
        ),
        word(
            "month",
            "mo",
            ("month", "months"),
            TIME,
            365.0 * DAY / 12.0,
            &["month", "months"],
            &["mo"],
        ),
        word(
            "year",
            "yr",
            ("year", "years"),
            TIME,
            365.0 * DAY,
            &["year", "years"],
            &["yr", "yrs"],
        ),
        word(
            "fortnight",
            "fortnight",
            ("fortnight", "fortnights"),
            TIME,
            14.0 * DAY,
            &["fortnight", "fortnights"],
            &[],
        ),
        word(
            "decade",
            "decade",
            ("decade", "decades"),
            TIME,
            3650.0 * DAY,
            &["decade", "decades"],
            &[],
        ),
        word(
            "century",
            "century",
            ("century", "centuries"),
            TIME,
            36_500.0 * DAY,
            &["century", "centuries"],
            &[],
        ),
        // Frequency and speed.
        symbol("Hz", "Hz", FREQUENCY, 1.0, &["Hz"], &["hertz"], Prefixes::All),
        symbol("mph", "mph", SPEED, 0.447_04, &["mph"], &[], Prefixes::None),
        symbol(
            "km/h",
            "km/h",
            SPEED,
            1000.0 / 3600.0,
            &["kph", "kmh"],
            &[],
            Prefixes::None,
        ),
        word(
            "knot",
            "kn",
            ("knot", "knots"),
            SPEED,
            1852.0 / 3600.0,
            &["knot", "knots"],
            &["kn", "kt", "kts"],
        ),
        // Temperature.
        Spec {
            id: "K",
            symbol: "K",
            words: NO_WORDS,
            style: Symbol,
            dim: TEMPERATURE,
            scale: Scale::Temperature {
                factor: 1.0,
                offset: 0.0,
            },
            symbols: &["K"],
            names: &["kelvin", "kelvins"],
            abbrevs: &[],
            prefixes: Prefixes::All,
        },
        Spec {
            id: "°C",
            symbol: "°C",
            words: NO_WORDS,
            style: Symbol,
            dim: TEMPERATURE,
            scale: Scale::Temperature {
                factor: 1.0,
                offset: 273.15,
            },
            symbols: &["°C", "C", "℃", "ºC"],
            names: &[
                "celsius",
                "degc",
                "centigrade",
                "degrees celsius",
                "degree celsius",
            ],
            abbrevs: &[],
            prefixes: Prefixes::None,
        },
        Spec {
            id: "°F",
            symbol: "°F",
            words: NO_WORDS,
            style: Symbol,
            dim: TEMPERATURE,
            scale: Scale::Temperature {
                factor: 5.0 / 9.0,
                offset: 459.67,
            },
            symbols: &["°F", "F", "℉", "ºF"],
            names: &["fahrenheit", "degf", "degrees fahrenheit", "degree fahrenheit"],
            abbrevs: &[],
            prefixes: Prefixes::None,
        },
        // Angle.
        symbol(
            "rad",
            "rad",
            ANGLE,
            1.0,
            &["rad"],
            &["radian", "radians", "rads"],
            Prefixes::All,
        ),
        Spec {
            id: "°",
            symbol: "°",
            words: NO_WORDS,
            style: Attached,
            dim: ANGLE,
            scale: Scale::Factor(PI / 180.0),
            symbols: &["°", "º"],
            names: &["degree", "degrees", "deg", "degs"],
            abbrevs: &[],
            prefixes: Prefixes::None,
        },
        // Data.
        symbol("b", "b", DATA, 1.0, &["b"], &["bit", "bits"], Prefixes::Data),
        symbol("B", "B", DATA, 8.0, &["B"], &["byte", "bytes"], Prefixes::Data),
    ]
}

/// How a looked-up word or symbol names a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hit {
    pub(crate) index: usize,
    pub(crate) form: Form,
}

/// The table of built-in units with lookup by symbol and by name.
#[derive(Debug)]
pub(crate) struct Builtins {
    pub(crate) defs: Vec<UnitDef>,
    by_symbol: HashMap<String, Hit>,
    by_name: HashMap<String, Hit>,
    by_id: HashMap<String, usize>,
    /// The longest name in words, for phrase matching.
    pub(crate) max_words: usize,
}

impl Builtins {
    fn build() -> Self {
        let mut table = Self {
            defs: Vec::new(),
            by_symbol: HashMap::new(),
            by_name: HashMap::new(),
            by_id: HashMap::new(),
            max_words: 1,
        };
        for spec in specs() {
            table.add_spec(&spec);
        }
        table.by_symbol.insert("KB".to_owned(), table.by_symbol["kB"]);
        table.by_symbol.insert("mcg".to_owned(), table.by_symbol["µg"]);
        // German `qm` (Quadratmeter) means m², not quectometres.
        table.by_symbol.remove("qm");
        table.by_symbol.insert("Kb".to_owned(), table.by_symbol["kb"]);
        table.max_words = table
            .by_name
            .keys()
            .chain(table.by_symbol.keys())
            .map(|k| k.split(' ').count())
            .max()
            .unwrap_or(1);
        table
    }

    fn push(&mut self, def: UnitDef) -> usize {
        let index = self.defs.len();
        self.by_id.insert(def.id.clone(), index);
        self.defs.push(def);
        index
    }

    fn add_spec(&mut self, spec: &Spec) {
        let base = self.push(UnitDef {
            id: spec.id.to_owned(),
            symbol: spec.symbol.to_owned(),
            singular: spec.words.0,
            plural: spec.words.1,
            style: spec.style,
            dim: dims(spec.dim),
            scale: spec.scale,
            prefixable: spec.prefixes != Prefixes::None,
            prefix: None,
        });
        let word_style = spec.style == Style::Word;
        for &symbol in spec.symbols {
            let form = if word_style {
                Form::Abbrev(symbol)
            } else {
                Form::Default
            };
            self.by_symbol
                .insert(symbol.to_owned(), Hit { index: base, form });
        }
        for &abbrev in spec.abbrevs {
            self.by_symbol.insert(
                abbrev.to_owned(),
                Hit {
                    index: base,
                    form: Form::Abbrev(abbrev),
                },
            );
        }
        for &name in spec.names {
            self.by_name.insert(
                name.to_owned(),
                Hit {
                    index: base,
                    form: Form::Default,
                },
            );
        }
        if spec.prefixes == Prefixes::All {
            for prefix in SI_PREFIXES {
                self.add_prefixed(spec, prefix);
            }
        }
        if spec.prefixes == Prefixes::Data {
            let large = SI_PREFIXES.iter().filter(|p| p.factor >= 1e3);
            for prefix in large.chain(IEC_PREFIXES) {
                self.add_prefixed(spec, prefix);
            }
        }
    }

    fn add_prefixed(&mut self, spec: &Spec, prefix: &Prefix) {
        let scale = match spec.scale {
            Scale::Factor(factor) => Scale::Factor(factor * prefix.factor),
            Scale::Temperature { factor, offset: 0.0 } => Scale::Temperature {
                factor: factor * prefix.factor,
                offset: 0.0,
            },
            Scale::Temperature { .. } | Scale::Pixel | Scale::Em => return,
        };
        let symbol = format!("{}{}", prefix.symbols[0], spec.symbol);
        let index = self.push(UnitDef {
            id: symbol.clone(),
            symbol,
            singular: "",
            plural: "",
            style: Style::Symbol,
            dim: dims(spec.dim),
            scale,
            prefixable: false,
            prefix: Some(prefix.symbols[0]),
        });
        let hit = Hit {
            index,
            form: Form::Default,
        };
        for prefix_symbol in prefix.symbols {
            for base in spec.symbols {
                self.by_symbol
                    .entry(format!("{prefix_symbol}{base}"))
                    .or_insert(hit);
            }
        }
        for prefix_name in prefix.names {
            for name in spec.names {
                self.by_name.entry(format!("{prefix_name}{name}")).or_insert(hit);
            }
        }
    }

    /// Looks up a word or symbol: symbols case-sensitively, names case-insensitively.
    pub(crate) fn lookup(&self, text: &str) -> Option<Hit> {
        self.by_symbol
            .get(text)
            .copied()
            .or_else(|| self.by_name.get(&text.to_lowercase()).copied())
    }

    /// Looks up a case-sensitive symbol or abbreviation only (`m`, `ft`), not a name.
    pub(crate) fn symbol(&self, text: &str) -> Option<Hit> {
        self.by_symbol.get(text).copied()
    }

    /// Every symbol and abbreviation of a length unit (`m`, `km`, `ft`), with its unit.
    pub(crate) fn length_symbols(&self) -> impl Iterator<Item = (&str, usize)> {
        self.by_symbol
            .iter()
            .filter(|(_, hit)| is_length(hit.index))
            .map(|(symbol, hit)| (symbol.as_str(), hit.index))
    }

    /// The unit `base` with an SI or IEC prefix (`k` + metre = `km`), when that unit exists.
    pub(crate) fn prefixed(&self, base: usize, prefix_symbol: &str) -> Option<usize> {
        let def = self.defs.get(base)?;
        if !def.prefixable {
            return None;
        }
        let prefix = SI_PREFIXES
            .iter()
            .chain(IEC_PREFIXES)
            .find(|p| p.symbols.contains(&prefix_symbol))?;
        self.by_id(&format!("{}{}", prefix.symbols[0], def.symbol))
    }

    /// Looks up a unit by its id.
    pub(crate) fn by_id(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    /// The definition at `index`.
    pub(crate) fn def(&self, index: usize) -> &UnitDef {
        &self.defs[index]
    }
}

fn dims(spec: &[(BaseDim, i8)]) -> Dimension {
    spec.iter().fold(Dimension::default(), |acc, &(base, exp)| {
        acc.mul_pow(&Dimension::of(base, exp), 1)
    })
}

/// Every SI and IEC prefix as its display symbol and long English names (`k`, `["kilo"]`).
pub(crate) fn prefix_names() -> impl Iterator<Item = (&'static str, &'static [&'static str])> {
    SI_PREFIXES
        .iter()
        .chain(IEC_PREFIXES)
        .map(|prefix| (prefix.symbols[0], prefix.names))
}

/// The built-in unit table, built on first use.
pub(crate) static BUILTINS: LazyLock<Builtins> = LazyLock::new(Builtins::build);

/// Whether a built-in unit measures length to the first power (for `sq`/`cubic` prefixes).
pub(crate) fn is_length(index: usize) -> bool {
    BUILTINS.def(index).dim == Dimension::of(BaseDim::Length, 1)
}

/// Unicode superscript for a small exponent, `^n` otherwise.
pub(crate) fn superscript(power: i8) -> String {
    match power {
        1 => String::new(),
        2 => "²".to_owned(),
        3 => "³".to_owned(),
        -1 => "⁻¹".to_owned(),
        -2 => "⁻²".to_owned(),
        -3 => "⁻³".to_owned(),
        other => format!("^{other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_expand_case_sensitively() {
        let km = BUILTINS.lookup("km").expect("km");
        assert_eq!(BUILTINS.def(km.index).symbol, "km");
        let mb = BUILTINS.lookup("MB").expect("MB");
        assert_eq!(BUILTINS.def(mb.index).scale, Scale::Factor(8e6));
        let mbit = BUILTINS.lookup("Mb").expect("Mb");
        assert_eq!(BUILTINS.def(mbit.index).scale, Scale::Factor(1e6));
        assert!(BUILTINS.lookup("mB").is_none(), "milli-bytes are not a thing");
        let gib = BUILTINS.lookup("GiB").expect("GiB");
        assert_eq!(
            BUILTINS.def(gib.index).scale,
            Scale::Factor(8.0 * 1_073_741_824.0)
        );
        assert_eq!(
            BUILTINS
                .lookup("picometers")
                .map(|h| BUILTINS.def(h.index).symbol.clone()),
            Some("pm".into())
        );
    }

    #[test]
    fn names_are_case_insensitive_symbols_are_not() {
        assert!(BUILTINS.lookup("Meters").is_some());
        assert!(BUILTINS.lookup("K").is_some());
        assert!(BUILTINS.lookup("k").is_none());
        assert!(BUILTINS.lookup("M").is_none());
        assert!(BUILTINS.lookup("in").is_none());
    }

    #[test]
    fn dimensions_multiply() {
        let length = Dimension::of(BaseDim::Length, 1);
        let area = length.mul_pow(&length, 1);
        assert_eq!(area, Dimension::of(BaseDim::Length, 2));
        assert!(area.mul_pow(&length, -2).is_none());
    }
}
