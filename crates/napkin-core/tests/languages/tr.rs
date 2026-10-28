//! Turkish (`tr`).

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const TURKISH: Table = &[
    // Operators and conversion (`kaç` before the target, `olarak`/`cinsinden` after it)
    (
        &[
            "5 artı 3",
            "10 eksi 3",
            "3 çarpı 4",
            "3 kere 4",
            "8 bölü 2",
            "5 inç kaç cm",
            "5 inç cm olarak",
            "5 inç cm cinsinden",
            "100 dolar kaç euro",
            "60 km saatte",
        ],
        &[
            Text("8"),
            Text("7"),
            Text("12"),
            Text("12"),
            Text("4"),
            Text("12,7 cm"),
            Text("12,7 cm"),
            Text("12,7 cm"),
            Text("€80"),
            Text("60 km/h"),
        ],
    ),
    (
        &["kira eşittir 15000 TL", "kira bölü 2"],
        &[Text("15.000 TRY"), Text("7.500 TRY")],
    ),
    // Percentages: the percent word comes before the number
    (
        &["yüzde 20", "50 + yüzde 20", "yüzde 20 of 50"],
        &[Text("20%"), Text("60"), Text("10")],
    ),
    // Scales (`bin` is a thousand after a number, binary after `in`)
    (
        &["2 milyon", "3 milyar", "5 bin TL", "1,5 milyon TL", "255 in bin"],
        &[
            Text("2.000.000"),
            Text("3.000.000.000"),
            Text("5.000 TRY"),
            Text("1.500.000 TRY"),
            Text("0b11111111"),
        ],
    ),
    // Dates and times
    (
        &[
            "bugün",
            "yarın",
            "dün",
            "öbür gün",
            "şimdi",
            "Londra saati",
            "Moskova saati",
            "12 Mart 2026",
            "3 Eki 2026",
        ],
        &[
            Text("28 Eki 2026 Çar"),
            Text("29 Eki 2026 Per"),
            Text("27 Eki 2026 Sal"),
            Text("30 Eki 2026 Cum"),
            Text("13:00"),
            Text("12:00"),
            Text("15:00"),
            Text("12 Mar 2026 Per"),
            Text("3 Eki 2026 Cmt"),
        ],
    ),
    (
        &["kira: 15000 TL", "elektrik: 900 TL", "toplam"],
        &[Text("15.000 TRY"), Text("900 TRY"), Text("15.900 TRY")],
    ),
    // Units: one form after every number
    (
        &[
            "1 gün",
            "3 gün",
            "1,5 saat",
            "90 dakika kaç saat",
            "1 ay kaç gün",
            "3 yil",
            "5 km kaç mil",
            "20 metrekare",
            "2 fit kare",
        ],
        &[
            Text("1 gün"),
            Text("3 gün"),
            Text("1,5 saat"),
            Text("1,5 saat"),
            Text("30,42 gün"),
            Text("3 yıl"),
            Text("3,11 mil"),
            Text("20 m²"),
            Text("2 fit kare"),
        ],
    ),
    (
        &[
            "3 su bardağı kaç ml",
            "5 kg kaç libre",
            "100 derece santigrat kaç fahrenhayt",
            "10 euro kaç dolar",
            "karekök 16",
            "İyi günler",
        ],
        &[
            Text("709,76 ml"),
            Text("11,02 libre"),
            Text("212 °F"),
            Text("$12,50"),
            Text("4"),
            Nothing,
        ],
    ),
];

#[test]
fn turkish() {
    assert_table(InputLanguage::Turkish, NumberStyle::German, TURKISH);
}
