//! Spanish: `es`.

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const SPANISH: Table = &[
    // Operators and conversion (`por` multiplies between numbers, divides before a unit)
    (
        &[
            "3 por 4",
            "8 dividido por 2",
            "6 entre 3",
            "7 multiplicado por 3",
            "3 veces 4",
        ],
        &[Text("12"), Text("4"), Text("2"), Text("21"), Text("12")],
    ),
    (
        &[
            "5 más 3",
            "5 y 3",
            "10 menos 3",
            "10 sin 4",
            "sin 30",
            "5 € con 2 €",
        ],
        &[
            Text("8"),
            Text("8"),
            Text("7"),
            Text("6"),
            Text("0,5"),
            Text("7 €"),
        ],
    ),
    (
        &[
            "5 pulgadas en cm",
            "5 pulgadas a cm",
            "1 milla como km",
            "1 milla para km",
        ],
        &[Text("12,7 cm"), Text("12,7 cm"), Text("1,61 km"), Text("1,61 km")],
    ),
    (
        &["60 km por hora", "120 km / 2 horas"],
        &[Text("60 km/h"), Text("60 km/h")],
    ),
    (
        &[
            "x es 5",
            "x por 2",
            "alquiler es igual a 800 €",
            "alquiler entre 2",
            "z vale 3",
        ],
        &[Text("5"), Text("10"), Text("800 €"), Text("400 €"), Text("3")],
    ),
    // `y` is plus, but a variable named `y` shadows it
    (&["y = 5", "y por 2"], &[Text("5"), Text("10")]),
    // Percentages
    (
        &[
            "20 por ciento de 50",
            "20 % de 50",
            "5 % sobre 30 €",
            "6 % de descuento sobre 40 €",
        ],
        &[Text("10"), Text("10"), Text("31,50 €"), Text("37,60 €")],
    ),
    (
        &["10 % de aumento sobre 50", "100 € más 21 %"],
        &[Text("55"), Text("121 €")],
    ),
    (
        &[
            "25 como % de 50",
            "75 como % incremento de 50",
            "25 como un % de descuento de 50",
            "0,25 como %",
            "0,25 en %",
        ],
        &[Text("50%"), Text("50%"), Text("50%"), Text("25%"), Text("25%")],
    ),
    (
        &[
            "20 % de qué es 10",
            "25 % sobre lo que es 125",
            "20 % de descuento sobre qué es 80",
        ],
        &[Text("50"), Text("100"), Text("100")],
    ),
    // Scales (long scale; m stays metre, miles stays the mile)
    (
        &[
            "2 millones",
            "3 mil millones",
            "3 millardos",
            "1,5 millones €",
            "2 billones",
            "4 mil",
            "2 m",
            "5 miles en km",
        ],
        &[
            Text("2.000.000"),
            Text("3.000.000.000"),
            Text("3.000.000.000"),
            Text("1.500.000 €"),
            Text("2.000.000.000.000"),
            Text("4.000"),
            Text("2 m"),
            Text("8,05 km"),
        ],
    ),
    // Dates and times
    (
        &[
            "hoy",
            "hoy + 2 semanas",
            "mañana",
            "ayer",
            "pasado mañana",
            "anteayer",
        ],
        &[
            Text("mié, 28 oct 2026"),
            Text("mié, 11 nov 2026"),
            Text("jue, 29 oct 2026"),
            Text("mar, 27 oct 2026"),
            Text("vie, 30 oct 2026"),
            Text("lun, 26 oct 2026"),
        ],
    ),
    (
        &["ahora", "la hora", "mediodía + 2 horas", "medianoche"],
        &[Text("13:00"), Text("13:00"), Text("14:00"), Text("00:00")],
    ),
    (
        &[
            "qué hora es en Tokio",
            "la hora en Londres",
            "ahora en Moscú",
            "14:00 en Nueva York",
            "la hora en Pekín",
            "Viena ahora",
            "la hora en El Cairo",
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
            "la hora en Lisboa",
            "la hora en Varsovia",
            "la hora en Bruselas",
            "la hora en Copenhague",
            "la hora en Ginebra",
            "la hora en Múnich",
            "la hora en Ciudad de México",
            "la hora en Bogotá",
        ],
        &[
            Text("12:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("06:00"),
            Text("07:00"),
        ],
    ),
    (
        &[
            "12 de marzo de 2026",
            "12 marzo 2026",
            "3 oct 2026",
            "1 de mayo",
            "3 de octubre del 2026",
            "2026-12-24 - hoy",
        ],
        &[
            Text("jue, 12 mar 2026"),
            Text("jue, 12 mar 2026"),
            Text("sáb, 3 oct 2026"),
            Text("vie, 1 may 2026"),
            Text("sáb, 3 oct 2026"),
            Text("57 días"),
        ],
    ),
    // Document references
    (
        &["alquiler: 850 €", "luz: 64 €", "suma"],
        &[Text("850 €"), Text("64 €"), Text("914 €")],
    ),
    (
        &["alquiler: 850 €", "luz: 64 €", "promedio"],
        &[Text("850 €"), Text("64 €"), Text("457 €")],
    ),
    (
        &["4 €", "6 €", "media", "anterior * 2", "Total: total"],
        &[Text("4 €"), Text("6 €"), Text("5 €"), Text("10 €"), Text("25 €")],
    ),
    // Units and localized output
    (
        &[
            "1 mes en días",
            "1 día",
            "2 horas",
            "5 pulgadas",
            "3 pies",
            "90 minutos en horas",
        ],
        &[
            Text("30,42 días"),
            Text("1 día"),
            Text("2 horas"),
            Text("5 pulgadas"),
            Text("3 pies"),
            Text("1,5 horas"),
        ],
    ),
    (
        &[
            "1 hora 30 minutos",
            "3 días",
            "2 sem",
            "1 año",
            "3 años",
            "10 seg",
        ],
        &[
            Text("90 minutos"),
            Text("3 días"),
            Text("2 semanas"),
            Text("1 año"),
            Text("3 años"),
            Text("10 segundos"),
        ],
    ),
    (
        &[
            "20 metros cuadrados",
            "3 centímetros cúbicos",
            "2 pies cuadrados",
            "2 hectáreas en áreas",
            "10 milímetros en cm",
        ],
        &[
            Text("20 m²"),
            Text("3 cm³"),
            Text("2 ft²"),
            Text("200 áreas"),
            Text("1 cm"),
        ],
    ),
    (
        &[
            "5 kilómetros en millas",
            "250 gramos en onzas",
            "2 litros en galones",
            "3 tazas en mililitros",
            "2 cucharadas en ml",
        ],
        &[
            Text("3,11 millas"),
            Text("8,82 onzas"),
            Text("0,53 galones"),
            Text("709,76 ml"),
            Text("29,57 ml"),
        ],
    ),
    (
        &[
            "100 grados Celsius en Fahrenheit",
            "90 grados en radianes",
            "1 GiB en MB",
            "5 kilos en libras",
        ],
        &[
            Text("212 °F"),
            Text("1,57 rad"),
            Text("1.073,74 MB"),
            Text("11,02 libras"),
        ],
    ),
    (
        &[
            "5 libras en kg",
            "5 libras en euros",
            "£5 + 3 libras",
            "2 quintales en kg",
            "3 nudos",
        ],
        &[
            Text("2,27 kg"),
            Text("5,33 €"),
            Text("8 £"),
            Text("200 kg"),
            Text("3 nudos"),
        ],
    ),
    (
        &[
            "100 km a la hora",
            "3 € el kilo",
            "10 € al día",
            "3 millones de dólares",
            "2 kilos de tomates",
            "5 pulgadas cúbicas",
        ],
        &[
            Text("100 km/h"),
            Text("3 €/kg"),
            Text("10 €/d"),
            Text("3.000.000 $"),
            Text("2 kg"),
            Text("5 in³"),
        ],
    ),
    // Currencies
    (
        &[
            "3 dólares canadienses en euros",
            "5 francos suizos en euros",
            "100 yenes en euros",
            "10 dólares en euros",
            "20 € en libras esterlinas",
            "3 pesos mexicanos",
        ],
        &[
            Text("1,92 €"),
            Text("4,44 €"),
            Text("0,53 €"),
            Text("8 €"),
            Text("18,75 £"),
            Text("3 MXN"),
        ],
    ),
    // Functions and formats
    (
        &[
            "raíz cuadrada 16",
            "raíz cúbica 27",
            "255 en hexadecimal",
            "5 en binario",
        ],
        &[Text("4"), Text("3"), Text("0xFF"), Text("0b101")],
    ),
    // Prose stays prose
    (&["Buenos días", "Cuesta 5 €"], &[Nothing, Text("5 €")]),
];

#[test]
fn spanish() {
    assert_table(InputLanguage::Spanish, NumberStyle::German, SPANISH);
}
