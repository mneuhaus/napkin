//! Portuguese: `pt-BR`, `pt-PT`.

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const BRAZILIAN: Table = &[
    // Operators and conversion (`e` is Euler's number unless it stands between two operands;
    // `em` is the CSS unit unless a unit follows)
    (
        &[
            "3 vezes 4",
            "8 dividido por 2",
            "10 por 4",
            "7 multiplicado por 3",
        ],
        &[Text("12"), Text("4"), Text("2,5"), Text("21")],
    ),
    (
        &[
            "5 e 3",
            "10 menos 3",
            "10 sem 4",
            "5 € com 2 €",
            "2 + e",
            "10 módulo 3",
        ],
        &[
            Text("8"),
            Text("7"),
            Text("6"),
            Text("€ 7"),
            Text("4,72"),
            Text("1"),
        ],
    ),
    (
        &[
            "5 polegadas em cm",
            "5 polegadas para cm",
            "1 milha como km",
            "1 milha no km",
            "2 em",
            "2 em para px",
        ],
        &[
            Text("12,7 cm"),
            Text("12,7 cm"),
            Text("1,61 km"),
            Text("1,61 km"),
            Text("2 em"),
            Text("32 px"),
        ],
    ),
    (
        &["60 km por hora", "120 km / 2 horas", "10 reais por hora"],
        &[Text("60 km/h"), Text("60 km/h"), Text("10 BRL/h")],
    ),
    (
        &[
            "x é igual a 5",
            "x vezes 2",
            "aluguel é 800 €",
            "aluguel dividido por 2",
        ],
        &[Text("5"), Text("10"), Text("€ 800"), Text("€ 400")],
    ),
    // Percentages
    (
        &[
            "20 por cento de 50",
            "20 % de 50",
            "5 % a mais em 30 €",
            "6 % de desconto em 40 €",
        ],
        &[Text("10"), Text("10"), Text("€ 31,50"), Text("€ 37,60")],
    ),
    (
        &[
            "10 % de acréscimo sobre 50",
            "10 % a menos de 50",
            "100 € mais 19 %",
        ],
        &[Text("55"), Text("45"), Text("€ 119")],
    ),
    (
        &[
            "25 como % de 50",
            "75 como % de aumento sobre 50",
            "25 como % de desconto de 50",
            "25 como uma porcentagem de 50",
        ],
        &[Text("50%"), Text("50%"), Text("50%"), Text("50%")],
    ),
    (
        &[
            "20 % de quanto é 10",
            "25 % sobre quanto é 125",
            "5 % de quanto é 6 €",
            "20 % de desconto sobre quanto é 80",
        ],
        &[Text("50"), Text("100"), Text("€ 120"), Text("100")],
    ),
    // Scales (short scale in Brazil; m stays metre)
    (
        &[
            "2 milhões",
            "3 bilhões",
            "1,5 mil",
            "2 trilhões",
            "2 milhões de reais",
            "2 m",
        ],
        &[
            Text("2.000.000"),
            Text("3.000.000.000"),
            Text("1.500"),
            Text("2.000.000.000.000"),
            Text("2.000.000 BRL"),
            Text("2 m"),
        ],
    ),
    (
        &["3 mil milhões", "5 mil reais", "R$ 50", "2 bi"],
        &[
            Text("3.000.000.000"),
            Text("5.000 BRL"),
            Text("50 BRL"),
            Text("2.000.000.000"),
        ],
    ),
    // Dates and times
    (
        &[
            "hoje",
            "hoje + 2 semanas",
            "amanhã",
            "ontem",
            "depois de amanhã",
            "anteontem",
        ],
        &[
            Text("qua., 28 de out. de 2026"),
            Text("qua., 11 de nov. de 2026"),
            Text("qui., 29 de out. de 2026"),
            Text("ter., 27 de out. de 2026"),
            Text("sex., 30 de out. de 2026"),
            Text("seg., 26 de out. de 2026"),
        ],
    ),
    (
        &["agora", "hora", "tempo", "meio-dia", "hora + 2 horas"],
        &[
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("12:00"),
            Text("15:00"),
        ],
    ),
    (
        &[
            "tempo em Viena",
            "hora em Moscou",
            "agora em Pequim",
            "14:00 em Nova York",
            "Zurique tempo",
            "tempo no Cairo",
        ],
        &[
            Text("13:00"),
            Text("15:00"),
            Text("20:00"),
            Text("09:00"),
            Text("13:00"),
            Text("15:00"),
        ],
    ),
    (
        &["12 março 2026", "3 out. 2026", "1 maio", "2026-12-24 - hoje"],
        &[
            Text("qui., 12 de mar. de 2026"),
            Text("sáb., 3 de out. de 2026"),
            Text("sex., 1 de mai. de 2026"),
            Text("57 dias"),
        ],
    ),
    (
        &["12 de março de 2026", "1 de maio"],
        &[Text("qui., 12 de mar. de 2026"), Text("sex., 1 de mai. de 2026")],
    ),
    (
        &[
            "14:00 em Lisboa",
            "tempo em Londres",
            "tempo em Varsóvia",
            "tempo em Bruxelas",
            "tempo em Copenhague",
            "tempo em Genebra",
            "tempo em Munique",
            "hora em Tóquio",
        ],
        &[
            Text("13:00"),
            Text("12:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("13:00"),
            Text("21:00"),
        ],
    ),
    // Document references
    (
        &["Aluguel: 850 €", "Luz: 64 €", "soma"],
        &[Text("€ 850"), Text("€ 64"), Text("€ 914")],
    ),
    (
        &["Aluguel: 850 €", "Luz: 64 €", "média", "méd."],
        &[Text("€ 850"), Text("€ 64"), Text("€ 457"), Text("€ 457")],
    ),
    (
        &["4 €", "6 €", "média", "anterior * 2", "Total: soma"],
        &[Text("€ 4"), Text("€ 6"), Text("€ 5"), Text("€ 10"), Text("€ 25")],
    ),
    // Units and localized output (Brazil: singular below 2, `1,5 hora`)
    (
        &[
            "1 mês em dias",
            "1 dia",
            "2 horas",
            "5 polegadas",
            "3 pés",
            "90 minutos em horas",
        ],
        &[
            Text("30,42 dias"),
            Text("1 dia"),
            Text("2 horas"),
            Text("5 polegadas"),
            Text("3 pés"),
            Text("1,5 hora"),
        ],
    ),
    (
        &[
            "1 hora e 30 minutos",
            "2 semanas",
            "1 ano",
            "2 h",
            "10 seg",
            "1 século",
        ],
        &[
            Text("90 minutos"),
            Text("2 semanas"),
            Text("1 ano"),
            Text("2 h"),
            Text("10 segundos"),
            Text("1 século"),
        ],
    ),
    (
        &[
            "20 metros quadrados",
            "3 centímetros cúbicos",
            "2 pés quadrados",
            "5 polegadas cúbicas",
            "10 hectares em m²",
        ],
        &[
            Text("20 m²"),
            Text("3 cm³"),
            Text("2 ft²"),
            Text("5 in³"),
            Text("100.000 m²"),
        ],
    ),
    (
        &[
            "5 quilômetros em milhas",
            "250 gramas em onças",
            "2 litros em galões",
            "3 xícaras em mililitros",
        ],
        &[
            Text("3,11 milhas"),
            Text("8,82 onças"),
            Text("0,53 galão"),
            Text("709,76 ml"),
        ],
    ),
    (
        &[
            "100 graus Celsius em Fahrenheit",
            "90 graus em radianos",
            "1 GiB em MB",
            "5 quilos em libras",
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
            "5 libras em kg",
            "5 libras em euros",
            "£5 + 3 libras",
            "2 quintais em kg",
        ],
        &[Text("2,27 kg"), Text("€ 5,33"), Text("£ 8"), Text("200 kg")],
    ),
    (
        &[
            "2 colheres de sopa em ml",
            "1 colher de chá",
            "10 km em milhas náuticas",
            "3 nós",
            "1 quinzena em dias",
            "1 ano-luz em km",
        ],
        &[
            Text("29,57 ml"),
            Text("1 colher de chá"),
            Text("5,4 milhas náuticas"),
            Text("3 nós"),
            Text("14 dias"),
            Text("9.460.730.472.580,8 km"),
        ],
    ),
    // Currencies
    (
        &[
            "100 dólares em euros",
            "50 libras esterlinas em euros",
            "20 francos suíços em dólares",
            "1000 ienes em dólares",
        ],
        &[Text("€ 80"), Text("€ 53,33"), Text("$ 22,22"), Text("$ 6,67")],
    ),
    // Functions and formats
    (
        &[
            "raiz quadrada 16",
            "raiz quadrada de 16",
            "raiz cúbica 27",
            "255 em hexadecimal",
            "5 em binário",
            "módulo -5",
        ],
        &[
            Text("4"),
            Text("4"),
            Text("3"),
            Text("0xFF"),
            Text("0b101"),
            Text("5"),
        ],
    ),
    // `h` between hours and minutes, the ordinal day
    (
        &["14h30", "9h05", "14h", "1º de maio", "24-12-2026"],
        &[
            Text("14:30"),
            Text("09:05"),
            Text("14 h"),
            Text("sex., 1 de mai. de 2026"),
            Text("qui., 24 de dez. de 2026"),
        ],
    ),
    // Prose stays prose
    (
        &["Bom dia", "Isso custa 5 €", "Vou para casa às 14:00"],
        &[Nothing, Text("€ 5"), Text("14:00")],
    ),
];

/// Portugal: long scale, `1,5 horas`, `chávena`, `quarta, 28/10/2026`.
const EUROPEAN: Table = &[
    (
        &["2 biliões", "3 mil milhões", "2 bilhões", "2 milhões de euros"],
        &[
            Text("2.000.000.000.000"),
            Text("3.000.000.000"),
            Text("2.000.000.000"),
            Text("2.000.000 €"),
        ],
    ),
    (
        &[
            "hoje",
            "amanhã",
            "90 minutos em horas",
            "1 hora",
            "3 xícaras",
            "3 chávenas",
        ],
        &[
            Text("quarta, 28/10/2026"),
            Text("quinta, 29/10/2026"),
            Text("1,5 horas"),
            Text("1 hora"),
            Text("3 chávenas"),
            Text("3 chávenas"),
        ],
    ),
    (
        &[
            "agora em Moscovo",
            "14:00 em Nova Iorque",
            "tempo em Copenhaga",
            "2 m",
            "50 libras em euros",
        ],
        &[
            Text("15:00"),
            Text("09:00"),
            Text("13:00"),
            Text("2 m"),
            Text("53,33 €"),
        ],
    ),
];

#[test]
fn brazilian_portuguese() {
    assert_table(InputLanguage::PortugueseBrazil, NumberStyle::German, BRAZILIAN);
}

#[test]
fn european_portuguese() {
    assert_table(InputLanguage::PortuguesePortugal, NumberStyle::German, EUROPEAN);
}
