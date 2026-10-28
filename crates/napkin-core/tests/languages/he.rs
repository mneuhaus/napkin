//! Hebrew (`he`).

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const HEBREW: Table = &[
    // Operators and conversion (a separate ב or ל converts; abbreviations use gershayim ״)
    (
        &[
            "5 ועוד 3",
            "10 פחות 3",
            "3 כפול 4",
            "8 חלקי 2",
            "5 אינץ׳ ב ס״מ",
            "5 אינץ' ל ס״מ",
            "5 ק״מ בתור מייל",
            "60 ק״מ לשעה",
            "100 ש״ח לשעה",
        ],
        &[
            Text("8"),
            Text("7"),
            Text("12"),
            Text("4"),
            Text("12.7 cm"),
            Text("12.7 cm"),
            Text("3.11 מייל"),
            Text("60 km/h"),
            Text("100 ILS/h"),
        ],
    ),
    (
        &["שכירות שווה 4500 ש״ח", "שכירות חלקי 2"],
        &[Text("4,500 ILS"), Text("2,250 ILS")],
    ),
    // Percentages
    (
        &[
            "20 אחוז מתוך 50",
            "20% של 50",
            "20% הנחה על 80",
            "10% תוספת על 50",
            "25 כאחוז מ 50",
            "20% מתוך מה זה 10",
        ],
        &[
            Text("10"),
            Text("10"),
            Text("64"),
            Text("55"),
            Text("50%"),
            Text("50"),
        ],
    ),
    (
        &["2 מיליון", "3 מיליארד", "5 אלף ש״ח"],
        &[Text("2,000,000"), Text("3,000,000,000"), Text("5,000 ILS")],
    ),
    // Dates and times (`במרץ` with the joined ב)
    (
        &[
            "היום",
            "מחר",
            "מחרתיים",
            "שלשום",
            "עכשיו",
            "השעה ב לונדון",
            "בלונדון השעה",
            "14:00 ב ניו יורק",
            "12 במרץ 2026",
        ],
        &[
            Text("יום ד׳, 28 באוק׳ 2026"),
            Text("יום ה׳, 29 באוק׳ 2026"),
            Text("יום ו׳, 30 באוק׳ 2026"),
            Text("יום ב׳, 26 באוק׳ 2026"),
            Text("13:00"),
            Text("12:00"),
            Text("12:00"),
            Text("09:00"),
            Text("יום ה׳, 12 במרץ 2026"),
        ],
    ),
    (
        &["שכירות: 4500 ש״ח", "חשמל: 300 ש״ח", "סה״כ", "ממוצע"],
        &[
            Text("4,500 ILS"),
            Text("300 ILS"),
            Text("4,800 ILS"),
            Text("3,200 ILS"),
        ],
    ),
    // Units: one | two | other (two takes the plural with a numeral)
    (
        &[
            "1 יום",
            "2 ימים",
            "1 שעה",
            "2 שעות",
            "1.5 שעות",
            "90 דקות ב שעות",
            "1 חודש ב ימים",
            "3 דק׳",
        ],
        &[
            Text("1 יום"),
            Text("2 ימים"),
            Text("1 שעה"),
            Text("2 שעות"),
            Text("1.5 שעות"),
            Text("1.5 שעות"),
            Text("30.42 ימים"),
            Text("3 דקות"),
        ],
    ),
    (
        &[
            "5 ק״מ ב מיילים",
            "20 מ״ר",
            "20 מטר רבוע",
            "2 קוב ב ליטר",
            "3 כוסות ב מ״ל",
            "5 ק״ג ב ליברות",
            "100 מעלות צלזיוס ב פרנהייט",
        ],
        &[
            Text("3.11 מייל"),
            Text("20 m²"),
            Text("20 m²"),
            Text("2,000 l"),
            Text("709.76 ml"),
            Text("11.02 ליברות"),
            Text("212 °F"),
        ],
    ),
    (
        &["5 דולר ב אירו", "10 יורו ב דולרים", "3 ש״ח", "שורש 16", "שלום"],
        &[Text("4 €"), Text("12.50 $"), Text("3 ILS"), Text("4"), Nothing],
    ),
];

#[test]
fn hebrew() {
    assert_table(InputLanguage::Hebrew, NumberStyle::English, HEBREW);
}
