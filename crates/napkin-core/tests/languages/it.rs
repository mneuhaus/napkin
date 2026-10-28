//! Italian: `it`.

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const ITALIAN: Table = &[
    // Operators and conversion (`per` multiplies between numbers, divides before a unit)
    (
        &[
            "3 per 4",
            "8 diviso 2",
            "8 diviso per 2",
            "3 fratto 4",
            "7 moltiplicato per 3",
        ],
        &[Text("12"), Text("4"), Text("4"), Text("0,75"), Text("21")],
    ),
    (
        &["5 più 3", "5 e 3", "10 meno 3", "10 senza 4", "5 € con 2 €", "e"],
        &[
            Text("8"),
            Text("8"),
            Text("7"),
            Text("6"),
            Text("7 €"),
            Text("2,72"),
        ],
    ),
    (
        &["5 pollici in cm", "5 pollici a cm", "1 miglio convertito in km"],
        &[Text("12,7 cm"), Text("12,7 cm"), Text("1,61 km")],
    ),
    (
        &["60 km per ora", "120 km / 2 ore"],
        &[Text("60 km/h"), Text("60 km/h")],
    ),
    (
        &[
            "x è 5",
            "x per 2",
            "affitto è uguale a 800 €",
            "affitto diviso 2",
            "y vale 3",
        ],
        &[Text("5"), Text("10"), Text("800 €"), Text("400 €"), Text("3")],
    ),
    // Percentages
    (
        &[
            "20 percento di 50",
            "20 % di 50",
            "5 % su 30 €",
            "6 % di sconto su 40 €",
        ],
        &[Text("10"), Text("10"), Text("31,50 €"), Text("37,60 €")],
    ),
    (
        &["10 % in più su 50", "10 % in meno su 50", "100 € più 22 %"],
        &[Text("55"), Text("45"), Text("122 €")],
    ),
    (
        &[
            "25 come % di 50",
            "75 come % su 50",
            "25 come % di sconto di 50",
            "0,25 in %",
        ],
        &[Text("50%"), Text("50%"), Text("50%"), Text("25%")],
    ),
    (
        &[
            "20 % di un numero fa 10",
            "25 % di aumento fa 125",
            "20 % di sconto fa 80",
        ],
        &[Text("50"), Text("100"), Text("100")],
    ),
    // Scales (m stays metre)
    (
        &[
            "2 milioni",
            "3 miliardi",
            "1,5 mln €",
            "4 mila",
            "2 mille",
            "2 mille miliardi",
            "2 m",
        ],
        &[
            Text("2.000.000"),
            Text("3.000.000.000"),
            Text("1.500.000 €"),
            Text("4.000"),
            Text("2.000"),
            Text("2.000.000.000.000"),
            Text("2 m"),
        ],
    ),
    // Dates and times
    (
        &[
            "oggi",
            "oggi + 2 settimane",
            "domani",
            "ieri",
            "dopodomani",
            "l'altro ieri",
        ],
        &[
            Text("mer 28 ott 2026"),
            Text("mer 11 nov 2026"),
            Text("gio 29 ott 2026"),
            Text("mar 27 ott 2026"),
            Text("ven 30 ott 2026"),
            Text("lun 26 ott 2026"),
        ],
    ),
    (
        &["adesso", "ora corrente", "mezzogiorno + 2 ore", "mezzanotte"],
        &[Text("13:00"), Text("13:00"), Text("14:00"), Text("00:00")],
    ),
    (
        &[
            "che ora è a Tokyo",
            "l'ora a Londra",
            "adesso a Mosca",
            "14:00 a Nuova York",
            "che ore sono a Pechino",
            "ora attuale a Vienna",
            "l'ora al Cairo",
        ],
        &[
            Text("21:00"),
            Text("12:00"),
            Text("15:00"),
            Text("09:00"),
            Text("20:00"),
            Text("13:00"),
            Text("15:00"),
        ],
    ),
    (
        &[
            "l'ora a Parigi",
            "l'ora a Lisbona",
            "l'ora a Varsavia",
            "l'ora a Bruxelles",
            "l'ora a Copenaghen",
            "l'ora a Ginevra",
            "l'ora a Monaco di Baviera",
        ],
        &[
            Text("13:00"),
            Text("12:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
        ],
    ),
    (
        &["12 marzo 2026", "3 ott 2026", "1 maggio", "2026-12-24 - oggi"],
        &[
            Text("gio 12 mar 2026"),
            Text("sab 3 ott 2026"),
            Text("ven 1 mag 2026"),
            Text("57 giorni"),
        ],
    ),
    // Document references
    (
        &["affitto: 850 €", "luce: 64 €", "somma"],
        &[Text("850 €"), Text("64 €"), Text("914 €")],
    ),
    (
        &["affitto: 850 €", "luce: 64 €", "media"],
        &[Text("850 €"), Text("64 €"), Text("457 €")],
    ),
    (
        &["4 €", "6 €", "media", "precedente * 2", "Totale: totale"],
        &[Text("4 €"), Text("6 €"), Text("5 €"), Text("10 €"), Text("25 €")],
    ),
    // Units and localized output
    (
        &[
            "1 mese in giorni",
            "1 giorno",
            "2 ore",
            "5 pollici",
            "3 piedi",
            "90 minuti in ore",
        ],
        &[
            Text("30,42 giorni"),
            Text("1 giorno"),
            Text("2 ore"),
            Text("5 pollici"),
            Text("3 piedi"),
            Text("1,5 ore"),
        ],
    ),
    (
        &["1 ora 30 minuti", "3 gg", "2 sett", "1 anno", "3 anni", "10 sec"],
        &[
            Text("90 minuti"),
            Text("3 giorni"),
            Text("2 settimane"),
            Text("1 anno"),
            Text("3 anni"),
            Text("10 secondi"),
        ],
    ),
    (
        &[
            "20 metri quadrati",
            "20 mq",
            "3 centimetri cubi",
            "2 piedi quadrati",
            "2 ettari in are",
        ],
        &[
            Text("20 m²"),
            Text("20 m²"),
            Text("3 cm³"),
            Text("2 ft²"),
            Text("200 are"),
        ],
    ),
    (
        &[
            "5 chilometri in miglia",
            "250 grammi in once",
            "2 litri in galloni",
            "3 tazze in millilitri",
            "2 etti in grammi",
        ],
        &[
            Text("3,11 miglia"),
            Text("8,82 once"),
            Text("0,53 galloni"),
            Text("709,76 ml"),
            Text("200 g"),
        ],
    ),
    (
        &[
            "100 gradi Celsius in Fahrenheit",
            "90 gradi in radianti",
            "1 GiB in MB",
            "5 chili in libbre",
        ],
        &[
            Text("212 °F"),
            Text("1,57 rad"),
            Text("1.073,74 MB"),
            Text("11,02 libbre"),
        ],
    ),
    (
        &[
            "5 libbre in kg",
            "2 quintali in kg",
            "3 nodi",
            "1 miglio nautico in km",
        ],
        &[Text("2,27 kg"), Text("200 kg"), Text("3 nodi"), Text("1,85 km")],
    ),
    (
        &[
            "15 € all'ora",
            "100 km all'ora",
            "3 € al chilo",
            "3 milioni di euro",
            "3 milioni d'euro",
            "2 chili di mele",
        ],
        &[
            Text("15 €/h"),
            Text("100 km/h"),
            Text("3 €/kg"),
            Text("3.000.000 €"),
            Text("3.000.000 €"),
            Text("2 kg"),
        ],
    ),
    // Currencies
    (
        &[
            "3 dollari canadesi in euro",
            "5 franchi svizzeri in euro",
            "100 yen in euro",
            "10 dollari in euro",
            "20 € in sterline",
        ],
        &[
            Text("1,92 €"),
            Text("4,44 €"),
            Text("0,53 €"),
            Text("8 €"),
            Text("18,75 £"),
        ],
    ),
    // Functions and formats
    (
        &[
            "radice quadrata 16",
            "radice cubica 27",
            "255 in esadecimale",
            "5 in binario",
        ],
        &[Text("4"), Text("3"), Text("0xFF"), Text("0b101")],
    ),
    // The ordinal day
    (&["1º maggio 2026"], &[Text("ven 1 mag 2026")]),
    // Prose stays prose
    (&["Buongiorno a tutti", "Costa 5 €"], &[Nothing, Text("5 €")]),
];

#[test]
fn italian() {
    assert_table(InputLanguage::Italian, NumberStyle::German, ITALIAN);
}
