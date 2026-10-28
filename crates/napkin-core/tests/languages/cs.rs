//! Czech (`cs`) and Bosnian (`bs-BA`).

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const CZECH: Table = &[
    (
        &[
            "5 a 3",
            "10 minus 3",
            "3 krát 4",
            "8 děleno 2",
            "5 palců na cm",
            "5 palců v cm",
            "60 km za hodinu",
        ],
        &[
            Text("8"),
            Text("7"),
            Text("12"),
            Text("4"),
            Text("12,7 cm"),
            Text("12,7 cm"),
            Text("60 km/h"),
        ],
    ),
    (
        &["nájem je 800 Kč", "nájem děleno 2"],
        &[Text("800 CZK"), Text("400 CZK")],
    ),
    (
        &[
            "20 procent z 50",
            "10 % navíc k 50",
            "20 % sleva z 80",
            "100 Kč + 21 %",
            "25 jako % z 50",
            "20 % z čeho je 10",
        ],
        &[
            Text("10"),
            Text("55"),
            Text("64"),
            Text("121 CZK"),
            Text("50%"),
            Text("50"),
        ],
    ),
    (
        &["2 miliony", "3 miliardy", "2 biliony", "5 mil. Kč", "5 mil na km"],
        &[
            Text("2 000 000"),
            Text("3 000 000 000"),
            Text("2 000 000 000 000"),
            Text("5 000 000 CZK"),
            Text("8,05 km"),
        ],
    ),
    (
        &[
            "dnes",
            "zítra",
            "pozítří",
            "čas v Praze",
            "čas v Londýně",
            "čas ve Vídni",
            "14:00 v New Yorku",
            "12. března 2026",
        ],
        &[
            Text("st 28. 10. 2026"),
            Text("čt 29. 10. 2026"),
            Text("pá 30. 10. 2026"),
            Text("13:00"),
            Text("12:00"),
            Text("13:00"),
            Text("09:00"),
            Text("čt 12. 3. 2026"),
        ],
    ),
    (
        &["nájem: 850 Kč", "elektřina: 64 Kč", "součet", "průměr"],
        &[
            Text("850 CZK"),
            Text("64 CZK"),
            Text("914 CZK"),
            Text("609,33 CZK"),
        ],
    ),
    // Plural forms one | few | other | many (many is the fractions)
    (
        &[
            "1 den",
            "2 dny",
            "5 dní",
            "1,5 dne",
            "5 hodin",
            "1,5 hodiny",
            "3 roky",
            "5 let",
            "1 měsíc ve dnech",
        ],
        &[
            Text("1 den"),
            Text("2 dny"),
            Text("5 dní"),
            Text("1,5 dne"),
            Text("5 hodin"),
            Text("1,5 hodiny"),
            Text("3 roky"),
            Text("5 let"),
            Text("30,42 dne"),
        ],
    ),
    (
        &[
            "5 km na míle",
            "20 čtverečních metrů",
            "3 hrnky v ml",
            "5 kg na libry",
            "5 liber v eurech",
            "10 deka v g",
            "2 metráky v kg",
            "100 stupňů Celsia na Fahrenheita",
            "5 dolarů v eurech",
        ],
        &[
            Text("3,11 míle"),
            Text("20 m²"),
            Text("709,76 ml"),
            Text("11,02 libry"),
            Text("€5,33"),
            Text("100 g"),
            Text("200 kg"),
            Text("212 °F"),
            Text("€4"),
        ],
    ),
    (
        &["odmocnina z 16", "255 na šestnáctkovou soustavu", "Dobrý den"],
        &[Text("4"), Text("0xFF"), Nothing],
    ),
];

const BOSNIAN: Table = &[
    (
        &[
            "5 i 3",
            "10 manje 3",
            "3 puta 4",
            "8 kroz 2",
            "5 inča u cm",
            "60 km na sat",
        ],
        &[
            Text("8"),
            Text("7"),
            Text("12"),
            Text("4"),
            Text("12,7 cm"),
            Text("60 km/h"),
        ],
    ),
    (
        &["kirija je 800 KM", "kirija kroz 2"],
        &[Text("800 BAM"), Text("400 BAM")],
    ),
    (
        &[
            "20 posto od 50",
            "17 % na 100 KM",
            "20 % popusta na 80",
            "25 kao % od 50",
            "20 % od čega je 10",
        ],
        &[Text("10"), Text("117 BAM"), Text("64"), Text("50%"), Text("50")],
    ),
    (
        &["2 miliona", "3 milijarde", "4 hiljade", "1,5 mil. KM"],
        &[
            Text("2.000.000"),
            Text("3.000.000.000"),
            Text("4.000"),
            Text("1.500.000 BAM"),
        ],
    ),
    (
        &[
            "danas",
            "juče",
            "prekosutra",
            "vrijeme u Sarajevu",
            "vrijeme u Moskvi",
            "14:00 u Njujorku",
            "12. marta 2026.",
        ],
        &[
            Text("sri, 28. okt 2026."),
            Text("uto, 27. okt 2026."),
            Text("pet, 30. okt 2026."),
            Text("13:00"),
            Text("15:00"),
            Text("09:00"),
            Text("čet, 12. mar 2026."),
        ],
    ),
    (
        &["kirija: 850 KM", "struja: 64 KM", "zbir"],
        &[Text("850 BAM"), Text("64 BAM"), Text("914 BAM")],
    ),
    // Plural forms one | few | other
    (
        &[
            "1 sat",
            "2 sata",
            "5 sati",
            "21 sat",
            "2 dana",
            "3 godine",
            "5 godina",
            "1 mjesec u danima",
        ],
        &[
            Text("1 sat"),
            Text("2 sata"),
            Text("5 sati"),
            Text("21 sat"),
            Text("2 dana"),
            Text("3 godine"),
            Text("5 godina"),
            Text("30,42 dana"),
        ],
    ),
    (
        &[
            "5 km u milje",
            "20 kvadratnih metara",
            "2 litre u galone",
            "5 kg u funte",
            "5 funti u eurima",
            "10 deka u g",
            "9 $ u eure",
        ],
        &[
            Text("3,11 milja"),
            Text("20 m²"),
            Text("0,53 galona"),
            Text("11,02 funti"),
            Text("€5,33"),
            Text("100 g"),
            Text("€7,20"),
        ],
    ),
    (
        &["korijen iz 16", "255 u heksadecimalno", "Dobar dan"],
        &[Text("4"), Text("0xFF"), Nothing],
    ),
];

#[test]
fn czech() {
    assert_table(InputLanguage::Czech, NumberStyle::French, CZECH);
}

#[test]
fn bosnian() {
    assert_table(InputLanguage::Bosnian, NumberStyle::German, BOSNIAN);
}
