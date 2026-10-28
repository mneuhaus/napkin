//! Korean: `ko`.

use napkin_core::{InputLanguage, NumberStyle};

use crate::Expect::{Nothing, Text};
use crate::{Table, assert_table};

const KOREAN: Table = &[
    // Operators and conversion
    (
        &[
            "3 곱하기 4",
            "8 나누기 2",
            "10 나누기 4",
            "5 더하기 3",
            "10 빼기 3",
            "5 플러스 3",
            "10 마이너스 4",
        ],
        &[
            Text("12"),
            Text("4"),
            Text("2.5"),
            Text("8"),
            Text("7"),
            Text("8"),
            Text("6"),
        ],
    ),
    (
        &[
            "5인치는 몇 센티미터",
            "5인치 몇 cm",
            "9달러는 몇 유로",
            "5 inch in cm",
        ],
        &[Text("12.7 cm"), Text("12.7 cm"), Text("€7.20"), Text("12.7 cm")],
    ),
    (
        &["x는 5", "x 곱하기 2", "가격 = 5000원", "가격 * 2"],
        &[Text("5"), Text("10"), Text("5,000 KRW"), Text("10,000 KRW")],
    ),
    // Percentages: the base comes first (`50의 20%`)
    (
        &[
            "50의 20%",
            "30000원의 15%",
            "30000원 - 15%",
            "20퍼센트",
            "20 프로",
        ],
        &[
            Text("10"),
            Text("4,500 KRW"),
            Text("25,500 KRW"),
            Text("20%"),
            Text("20%"),
        ],
    ),
    // Scales: 만 is 10^4, 억 10^8, 조 10^12
    (
        &["3억 원", "5만", "3천", "1.5만", "2조", "3천만 원"],
        &[
            Text("300,000,000 KRW"),
            Text("50,000"),
            Text("3,000"),
            Text("15,000"),
            Text("2,000,000,000,000"),
            Text("30,000,000 KRW"),
        ],
    ),
    // Dates and times
    (
        &["오늘", "오늘 + 2주", "내일", "어제", "모레", "그저께"],
        &[
            Text("2026년 10월 28일 (수)"),
            Text("2026년 11월 11일 (수)"),
            Text("2026년 10월 29일 (목)"),
            Text("2026년 10월 27일 (화)"),
            Text("2026년 10월 30일 (금)"),
            Text("2026년 10월 26일 (월)"),
        ],
    ),
    (
        &["지금", "시간", "3시", "15시", "3시 + 2시간", "정오", "자정"],
        &[
            Text("13:00"),
            Text("13:00"),
            Text("03:00"),
            Text("15:00"),
            Text("05:00"),
            Text("12:00"),
            Text("00:00"),
        ],
    ),
    (
        &[
            "도쿄 시간",
            "서울 시간",
            "뉴욕 지금",
            "15시 in 런던",
            "지금 in 서울",
        ],
        &[
            Text("21:00"),
            Text("21:00"),
            Text("08:00"),
            Text("14:00"),
            Text("21:00"),
        ],
    ),
    (
        &["2026년 3월 12일", "3월 12일", "2026-12-24 - 오늘"],
        &[
            Text("2026년 3월 12일 (목)"),
            Text("2026년 3월 12일 (목)"),
            Text("57 일"),
        ],
    ),
    // Document references
    (
        &["월세: 850000원", "관리비: 120000원", "합계"],
        &[Text("850,000 KRW"), Text("120,000 KRW"), Text("970,000 KRW")],
    ),
    (
        &["월세: 850000원", "관리비: 120000원", "평균", "이전 / 2"],
        &[
            Text("850,000 KRW"),
            Text("120,000 KRW"),
            Text("485,000 KRW"),
            Text("242,500 KRW"),
        ],
    ),
    // Units and localized output (one form for every count)
    (
        &[
            "1개월 in 일",
            "1 일",
            "2 시간",
            "5 인치",
            "3 피트",
            "90분은 몇 시간",
        ],
        &[
            Text("30.42 일"),
            Text("1 일"),
            Text("2 시간"),
            Text("5 인치"),
            Text("3 피트"),
            Text("1.5 시간"),
        ],
    ),
    (
        &["1시간 30분", "3일", "2주", "1년", "10초", "3개월", "3달"],
        &[
            Text("90 분"),
            Text("3 일"),
            Text("2 주"),
            Text("1 년"),
            Text("10 초"),
            Text("3 개월"),
            Text("3 개월"),
        ],
    ),
    (
        &[
            "20제곱미터",
            "3세제곱센티미터",
            "2제곱피트",
            "100제곱미터 in 평",
            "30평은 몇 제곱미터",
        ],
        &[
            Text("20 m²"),
            Text("3 cm³"),
            Text("2 제곱피트"),
            Text("30.25 평"),
            Text("99.17 m²"),
        ],
    ),
    (
        &[
            "5킬로미터는 몇 마일",
            "250그램은 몇 온스",
            "2리터는 몇 갤런",
            "3컵 in 밀리리터",
            "3큰술 in 작은술",
        ],
        &[
            Text("3.11 마일"),
            Text("8.82 온스"),
            Text("0.53 갤런"),
            Text("709.76 ml"),
            Text("9 작은술"),
        ],
    ),
    (
        &[
            "100섭씨 in 화씨",
            "90도 in 라디안",
            "1기가바이트 in 메가바이트",
            "5킬로그램은 몇 파운드",
            "5파운드 in kg",
        ],
        &[
            Text("212 °F"),
            Text("1.57 rad"),
            Text("1,000 MB"),
            Text("11.02 파운드"),
            Text("2.27 kg"),
        ],
    ),
    // Currencies (`파운드` is sterling on money lines)
    (
        &[
            "£5 + 3파운드",
            "1000엔 in 달러",
            "100달러는 몇 엔",
            "100불 in 유로",
        ],
        &[Text("£8"), Text("$6.67"), Text("¥15,000"), Text("€80")],
    ),
    // Functions and formats
    (
        &["제곱근 16", "세제곱근 27", "255 in 십육진수"],
        &[Text("4"), Text("3"), Text("0xFF")],
    ),
    // Sino-Korean numerals need a multiplier, and two syllables when they stand alone
    (
        &[
            "삼백",
            "오천원",
            "만원",
            "삼백만원",
            "이십분",
            "삼십 분",
            "십만",
            "5십만",
            "5만원",
        ],
        &[
            Text("300"),
            Text("5,000 KRW"),
            Text("10,000 KRW"),
            Text("3,000,000 KRW"),
            Text("20 분"),
            Text("30 분"),
            Text("100,000"),
            Text("500,000"),
            Text("50,000 KRW"),
        ],
    ),
    (
        &["이사", "사이", "천사", "백조", "천만에요", "3월 12일", "１２３"],
        &[
            Nothing,
            Nothing,
            Nothing,
            Nothing,
            Nothing,
            Text("2026년 3월 12일 (목)"),
            Text("123"),
        ],
    ),
    // Prose stays prose
    (&["안녕하세요", "사과 3개"], &[Nothing, Text("3")]),
];

#[test]
fn korean() {
    assert_table(InputLanguage::Korean, NumberStyle::English, KOREAN);
}
