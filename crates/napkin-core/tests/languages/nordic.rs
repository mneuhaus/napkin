//! Danish (`da`), Norwegian (`no`), Swedish (`sv`) and Finnish (`fi`).

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::Text;
use crate::{Table, assert_table};

const DANISH: Table = &[
    (
        &[
            "3 gange 4",
            "8 divideret med 2",
            "5 og 3",
            "10 fratrukket 4",
            "5 tommer i cm",
        ],
        &[Text("12"), Text("4"), Text("8"), Text("6"), Text("12,7 cm")],
    ),
    (
        &[
            "20 procent af 50",
            "10 % rabat på 50",
            "10 % oven i 50",
            "25 som % af 50",
        ],
        &[Text("10"), Text("45"), Text("55"), Text("50%")],
    ),
    // Long scale: milliard = 10^9, billion = 10^12
    (
        &["2 millioner", "2 milliarder", "2 billioner"],
        &[
            Text("2.000.000"),
            Text("2.000.000.000"),
            Text("2.000.000.000.000"),
        ],
    ),
    (
        &[
            "i dag",
            "i morgen",
            "i går",
            "12. marts 2026",
            "tid i København",
            "tid i Moskva",
        ],
        &[
            Text("ons. 28. okt. 2026"),
            Text("tors. 29. okt. 2026"),
            Text("tirs. 27. okt. 2026"),
            Text("tors. 12. mar. 2026"),
            Text("13:00"),
            Text("15:00"),
        ],
    ),
    (
        &["1 time", "3 timer", "2 dage", "1 uge", "90 minutter i timer"],
        &[
            Text("1 time"),
            Text("3 timer"),
            Text("2 dage"),
            Text("1 uge"),
            Text("1,5 time"),
        ],
    ),
    (
        &[
            "5 km i engelske mil",
            "20 kvm",
            "2 kvadratfod",
            "5 pund i kg",
            "100 km/t",
        ],
        &[
            Text("3,11 engelske mil"),
            Text("20 m²"),
            Text("2 kvadratfod"),
            Text("2,27 kg"),
            Text("100 km/h"),
        ],
    ),
    (
        &[
            "husleje: 9500 kr",
            "el: 650 kr",
            "total",
            "x er 5",
            "x gange 2",
            "3 SEK",
        ],
        &[
            Text("9.500 DKK"),
            Text("650 DKK"),
            Text("10.150 DKK"),
            Text("5"),
            Text("10"),
            Text("3 SEK"),
        ],
    ),
];

const NORWEGIAN: Table = &[
    (
        &[
            "3 ganger 4",
            "8 delt på 2",
            "5 og 3",
            "10 fratrukket 4",
            "5 tommer i cm",
        ],
        &[Text("12"), Text("4"), Text("8"), Text("6"), Text("12,7 cm")],
    ),
    (
        &[
            "20 prosent av 50",
            "10 % rabatt på 50",
            "10 % på 50",
            "10 % fra 50",
            "25 som % av 50",
        ],
        &[Text("10"), Text("45"), Text("55"), Text("45"), Text("50%")],
    ),
    // Long scale: milliard = 10^9, billion = 10^12
    (
        &["2 millioner", "2 milliarder", "2 billioner"],
        &[
            Text("2 000 000"),
            Text("2 000 000 000"),
            Text("2 000 000 000 000"),
        ],
    ),
    (
        &[
            "i dag",
            "i morgen",
            "i går",
            "12. mars 2026",
            "tid i Bergen",
            "tid i Moskva",
        ],
        &[
            Text("ons. 28. okt. 2026"),
            Text("tor. 29. okt. 2026"),
            Text("tir. 27. okt. 2026"),
            Text("tor. 12. mar. 2026"),
            Text("13:00"),
            Text("15:00"),
        ],
    ),
    // `time` is the hour after a number and stays `now` on its own
    (
        &[
            "1 time",
            "3 timer",
            "2 dager",
            "1 uke",
            "90 minutter i timer",
            "time",
        ],
        &[
            Text("1 time"),
            Text("3 timer"),
            Text("2 dager"),
            Text("1 uke"),
            Text("1,5 timer"),
            Text("13:00"),
        ],
    ),
    (
        &["5 km i engelske mil", "100 kr + 50 kr", "20 $ i euro", "x er 5"],
        &[
            Text("3,11 engelske mil"),
            Text("150 NOK"),
            Text("16 €"),
            Text("5"),
        ],
    ),
];

const SWEDISH: Table = &[
    (
        &[
            "3 gånger 4",
            "8 delat med 2",
            "5 och 3",
            "10 minus 4",
            "5 tum i cm",
        ],
        &[Text("12"), Text("4"), Text("8"), Text("6"), Text("12,7 cm")],
    ),
    // `av` is `of`
    (
        &[
            "20 procent av 50",
            "10 % rabatt på 50",
            "10 % på 50",
            "25 som % av 50",
        ],
        &[Text("10"), Text("45"), Text("55"), Text("50%")],
    ),
    // Long scale: miljard = 10^9, biljon = 10^12
    (
        &["2 miljoner", "2 miljarder", "2 biljoner"],
        &[
            Text("2 000 000"),
            Text("2 000 000 000"),
            Text("2 000 000 000 000"),
        ],
    ),
    (
        &[
            "idag",
            "i morgon",
            "igår",
            "12 mars 2026",
            "tid i Köpenhamn",
            "tid i Helsingfors",
        ],
        &[
            Text("ons 28 okt. 2026"),
            Text("tors 29 okt. 2026"),
            Text("tis 27 okt. 2026"),
            Text("tors 12 mars 2026"),
            Text("13:00"),
            Text("14:00"),
        ],
    ),
    (
        &["1 timme", "3 timmar", "2 dagar", "1 vecka", "90 minuter i timmar"],
        &[
            Text("1 timme"),
            Text("3 timmar"),
            Text("2 dagar"),
            Text("1 vecka"),
            Text("1,5 timmar"),
        ],
    ),
    // `kilo` is the kilogram and `hekto` 100 g in Swedish
    (
        &[
            "2 kilo i gram",
            "3 hekto i gram",
            "75 kvm",
            "3 krm",
            "2 msk i tsk",
        ],
        &[
            Text("2 000 g"),
            Text("300 g"),
            Text("75 m²"),
            Text("3 ml"),
            Text("6 teskedar"),
        ],
    ),
    (
        &["hyra: 8500 kr", "mat: 3200 kr", "summa", "x är 5"],
        &[
            Text("8 500 SEK"),
            Text("3 200 SEK"),
            Text("11 700 SEK"),
            Text("5"),
        ],
    ),
];

const FINNISH: Table = &[
    (
        &["3 kertaa 4", "8 jaettuna 2", "5 ja 3", "10 miinus 4"],
        &[Text("12"), Text("4"), Text("8"), Text("6")],
    ),
    (
        &[
            "5 tuumaa muunnettuna senttimetreiksi",
            "90 minuuttia muunnettuna tunneiksi",
            "5 km muunnettuna maileiksi",
        ],
        &[Text("12,7 cm"), Text("1,5 tuntia"), Text("3,11 mailia")],
    ),
    (
        &[
            "20 % summasta 50",
            "10 % alennus 50",
            "10 % päälle 50",
            "25 prosentteina luvusta 50",
        ],
        &[Text("10"), Text("45"), Text("55"), Text("50%")],
    ),
    // Long scale: miljardi = 10^9, biljoona = 10^12
    (
        &["2 miljoonaa", "2 miljardia", "2 biljoonaa"],
        &[
            Text("2 000 000"),
            Text("2 000 000 000"),
            Text("2 000 000 000 000"),
        ],
    ),
    (
        &[
            "tänään",
            "huomenna",
            "eilen",
            "12. maaliskuuta 2026",
            "Lontoon aika",
            "aika Tukholma",
        ],
        &[
            Text("ke 28.10.2026"),
            Text("to 29.10.2026"),
            Text("ti 27.10.2026"),
            Text("to 12.3.2026"),
            Text("12:00"),
            Text("13:00"),
        ],
    ),
    // One takes the nominative, every other count the partitive
    (
        &["1 tunti", "3 tuntia", "2 päivää", "1 viikko", "1,5 tuntia"],
        &[
            Text("1 tunti"),
            Text("3 tuntia"),
            Text("2 päivää"),
            Text("1 viikko"),
            Text("1,5 tuntia"),
        ],
    ),
    (
        &[
            "20 neliömetriä",
            "2 neliöjalkaa",
            "100 € + 50 euroa",
            "neliöjuuri 16",
        ],
        &[Text("20 m²"), Text("2 neliöjalkaa"), Text("150 €"), Text("4")],
    ),
    (
        &["vuokra: 850 €", "ruoka: 320 €", "yhteensä", "x yhtä suuri kuin 5"],
        &[Text("850 €"), Text("320 €"), Text("1 170 €"), Text("5")],
    ),
];

#[test]
fn danish() {
    assert_table(InputLanguage::Danish, NumberStyle::German, DANISH);
}

#[test]
fn norwegian() {
    assert_table(InputLanguage::Norwegian, NumberStyle::French, NORWEGIAN);
}

#[test]
fn swedish() {
    assert_table(InputLanguage::Swedish, NumberStyle::French, SWEDISH);
}

#[test]
fn finnish() {
    assert_table(InputLanguage::Finnish, NumberStyle::French, FINNISH);
}
