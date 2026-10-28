//! Dutch: `nl`, `nl-BE`, and Afrikaans (`af`).

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const DUTCH: Table = &[
    // Operators and conversion
    (
        &["3 maal 4", "8 gedeeld door 2", "7 keer 3", "9 delen door 3"],
        &[Text("12"), Text("4"), Text("21"), Text("3")],
    ),
    (
        &["5 en 3", "5 met 3", "10 minder 3", "10 zonder 4", "5 € met 2 €"],
        &[Text("8"), Text("8"), Text("7"), Text("6"), Text("€ 7")],
    ),
    // `min` subtracts only between two operands and stays minutes otherwise
    (
        &["10 min 3", "10 min", "1 uur 10 min", "x is 4", "x min 1"],
        &[Text("7"), Text("10 min"), Text("70 min"), Text("4"), Text("3")],
    ),
    (
        &["5 inch naar cm", "5 inch in cm", "1 mijl als km", "1 m op cm"],
        &[Text("12,7 cm"), Text("12,7 cm"), Text("1,61 km"), Text("100 cm")],
    ),
    (
        &["60 km per uur", "120 km / 2 uur", "100 km/u"],
        &[Text("60 km/h"), Text("60 km/h"), Text("100 km/h")],
    ),
    (
        &[
            "x is gelijk aan 5",
            "x maal 2",
            "huur is 800 €",
            "huur gedeeld door 2",
        ],
        &[Text("5"), Text("10"), Text("€ 800"), Text("€ 400")],
    ),
    // Percentages
    (
        &[
            "20 procent van 50",
            "20 % van 50",
            "5 % op 30 €",
            "6 % korting op 40 €",
        ],
        &[Text("10"), Text("10"), Text("€ 31,50"), Text("€ 37,60")],
    ),
    (
        &["10 % bij 50", "10 % erbovenop 50", "10 % afgetrokken van 50"],
        &[Text("55"), Text("55"), Text("45")],
    ),
    (
        &[
            "25 als % van 50",
            "75 als % bij 50",
            "25 als % afgetrokken van 50",
            "25 als een percentage van 50",
        ],
        &[Text("50%"), Text("50%"), Text("50%"), Text("50%")],
    ),
    (
        &[
            "20 % van wat is 10",
            "25 % bij wat is 125",
            "20 % afgetrokken wat is 80",
        ],
        &[Text("50"), Text("100"), Text("100")],
    ),
    // Scales (long scale: miljard = 10^9, biljoen = 10^12; m stays metre)
    (
        &[
            "2 miljoen",
            "3 miljard",
            "2 biljoen",
            "4 duizend",
            "1,5 mln €",
            "2 m",
        ],
        &[
            Text("2.000.000"),
            Text("3.000.000.000"),
            Text("2.000.000.000.000"),
            Text("4.000"),
            Text("€ 1.500.000"),
            Text("2 m"),
        ],
    ),
    // Dates and times
    (
        &[
            "vandaag",
            "vandaag + 2 weken",
            "morgen",
            "gisteren",
            "overmorgen",
            "eergisteren",
        ],
        &[
            Text("wo 28 okt. 2026"),
            Text("wo 11 nov. 2026"),
            Text("do 29 okt. 2026"),
            Text("di 27 okt. 2026"),
            Text("vr 30 okt. 2026"),
            Text("ma 26 okt. 2026"),
        ],
    ),
    (
        &[
            "nu",
            "tijd",
            "tijd in Wenen",
            "tijd in Moskou",
            "tijd in Peking",
            "14:00 in Londen",
            "Brussel tijd",
        ],
        &[
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("15:00"),
            Text("20:00"),
            Text("13:00"),
            Text("13:00"),
        ],
    ),
    (
        &[
            "tijd in Lissabon",
            "tijd in Warschau",
            "tijd in Kopenhagen",
            "tijd in Genève",
            "tijd in Keulen",
            "tijd in Parijs",
            "tijd in München",
            "tijd in Den Haag",
        ],
        &[
            Text("12:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
        ],
    ),
    (
        &["12 maart 2026", "3 okt. 2026", "1 mei", "2026-12-24 - vandaag"],
        &[
            Text("do 12 mrt. 2026"),
            Text("za 3 okt. 2026"),
            Text("vr 1 mei 2026"),
            Text("57 dagen"),
        ],
    ),
    // Dates with dashes are day first; spaced or short ones stay subtractions
    (
        &[
            "24-12-2026",
            "1-5-2026",
            "24-12-2026 - vandaag",
            "10-5-3",
            "24 - 12 - 2026",
            "2026-12-24",
            "31-2-2026",
        ],
        &[
            Text("do 24 dec. 2026"),
            Text("vr 1 mei 2026"),
            Text("57 dagen"),
            Text("2"),
            Text("-2.014"),
            Text("do 24 dec. 2026"),
            Text("-1.997"),
        ],
    ),
    // A hyphen between letters joins one word
    (
        &["e-mail: 5 + 3", "x = 7", "x-5", "-5", "5 - -3", "5km-3km"],
        &[
            Text("8"),
            Text("7"),
            Text("2"),
            Text("-5"),
            Text("8"),
            Text("2 km"),
        ],
    ),
    // Document references
    (
        &["huur: 850 €", "stroom: 64 €", "som"],
        &[Text("€ 850"), Text("€ 64"), Text("€ 914")],
    ),
    (
        &["huur: 850 €", "stroom: 64 €", "gemiddelde"],
        &[Text("€ 850"), Text("€ 64"), Text("€ 457")],
    ),
    (
        &["4 €", "6 €", "gem", "vorige * 2", "totaal"],
        &[Text("€ 4"), Text("€ 6"), Text("€ 5"), Text("€ 10"), Text("€ 25")],
    ),
    // Units and localized output
    (
        &[
            "1 maand in dagen",
            "1 dag",
            "2 uur",
            "5 inch",
            "3 voet",
            "90 minuten in uren",
        ],
        &[
            Text("30,42 dagen"),
            Text("1 dag"),
            Text("2 uur"),
            Text("5 inch"),
            Text("3 voet"),
            Text("1,5 uur"),
        ],
    ),
    (
        &["1 seconde", "10 sec", "1 week", "2 weken", "1 jaar", "3 j"],
        &[
            Text("1 seconde"),
            Text("10 seconden"),
            Text("1 week"),
            Text("2 weken"),
            Text("1 jaar"),
            Text("3 jaar"),
        ],
    ),
    (
        &[
            "20 vierkante meter",
            "3 kubieke centimeter",
            "2 vierkante voet",
            "5 kubieke inch",
            "2 hectare in are",
        ],
        &[
            Text("20 m²"),
            Text("3 cm³"),
            Text("2 vierkante voet"),
            Text("5 kubieke inch"),
            Text("200 are"),
        ],
    ),
    (
        &[
            "5 kilometer in mijl",
            "250 gram in ounce",
            "2 liter in gallon",
            "3 koppen in milliliter",
            "2 eetlepels in theelepels",
        ],
        &[
            Text("3,11 mijl"),
            Text("8,82 ounce"),
            Text("0,53 gallon"),
            Text("709,76 ml"),
            Text("6 theelepels"),
        ],
    ),
    (
        &[
            "100 graden Celsius in Fahrenheit",
            "90 graden in radialen",
            "1 GiB in MB",
            "10 knopen in km/u",
        ],
        &[
            Text("212 °F"),
            Text("1,57 rad"),
            Text("1.073,74 MB"),
            Text("18,52 km/h"),
        ],
    ),
    // A Dutch ons is 100 g; pond follows pound (weight, or GBP on money lines)
    (
        &[
            "3 ons in gram",
            "5 kilo in pond",
            "5 pond in kg",
            "5 pond in euro",
            "1 ton in kg",
        ],
        &[
            Text("300 g"),
            Text("11,02 pond"),
            Text("2,27 kg"),
            Text("€ 5,33"),
            Text("1.000 kg"),
        ],
    ),
    // Currencies
    (
        &[
            "$9 in euro",
            "50 dollar naar euro",
            "100 Zwitserse frank in euro",
            "20 pond sterling in euro",
            "1000 yen in euro",
            "3 Canadese dollar",
        ],
        &[
            Text("€ 7,20"),
            Text("€ 40"),
            Text("€ 88,89"),
            Text("€ 21,33"),
            Text("€ 5,33"),
            Text("3 CAD"),
        ],
    ),
    // Functions and formats
    (
        &[
            "vierkantswortel 16",
            "derdemachtswortel 27",
            "255 in hexadecimaal",
            "5 in binair",
        ],
        &[Text("4"), Text("3"), Text("0xFF"), Text("0b101")],
    ),
    // Prose stays prose
    (
        &["Goedemorgen", "Dat kost 5 €", "10 euro voor brood"],
        &[Nothing, Text("€ 5"), Text("€ 10")],
    ),
];

const BELGIAN: Table = &[
    (
        &[
            "2 u",
            "90 minuten in u",
            "tijd in Bergen",
            "vandaag",
            "3 ons in gram",
        ],
        &[
            Text("2 uur"),
            Text("1,5 uur"),
            Text("13:00"),
            Text("wo 28 okt. 2026"),
            Text("300 g"),
        ],
    ),
    // `u` between hours and minutes is a time of day
    (
        &["14u30", "14u30 in Londen", "14 u 30", "14u", "2u"],
        &[
            Text("14:30"),
            Text("13:30"),
            Text("14:30"),
            Text("14 uur"),
            Text("2 uur"),
        ],
    ),
];

const AFRIKAANS: Table = &[
    (
        &[
            "3 maal 4",
            "8 gedeel deur 2",
            "5 en 3",
            "10 sonder 4",
            "5 duim na cm",
        ],
        &[Text("12"), Text("4"), Text("8"), Text("6"), Text("12,7 cm")],
    ),
    (
        &[
            "20 persent van 50",
            "10 % by 50",
            "10 % afgetrek van 50",
            "25 as % van 50",
        ],
        &[Text("10"), Text("55"), Text("45"), Text("50%")],
    ),
    (
        &["vandag", "môre", "gister", "12 Maart 2026", "tyd in Londen"],
        &[
            Text("Wo. 28 Okt. 2026"),
            Text("Do. 29 Okt. 2026"),
            Text("Di. 27 Okt. 2026"),
            Text("Do. 12 Mrt. 2026"),
            Text("12:00"),
        ],
    ),
    (
        &["2 miljoen", "3 miljard", "2 biljoen", "1 dag", "3 dae", "2 ure"],
        &[
            Text("2 000 000"),
            Text("3 000 000 000"),
            Text("2 000 000 000 000"),
            Text("1 dag"),
            Text("3 dae"),
            Text("2 uur"),
        ],
    ),
    (
        &["huur: 8500 rand", "kos: 2000 rand", "som", "vorige / 2"],
        &[
            Text("8 500 ZAR"),
            Text("2 000 ZAR"),
            Text("10 500 ZAR"),
            Text("5 250 ZAR"),
        ],
    ),
    (
        &["$9 in euro", "5 pond in kg", "3 ons in g", "1 GiB in MB"],
        &[
            Text("€7,20"),
            Text("2,27 kg"),
            Text("85,05 g"),
            Text("1 073,74 MB"),
        ],
    ),
];

#[test]
fn dutch() {
    assert_table(InputLanguage::Dutch, NumberStyle::German, DUTCH);
}

#[test]
fn belgian_dutch() {
    assert_table(InputLanguage::DutchBelgium, NumberStyle::German, BELGIAN);
}

#[test]
fn afrikaans() {
    assert_table(InputLanguage::Afrikaans, NumberStyle::French, AFRIKAANS);
}
