//! Polish (`pl`).

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const POLISH: Table = &[
    // Operators and conversion
    (
        &[
            "5 plus 3",
            "5 i 3",
            "5 dodać 3",
            "10 minus 3",
            "10 bez 4",
            "3 razy 4",
            "8 podzielić przez 2",
            "8 przez 2",
            "10 dzielone przez 4",
            "17 modulo 5",
        ],
        &[
            Text("8"),
            Text("8"),
            Text("8"),
            Text("7"),
            Text("6"),
            Text("12"),
            Text("4"),
            Text("4"),
            Text("2,5"),
            Text("2"),
        ],
    ),
    (
        &[
            "5 cali w cm",
            "5 cali na cm",
            "5 cali do cm",
            "5 cali jako cm",
            "1 m to cm",
            "1 mila w km",
        ],
        &[
            Text("12,7 cm"),
            Text("12,7 cm"),
            Text("12,7 cm"),
            Text("12,7 cm"),
            Text("100 cm"),
            Text("1,61 km"),
        ],
    ),
    (
        &[
            "60 km na godzinę",
            "120 km / 2 godziny",
            "10 zł za kg",
            "100 zł dziennie",
            "100 € miesięcznie * 12 miesięcy",
        ],
        &[
            Text("60 km/h"),
            Text("60 km/h"),
            Text("10 PLN/kg"),
            Text("100 PLN/d"),
            Text("1 200 €"),
        ],
    ),
    (
        &[
            "x równa się 5",
            "x razy 2",
            "czynsz wynosi 800 zł",
            "czynsz przez 2",
        ],
        &[Text("5"), Text("10"), Text("800 PLN"), Text("400 PLN")],
    ),
    // Percentages (`od` reads as `of`: `podatek 19% od dochodu`)
    (
        &[
            "20 procent z 50",
            "20% z 50",
            "20 % od 50",
            "10 % narzut na 50",
            "10 % więcej niż 50",
            "10 % mniej niż 50",
            "6 % rabatu od 40 €",
            "100 zł + 23 %",
        ],
        &[
            Text("10"),
            Text("10"),
            Text("10"),
            Text("55"),
            Text("55"),
            Text("45"),
            Text("37,60 €"),
            Text("123 PLN"),
        ],
    ),
    (
        &[
            "25 jako % z 50",
            "25 jako procent od 50",
            "75 jako % wzrostu od 50",
            "25 jako % zmniejszenia z 50",
        ],
        &[Text("50%"), Text("50%"), Text("50%"), Text("50%")],
    ),
    (
        &[
            "20 % z czego jest 10",
            "20 % z czego to 10",
            "25 % po dodaniu czego jest 125",
            "20 % po odjęciu czego jest 80",
        ],
        &[Text("50"), Text("50"), Text("100"), Text("100")],
    ),
    // Scales (long scale: miliard = 10^9, bilion = 10^12; `m` stays metre)
    (
        &[
            "2 miliony",
            "3 miliardy",
            "2 biliony",
            "1,5 mln zł",
            "4 tysiące",
            "5 tys.",
            "2 m",
        ],
        &[
            Text("2 000 000"),
            Text("3 000 000 000"),
            Text("2 000 000 000 000"),
            Text("1 500 000 PLN"),
            Text("4 000"),
            Text("5 000"),
            Text("2 m"),
        ],
    ),
    // Dates and times
    (
        &[
            "dzisiaj",
            "dziś + 2 tygodnie",
            "jutro",
            "wczoraj",
            "pojutrze",
            "przedwczoraj",
        ],
        &[
            Text("śr., 28 paź 2026"),
            Text("śr., 11 lis 2026"),
            Text("czw., 29 paź 2026"),
            Text("wt., 27 paź 2026"),
            Text("pt., 30 paź 2026"),
            Text("pon., 26 paź 2026"),
        ],
    ),
    (
        &["teraz", "czas", "południe", "północ"],
        &[Text("13:00"), Text("13:00"), Text("12:00"), Text("00:00")],
    ),
    (
        &[
            "czas w Warszawie",
            "czas w Londynie",
            "czas w Moskwie",
            "czas w Pekinie",
            "14:00 w Nowym Jorku",
            "Wiedeń czas",
            "czas w Tokio",
        ],
        &[
            Text("13:00"),
            Text("12:00"),
            Text("15:00"),
            Text("20:00"),
            Text("09:00"),
            Text("13:00"),
            Text("21:00"),
        ],
    ),
    (
        &[
            "czas w Wiedniu",
            "czas w Monachium",
            "czas w Lizbonie",
            "czas w Brukseli",
            "czas w Kopenhadze",
            "czas w Genewie",
            "czas w Paryżu",
            "czas w Rzymie",
        ],
        &[
            Text("13:00"),
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
        &[
            "12 marca 2026",
            "12 marca 2026 r.",
            "3 paź 2026",
            "1 maja",
            "2026-12-24 - dziś",
        ],
        &[
            Text("czw., 12 mar 2026"),
            Text("czw., 12 mar 2026"),
            Text("sob., 3 paź 2026"),
            Text("pt., 1 maj 2026"),
            Text("57 dni"),
        ],
    ),
    // Document references
    (
        &["Czynsz: 850 zł", "Prąd: 64 zł", "suma"],
        &[Text("850 PLN"), Text("64 PLN"), Text("914 PLN")],
    ),
    (
        &["Czynsz: 850 zł", "Prąd: 64 zł", "średnia"],
        &[Text("850 PLN"), Text("64 PLN"), Text("457 PLN")],
    ),
    (
        &["4 €", "6 €", "średnia", "poprzedni * 2", "Razem: łącznie"],
        &[Text("4 €"), Text("6 €"), Text("5 €"), Text("10 €"), Text("25 €")],
    ),
    // Time units and localized output (one | few | many | other)
    (
        &[
            "1 dzień",
            "2 dni",
            "1,5 dnia",
            "2 godziny",
            "5 godzin",
            "1,5 godziny",
            "22 godziny",
            "12 godzin",
            "1 miesiąc w dniach",
        ],
        &[
            Text("1 dzień"),
            Text("2 dni"),
            Text("1,5 dnia"),
            Text("2 godziny"),
            Text("5 godzin"),
            Text("1,5 godziny"),
            Text("22 godziny"),
            Text("12 godzin"),
            Text("30,42 dnia"),
        ],
    ),
    (
        &[
            "90 minut w godzinach",
            "1 godzina 30 minut",
            "1 rok",
            "2 lata",
            "5 lat",
            "3 tygodnie",
            "5 tygodni",
            "2 tyg",
            "10 sek",
            "3 godz",
        ],
        &[
            Text("1,5 godziny"),
            Text("90 minut"),
            Text("1 rok"),
            Text("2 lata"),
            Text("5 lat"),
            Text("3 tygodnie"),
            Text("5 tygodni"),
            Text("2 tygodnie"),
            Text("10 sekund"),
            Text("3 godziny"),
        ],
    ),
    // Length, area, volume
    (
        &[
            "5 cali",
            "1 cal",
            "3 stopy",
            "5 stóp",
            "5 kilometrów w milach",
            "5 mil morskich w km",
        ],
        &[
            Text("5 cali"),
            Text("1 cal"),
            Text("3 stopy"),
            Text("5 stóp"),
            Text("3,11 mili"),
            Text("9,26 km"),
        ],
    ),
    (
        &[
            "20 metrów kwadratowych",
            "20 m kw.",
            "2 stopy kwadratowe",
            "5 ha w m²",
            "5 arów",
            "5 cali sześciennych",
        ],
        &[
            Text("20 m²"),
            Text("20 m²"),
            Text("2 stopy kw."),
            Text("50 000 m²"),
            Text("5 arów"),
            Text("5 cali sześc."),
        ],
    ),
    (
        &[
            "2 litry w galonach",
            "3 szklanki w mililitrach",
            "2 łyżki w ml",
            "3 łyżeczki w ml",
        ],
        &[
            Text("0,53 galona"),
            Text("709,76 ml"),
            Text("29,57 ml"),
            Text("14,79 ml"),
        ],
    ),
    // Weight, temperature, angles, data
    (
        &[
            "250 gramów w uncjach",
            "5 kg w funtach",
            "5 kilo w funtach",
            "5 funtów w kg",
            "2 kwintale w kg",
            "3 tony w kg",
            "10 deko w g",
        ],
        &[
            Text("8,82 uncji"),
            Text("11,02 funta"),
            Text("11,02 funta"),
            Text("2,27 kg"),
            Text("200 kg"),
            Text("3 000 kg"),
            Text("100 g"),
        ],
    ),
    (
        &[
            "100 stopni Celsjusza w Fahrenheitach",
            "90 stopni w radianach",
            "1 GiB w MB",
            "3 kelwiny",
        ],
        &[Text("212 °F"), Text("1,57 rad"), Text("1 073,74 MB"), Text("3 K")],
    ),
    // Currencies (`funt` is the currency on money lines, the weight elsewhere)
    (
        &[
            "5 dolarów w euro",
            "10 euro w dolarach",
            "3 funty szterlingi w euro",
            "5 funtów w euro",
            "£5 + 3 funty",
            "100 jenów w euro",
            "2 franki szwajcarskie w euro",
            "100 złotych",
            "3 SEK",
        ],
        &[
            Text("4 €"),
            Text("12,50 $"),
            Text("3,20 €"),
            Text("5,33 €"),
            Text("8 £"),
            Text("0,53 €"),
            Text("1,78 €"),
            Text("100 PLN"),
            Text("3 SEK"),
        ],
    ),
    // Functions and formats
    (
        &[
            "pierwiastek kwadratowy z 16",
            "pierwiastek z 16",
            "pierwiastek sześcienny z 27",
            "255 w szesnastkowym",
            "5 w systemie dwójkowym",
            "zaokrąglij 2,6",
            "silnia 5",
        ],
        &[
            Text("4"),
            Text("4"),
            Text("3"),
            Text("0xFF"),
            Text("0b101"),
            Text("3"),
            Text("120"),
        ],
    ),
    // Prose stays prose
    (
        &["Dzień dobry", "To kosztuje 5 zł", "Spotykamy się o 14:00"],
        &[Nothing, Text("5 PLN"), Text("14:00")],
    ),
];

#[test]
fn polish() {
    assert_table(InputLanguage::Polish, NumberStyle::French, POLISH);
}
