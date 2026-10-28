//! Romanian: `ro`.

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const ROMANIAN: Table = &[
    // Operators and conversion
    (
        &[
            "3 ori 4",
            "8 împărțit la 2",
            "5 și 3",
            "10 fără 4",
            "5 € cu 2 €",
            "2 + e",
        ],
        &[
            Text("12"),
            Text("4"),
            Text("8"),
            Text("6"),
            Text("€7"),
            Text("4,72"),
        ],
    ),
    (
        &[
            "5 țoli în cm",
            "1 milă la km",
            "60 km pe oră",
            "100 mărit cu 10 %",
        ],
        &[Text("12,7 cm"), Text("1,61 km"), Text("60 km/h"), Text("110")],
    ),
    (
        &["x egal cu 5", "x ori 2", "chiria e 800 €", "chiria împărțit la 2"],
        &[Text("5"), Text("10"), Text("€800"), Text("€400")],
    ),
    // Percentages (`din` is "of")
    (
        &[
            "20 la sută din 50",
            "20 % din 50",
            "5 % pe 30 €",
            "6 % reducere la 40 €",
            "10 % în plus față de 50",
        ],
        &[Text("10"), Text("10"), Text("€31,50"), Text("€37,60"), Text("55")],
    ),
    (
        &[
            "25 ca % din 50",
            "75 ca % creștere din 50",
            "20 % din cât este 10",
            "25 % pe cât este 125",
            "20 % reducere din cât este 80",
        ],
        &[Text("50%"), Text("50%"), Text("50"), Text("100"), Text("100")],
    ),
    // Scales (long scale; `de` after 20; m stays metre)
    (
        &[
            "2 milioane",
            "3 miliarde",
            "2 bilioane",
            "20 de mii de lei",
            "1,5 mil. lei",
            "2 m",
        ],
        &[
            Text("2.000.000"),
            Text("3.000.000.000"),
            Text("2.000.000.000.000"),
            Text("20.000 RON"),
            Text("1.500.000 RON"),
            Text("2 m"),
        ],
    ),
    // Dates and times
    (
        &[
            "azi",
            "mâine",
            "ieri",
            "poimâine",
            "12 martie 2026",
            "2026-12-24 - azi",
        ],
        &[
            Text("mie., 28 oct. 2026"),
            Text("joi, 29 oct. 2026"),
            Text("mar., 27 oct. 2026"),
            Text("vin., 30 oct. 2026"),
            Text("joi, 12 mar. 2026"),
            Text("57 de zile"),
        ],
    ),
    (
        &[
            "acum",
            "ora în Londra",
            "ora la Moscova",
            "acum în Pekin",
            "14:00 în Lisabona",
            "ora în București",
            "ora în Varșovia",
            "ora în Copenhaga",
        ],
        &[
            Text("13:00"),
            Text("12:00"),
            Text("15:00"),
            Text("20:00"),
            Text("13:00"),
            Text("14:00"),
            Text("13:00"),
            Text("13:00"),
        ],
    ),
    // Document references
    (
        &["Chirie: 850 €", "Curent: 64 €", "sumă"],
        &[Text("€850"), Text("€64"), Text("€914")],
    ),
    (
        &["4 €", "6 €", "medie", "anterior * 2"],
        &[Text("€4"), Text("€6"), Text("€5"), Text("€10")],
    ),
    // Units and localized output: one, few (2-19, fractions), other with `de`
    (
        &[
            "1 zi",
            "2 ore",
            "19 ore",
            "20 de ore",
            "1 lună în zile",
            "90 de minute în ore",
        ],
        &[
            Text("1 zi"),
            Text("2 ore"),
            Text("19 ore"),
            Text("20 de ore"),
            Text("30,42 zile"),
            Text("1,5 ore"),
        ],
    ),
    (
        &[
            "25 de picioare",
            "20 de metri pătrați",
            "3 centimetri cubi",
            "5 kilometri în mile",
            "250 de grame în uncii",
            "3 căni în mililitri",
        ],
        &[
            Text("25 de picioare"),
            Text("20 m²"),
            Text("3 cm³"),
            Text("3,11 mile"),
            Text("8,82 uncii"),
            Text("709,76 ml"),
        ],
    ),
    (
        &[
            "100 de grade Celsius în Fahrenheit",
            "1 GiB în MB",
            "5 kile în livre",
            "2 chintale în kg",
        ],
        &[
            Text("212 °F"),
            Text("1.073,74 MB"),
            Text("11,02 livre"),
            Text("200 kg"),
        ],
    ),
    // Currencies
    (
        &["100 de lei", "50 de euro în dolari", "10 lire sterline în euro"],
        &[Text("100 RON"), Text("$62,50"), Text("€10,67")],
    ),
    // Functions, formats, prose
    (
        &[
            "rădăcina pătrată din 16",
            "255 în hexazecimal",
            "Bună ziua",
            "Asta costă 5 €",
        ],
        &[Text("4"), Text("0xFF"), Nothing, Text("€5")],
    ),
];

#[test]
fn romanian() {
    assert_table(InputLanguage::Romanian, NumberStyle::German, ROMANIAN);
}
