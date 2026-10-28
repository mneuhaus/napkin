//! German: `de`, `de-AT`, `de-CH`.

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Fails, Nothing, Text};
use crate::{Table, assert_table};

const GERMAN: Table = &[
    // Operators and conversion
    (
        &["3 mal 4", "8 geteilt durch 2", "10 durch 4"],
        &[Text("12"), Text("4"), Text("2,5")],
    ),
    (
        &["5 und 3", "10 weniger 3", "10 ohne 4", "5 € mit 2 €"],
        &[Text("8"), Text("7"), Text("6"), Text("7 €")],
    ),
    (
        &["7 multipliziert mit 3", "9 dividiert durch 3"],
        &[Text("21"), Text("3")],
    ),
    (
        &["5 Zoll in cm", "5 Zoll nach cm", "1 Meile als km"],
        &[Text("12,7 cm"), Text("12,7 cm"), Text("1,61 km")],
    ),
    (
        &["60 km pro Stunde", "120 km / 2 Stunden"],
        &[Text("60 km/h"), Text("60 km/h")],
    ),
    (
        &["x ist gleich 5", "x mal 2", "Miete ist 800 €", "Miete durch 2"],
        &[Text("5"), Text("10"), Text("800 €"), Text("400 €")],
    ),
    // Percentages
    (
        &[
            "20 Prozent von 50",
            "20 % von 50",
            "5 % auf 30 €",
            "6 % Rabatt auf 40 €",
        ],
        &[Text("10"), Text("10"), Text("31,50 €"), Text("37,60 €")],
    ),
    (
        &[
            "10 % zusätzlich zu 50",
            "10 % abgezogen von 50",
            "100 € zzgl. 19 %",
        ],
        &[Text("55"), Text("45"), Text("119 €")],
    ),
    (
        &[
            "25 in Prozent von 50",
            "75 in % zuzüglich 50",
            "25 in % abzüglich 50",
        ],
        &[Text("50%"), Text("50%"), Text("50%")],
    ),
    (
        &["20 % von was ist 10", "25 % auf was ist 125", "5 % wovon ist 6 €"],
        &[Text("50"), Text("100"), Text("120 €")],
    ),
    // Scales (m stays metre)
    (
        &[
            "2 Millionen",
            "3 Milliarden",
            "1,5 Mio. €",
            "2 Billionen",
            "4 Tausend",
            "2 m",
        ],
        &[
            Text("2.000.000"),
            Text("3.000.000.000"),
            Text("1.500.000 €"),
            Text("2.000.000.000.000"),
            Text("4.000"),
            Text("2 m"),
        ],
    ),
    // Dates and times
    (
        &["heute", "heute + 2 Wochen", "morgen", "gestern", "übermorgen"],
        &[
            Text("Mi., 28. Okt. 2026"),
            Text("Mi., 11. Nov. 2026"),
            Text("Do., 29. Okt. 2026"),
            Text("Di., 27. Okt. 2026"),
            Text("Fr., 30. Okt. 2026"),
        ],
    ),
    (
        &[
            "jetzt",
            "Uhrzeit",
            "Zeit",
            "14 Uhr",
            "14:30 Uhr",
            "14 Uhr + 2 Stunden",
        ],
        &[
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("14:00"),
            Text("14:30"),
            Text("16:00"),
        ],
    ),
    (
        &[
            "Zeit in Wien",
            "Zeit in Moskau",
            "Zeit in Peking",
            "14 Uhr in New York",
            "Zürich Zeit",
        ],
        &[
            Text("13:00"),
            Text("15:00"),
            Text("20:00"),
            Text("09:00"),
            Text("13:00"),
        ],
    ),
    (
        &["12. März 2026", "3. Okt. 2026", "1. Mai", "2026-12-24 - heute"],
        &[
            Text("Do., 12. März 2026"),
            Text("Sa., 3. Okt. 2026"),
            Text("Fr., 1. Mai 2026"),
            Text("57 Tage"),
        ],
    ),
    (
        &[
            "14 Uhr in Lissabon",
            "Zeit in Rom",
            "Prag Zeit",
            "Zeit in Warschau",
            "Zeit in Brüssel",
            "Zeit in Kopenhagen",
        ],
        &[
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
        ],
    ),
    // Document references
    (
        &["Miete: 850 €", "Strom: 64 €", "Summe"],
        &[Text("850 €"), Text("64 €"), Text("914 €")],
    ),
    (
        &["Miete: 850 €", "Strom: 64 €", "Durchschnitt", "Ø"],
        &[Text("850 €"), Text("64 €"), Text("457 €"), Text("457 €")],
    ),
    (
        &["4 €", "6 €", "Mittelwert", "vorheriges * 2", "Gesamt: insgesamt"],
        &[Text("4 €"), Text("6 €"), Text("5 €"), Text("10 €"), Text("25 €")],
    ),
    // Units and localized output
    (
        &[
            "1 Monat in Tagen",
            "1 Tag",
            "2 Stunden",
            "5 Zoll",
            "3 Fuß",
            "90 Minuten in Stunden",
        ],
        &[
            Text("30,42 Tage"),
            Text("1 Tag"),
            Text("2 Stunden"),
            Text("5 Zoll"),
            Text("3 Fuß"),
            Text("1,5 Stunden"),
        ],
    ),
    (
        &["1 Stunde 30 Minuten", "3 Tg.", "2 Wo.", "1 J", "2 Std", "10 Sek"],
        &[
            Text("90 Minuten"),
            Text("3 Tage"),
            Text("2 Wochen"),
            Text("1 Jahr"),
            Text("2 Stunden"),
            Text("10 Sekunden"),
        ],
    ),
    (
        &[
            "20 Quadratmeter",
            "3 Kubikzentimeter",
            "2 Quadratfuß",
            "5 Kubikzoll",
        ],
        &[
            Text("20 m²"),
            Text("3 cm³"),
            Text("2 Quadratfuß"),
            Text("5 Kubikzoll"),
        ],
    ),
    (
        &[
            "5 Kilometer in Meilen",
            "250 Gramm in Unzen",
            "2 Liter in Gallonen",
            "3 Tassen in Milliliter",
        ],
        &[
            Text("3,11 Meilen"),
            Text("8,82 Unzen"),
            Text("0,53 Gallonen"),
            Text("709,76 ml"),
        ],
    ),
    (
        &[
            "100 Grad Celsius in Fahrenheit",
            "90 Grad in Radiant",
            "1 GiB in MB",
            "5 Kilo in Pfund",
        ],
        &[
            Text("212 °F"),
            Text("1,57 rad"),
            Text("1.073,74 MB"),
            Text("11,02 Pfund"),
        ],
    ),
    (
        &[
            "5 Pfund in kg",
            "5 Pfund in Euro",
            "£5 + 3 Pfund",
            "2 Zentner in kg",
        ],
        &[Text("2,27 kg"), Text("5,33 €"), Text("8 £"), Fails],
    ),
    // Currency names
    (
        &[
            "5 Schweizer Franken in Euro",
            "3 US-Dollar",
            "10 britische Pfund in Euro",
            "100 kanadische Dollar in Euro",
        ],
        &[Text("4,44 €"), Text("3 $"), Text("10,67 €"), Text("64 €")],
    ),
    (
        &[
            "1000 japanische Yen in Euro",
            "2 australische Dollar",
            "50 indische Rupien",
            "3 schwedische Kronen",
        ],
        &[Text("5,33 €"), Text("2 AUD"), Text("50 INR"), Text("3 SEK")],
    ),
    // Functions and formats
    (
        &[
            "Quadratwurzel 16",
            "Wurzel aus 16",
            "Kubikwurzel 27",
            "255 in hexadezimal",
            "5 in binär",
        ],
        &[Text("4"), Text("4"), Text("3"), Text("0xFF"), Text("0b101")],
    ),
    // Prose stays prose
    (
        &["Guten Tag", "Das kostet 5 €", "Wir treffen uns um 14 Uhr"],
        &[Nothing, Text("5 €"), Text("14:00")],
    ),
];

const AUSTRIAN: Table = &[(
    &["12. Jänner 2026", "2 Zentner in kg", "10 Deka in g", "3 Fuß"],
    &[
        Text("Mo., 12. Jän. 2026"),
        Text("200 kg"),
        Text("100 g"),
        Text("3 Fuß"),
    ],
)];

const SWISS: Table = &[(
    &["3 Fuss", "2 Zentner in kg", "5 Fr.", "heute", "20 Stutz in Euro"],
    &[
        Text("3 Fuss"),
        Text("200 kg"),
        Text("5 CHF"),
        Text("Mi., 28. Okt. 2026"),
        Text("€ 17.78"),
    ],
)];

#[test]
fn german() {
    assert_table(InputLanguage::German, NumberStyle::German, GERMAN);
}

#[test]
fn austrian_german() {
    assert_table(InputLanguage::GermanAustria, NumberStyle::German, AUSTRIAN);
}

#[test]
fn swiss_german() {
    assert_table(InputLanguage::GermanSwitzerland, NumberStyle::Swiss, SWISS);
}
