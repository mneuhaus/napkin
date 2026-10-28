//! French: `fr`.

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const FRENCH: Table = &[
    // Operators and conversion
    (
        &["3 fois 4", "8 divisé par 2", "15 sur 20", "7 multiplié par 3"],
        &[Text("12"), Text("4"), Text("0,75"), Text("21")],
    ),
    (
        &["5 et 3", "10 moins 3", "10 sans 4", "5 € avec 2 €"],
        &[Text("8"), Text("7"), Text("6"), Text("7 €")],
    ),
    (
        &[
            "5 pouces en cm",
            "5 pouces convertis en cm",
            "1 mile exprimé en km",
            "1 mile vers km",
        ],
        &[Text("12,7 cm"), Text("12,7 cm"), Text("1,61 km"), Text("1,61 km")],
    ),
    (
        &["60 km par heure", "120 km / 2 heures"],
        &[Text("60 km/h"), Text("60 km/h")],
    ),
    (
        &[
            "x vaut 5",
            "x fois 2",
            "loyer est 800 €",
            "loyer divisé par 2",
            "y est égal à 3",
        ],
        &[Text("5"), Text("10"), Text("800 €"), Text("400 €"), Text("3")],
    ),
    // Percentages
    (
        &[
            "20 pour cent de 50",
            "20 % de 50",
            "5 % d'augmentation de 30 €",
            "6 % de réduction sur 40 €",
        ],
        &[Text("10"), Text("10"), Text("31,50 €"), Text("37,60 €")],
    ),
    (
        &[
            "10 % de remise sur 50",
            "10 % en plus de 50",
            "5 % sur 30 €",
            "100 € plus 19 %",
        ],
        &[Text("45"), Text("55"), Text("31,50 €"), Text("119 €")],
    ),
    (
        &[
            "25 en % de 50",
            "75 en % d'augmentation de 50",
            "25 en pourcentage de réduction de 50",
            "0,25 en %",
        ],
        &[Text("50%"), Text("50%"), Text("50%"), Text("25%")],
    ),
    (
        &[
            "20 % de quoi font 10",
            "25 % de plus sur quoi fait 125",
            "20 % de réduction sur quoi est 80",
        ],
        &[Text("50"), Text("100"), Text("100")],
    ),
    // Scales (long scale; m stays metre)
    (
        &[
            "2 millions",
            "3 milliards",
            "1,5 million €",
            "2 billions",
            "4 mille",
            "2 milliers",
            "2 m",
        ],
        &[
            Text("2 000 000"),
            Text("3 000 000 000"),
            Text("1 500 000 €"),
            Text("2 000 000 000 000"),
            Text("4 000"),
            Text("2 000"),
            Text("2 m"),
        ],
    ),
    // Dates and times
    (
        &[
            "aujourd'hui",
            "aujourd'hui + 2 semaines",
            "demain",
            "hier",
            "après-demain",
            "avant-hier",
        ],
        &[
            Text("mer. 28 oct. 2026"),
            Text("mer. 11 nov. 2026"),
            Text("jeu. 29 oct. 2026"),
            Text("mar. 27 oct. 2026"),
            Text("ven. 30 oct. 2026"),
            Text("lun. 26 oct. 2026"),
        ],
    ),
    (
        &[
            "maintenant",
            "heure",
            "heure actuelle",
            "midi + 2 heures",
            "minuit",
        ],
        &[
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("14:00"),
            Text("00:00"),
        ],
    ),
    (
        &[
            "heure à Tokyo",
            "l'heure à Londres",
            "heure à Moscou",
            "heure à Pékin",
            "14:00 à New York",
            "maintenant à Vienne",
            "heure au Caire",
        ],
        &[
            Text("21:00"),
            Text("12:00"),
            Text("15:00"),
            Text("20:00"),
            Text("09:00"),
            Text("13:00"),
            Text("15:00"),
        ],
    ),
    (
        &[
            "heure à Lisbonne",
            "heure à Varsovie",
            "heure à Bruxelles",
            "heure à Copenhague",
            "heure à Genève",
            "heure au Japon",
        ],
        &[
            Text("12:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("21:00"),
        ],
    ),
    (
        &["12 mars 2026", "3 oct. 2026", "1 mai", "2026-12-24 - aujourd'hui"],
        &[
            Text("jeu. 12 mars 2026"),
            Text("sam. 3 oct. 2026"),
            Text("ven. 1 mai 2026"),
            Text("57 jours"),
        ],
    ),
    // `h` between hours and minutes is a time of day; `14h` alone stays hours
    (
        &[
            "14h30",
            "14 h 30",
            "9h05",
            "14h30 à New York",
            "14h30 + 2 heures",
            "Rendez-vous à 14h30",
        ],
        &[
            Text("14:30"),
            Text("14:30"),
            Text("09:05"),
            Text("09:30"),
            Text("16:30"),
            Text("14:30"),
        ],
    ),
    (
        &["14h", "2 h", "2h", "2 h 30 min", "14h75"],
        &[
            Text("14 h"),
            Text("2 h"),
            Text("2 h"),
            Text("150 min"),
            Text("14"),
        ],
    ),
    // The first of a month takes an ordinal; dates with dashes are day first
    (
        &[
            "1er mai",
            "1er mai 2026",
            "1re mai 2026",
            "1ère mai",
            "24-12-2026",
            "1-5-2026",
            "10-5-3",
        ],
        &[
            Text("ven. 1 mai 2026"),
            Text("ven. 1 mai 2026"),
            Text("ven. 1 mai 2026"),
            Text("ven. 1 mai 2026"),
            Text("jeu. 24 déc. 2026"),
            Text("ven. 1 mai 2026"),
            Text("2"),
        ],
    ),
    // A hyphen between letters joins one word
    (
        &[
            "Rendez-vous à 14:00",
            "e-mail : 5 + 3",
            "Coca-Cola 2 €",
            "après-demain",
            "pi-e",
            "10-5",
        ],
        &[
            Text("14:00"),
            Text("8"),
            Text("2 €"),
            Text("ven. 30 oct. 2026"),
            Text("0,42"),
            Text("5"),
        ],
    ),
    // Document references
    (
        &["loyer : 850 €", "électricité : 64 €", "somme"],
        &[Text("850 €"), Text("64 €"), Text("914 €")],
    ),
    (
        &["loyer : 850 €", "électricité : 64 €", "moyenne"],
        &[Text("850 €"), Text("64 €"), Text("457 €")],
    ),
    (
        &["4 €", "6 €", "moy", "précédent * 2", "Total : somme"],
        &[Text("4 €"), Text("6 €"), Text("5 €"), Text("10 €"), Text("25 €")],
    ),
    // Units and localized output
    (
        &[
            "1 mois en jours",
            "1 jour",
            "2 heures",
            "1,5 heure",
            "5 pouces",
            "3 pieds",
            "90 minutes en heures",
        ],
        &[
            Text("30,42 jours"),
            Text("1 jour"),
            Text("2 heures"),
            Text("1,5 heure"),
            Text("5 pouces"),
            Text("3 pieds"),
            Text("1,5 heure"),
        ],
    ),
    (
        &[
            "1 heure 30 minutes",
            "3 j",
            "2 sem",
            "1 an",
            "3 ans",
            "10 sec",
            "5 mn",
        ],
        &[
            Text("90 minutes"),
            Text("3 jours"),
            Text("2 semaines"),
            Text("1 an"),
            Text("3 ans"),
            Text("10 secondes"),
            Text("5 minutes"),
        ],
    ),
    (
        &[
            "20 mètres carrés",
            "3 centimètres cubes",
            "2 pieds carrés",
            "1 pied carré",
            "5 pouces cubes",
            "2 hectares en ares",
        ],
        &[
            Text("20 m²"),
            Text("3 cm³"),
            Text("2 pieds carrés"),
            Text("1 pied carré"),
            Text("5 pouces cubes"),
            Text("200 ares"),
        ],
    ),
    (
        &[
            "15 € de l'heure",
            "90 km à l'heure",
            "3 € le kilo",
            "3 millions de dollars",
            "3 millions d'euros",
            "2 kilos de pommes",
        ],
        &[
            Text("15 €/h"),
            Text("90 km/h"),
            Text("3 €/kg"),
            Text("3 000 000 $"),
            Text("3 000 000 €"),
            Text("2 kg"),
        ],
    ),
    (
        &[
            "5 kilomètres en miles",
            "250 grammes en onces",
            "2 litres en gallons",
            "3 tasses en millilitres",
            "2 cuillères à soupe en ml",
        ],
        &[
            Text("3,11 miles"),
            Text("8,82 onces"),
            Text("0,53 gallon"),
            Text("709,76 ml"),
            Text("29,57 ml"),
        ],
    ),
    (
        &[
            "100 degrés Celsius en Fahrenheit",
            "90 degrés en radians",
            "1 Gio en Mo",
            "3 Go en Mo",
            "5 kilos en livres",
        ],
        &[
            Text("212 °F"),
            Text("1,57 rad"),
            Text("1 073,74 MB"),
            Text("3 000 MB"),
            Text("11,02 livres"),
        ],
    ),
    (
        &[
            "5 livres en kg",
            "5 livres en euros",
            "£5 + 3 livres",
            "2 quintaux en kg",
            "3 nœuds",
        ],
        &[
            Text("2,27 kg"),
            Text("5,33 €"),
            Text("8 £"),
            Text("200 kg"),
            Text("3 nœuds"),
        ],
    ),
    // Currencies
    (
        &[
            "3 dollars canadiens en euros",
            "5 francs suisses en euros",
            "100 yens en euros",
            "10 dollars en euros",
            "20 € en livres sterling",
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
            "racine carrée 16",
            "racine cubique 27",
            "255 en hexadécimal",
            "5 en binaire",
        ],
        &[Text("4"), Text("3"), Text("0xFF"), Text("0b101")],
    ),
    // Prose stays prose
    (
        &["Bonjour tout le monde", "Ça coûte 5 €", "Il a 3 ans"],
        &[Nothing, Text("5 €"), Text("3 ans")],
    ),
];

#[test]
fn french() {
    assert_table(InputLanguage::French, NumberStyle::French, FRENCH);
}
