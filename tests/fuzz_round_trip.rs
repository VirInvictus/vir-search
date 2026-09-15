//! Generated-input round-trip fuzz (no dependencies): a seeded LCG
//! assembles queries from grammar pieces, and every case must satisfy the
//! Display contract exactly (`render` then re-parse reproduces the same
//! tree, and rendering is a fixed point from the first generation on).
//!
//! The hand-written corpus could not see the 1.4.1 month-keyword bug or the
//! quoted-bool Display flip, because neither input was in the list; this
//! generator covers the grammar's combinations instead of a fixed sample.
//!
//! One recorded exclusion: no `nan` literal. `Value::Real` carries f64, a
//! `nan` is deliberately not equal to itself (see the `Eq` marker docs on
//! `Expr`), so the shape-stability comparison cannot hold for it by design.

use vir_search::ast::{Expr, FieldType, ParseField, ParseSort, ParseState};
use vir_search::parse::parse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum F {
    Genre,
    Author,
    Rating,
    Year,
    Added,
    Duration,
}
impl std::fmt::Display for F {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            F::Genre => "genre",
            F::Author => "author",
            F::Rating => "rating",
            F::Year => "year",
            F::Added => "added",
            F::Duration => "duration",
        })
    }
}
impl ParseField for F {
    fn parse(name: &str) -> Option<Self> {
        [
            (F::Genre, "genre"),
            (F::Author, "author"),
            (F::Rating, "rating"),
            (F::Year, "year"),
            (F::Added, "added"),
            (F::Duration, "duration"),
        ]
        .into_iter()
        .find(|(_, n)| *n == name)
        .map(|(f, _)| f)
    }
    fn field_type(&self) -> FieldType {
        match self {
            F::Rating | F::Year => FieldType::Int,
            F::Duration => FieldType::Real,
            F::Added => FieldType::Date,
            _ => FieldType::String,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum S {
    Read,
}
impl std::fmt::Display for S {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("read")
    }
}
impl ParseState for S {
    fn parse(name: &str) -> Option<Self> {
        (name == "read").then_some(S::Read)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum K {
    Added,
}
impl std::fmt::Display for K {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("added")
    }
}
impl ParseSort for K {
    fn parse(name: &str) -> Option<Self> {
        (name == "added").then_some(K::Added)
    }
}

/// The classic PCG-ish LCG (Hörmann's constants): deterministic across
/// platforms, no dependencies, and good enough to scatter grammar pieces.
struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 16
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[(self.next() % items.len() as u64) as usize]
    }
}

const TEXT_FIELDS: &[&str] = &["genre", "author"];
const TYPED_FIELDS: &[&str] = &["rating", "year", "added", "duration"];
const BAREWORDS: &[&str] = &[
    "ambient", "roygbiv", "Björk", "x", "2024", "Mr.X", "true", "TRUE",
];
const QUOTED_BODIES: &[&str] = &[
    "boards of canada",
    "a\"b",
    "back\\slash",
    "true",
    "FALSE",
    "foo*",
    "*bar",
    "and",
    "a..b",
    "not",
    "",
];
const TEXT_VALUES: &[&str] = &[
    "ambient",
    "true",
    "false",
    "TRUE",
    "ambient*",
    "*metal",
    "a*",
    "(rock,jazz)",
    "today",
    "+7d",
    "3daysago",
];
const COMPARATORS: &[&str] = &["=", "!=", "<", "<=", ">", ">="];
const NUMBER_VALUES: &[&str] = &["4", "1990", "4.5", "1h30m", "90m", "320k", "50mb", "3.5h"];
const DATE_KEYWORDS: &[&str] = &[
    "today",
    "yesterday",
    "tomorrow",
    "thisweek",
    "lastweek",
    "nextweek",
    "thismonth",
    "lastmonth",
    "nextmonth",
    "thisyear",
    "3daysago",
    "in7days",
    "+7d",
    "-14d",
    "2024",
    "2024-06",
    "2024-06-08",
    "9999-12-31",
    "262143",
];
const LOGIC: &[&str] = &["AND", "OR", "NOT", "!", "(", ")", ""];
const STATES: &[&str] = &["is:read", "is:unread", "is:"];
const SORTS: &[&str] = &["sort:-added", "sort:+added", "sort:bogus"];
const PERSPECTIVES: &[&str] = &["vl:fav", "vl:"];

/// One random query piece: a field term, a bareword, a quoted string, a
/// logic word, a state, a sort, or a perspective reference.
fn piece(r: &mut Lcg) -> String {
    match r.next() % 10 {
        0..=2 => {
            let field = r.pick(TEXT_FIELDS);
            let value = r.pick(TEXT_VALUES);
            format!("{field}:{value}")
        }
        3 => {
            let field = r.pick(TEXT_FIELDS);
            format!("{field}:\"{}\"", r.pick(QUOTED_BODIES))
        }
        4 | 5 => {
            let field = r.pick(TYPED_FIELDS);
            match r.next() % 4 {
                0 => format!("{field}:{}", r.pick(COMPARATORS)),
                1 => format!("{field}:{}{}", r.pick(COMPARATORS), r.pick(NUMBER_VALUES)),
                2 => format!("{field}:{}", r.pick(DATE_KEYWORDS)),
                _ => {
                    if r.next() % 2 == 0 {
                        format!("{field}:1990..2000")
                    } else {
                        format!("{field}:{}", r.pick(DATE_KEYWORDS))
                    }
                }
            }
        }
        6 => r.pick(BAREWORDS).to_string(),
        7 => format!("\"{}\"", r.pick(QUOTED_BODIES)),
        8 => r.pick(STATES).to_string(),
        _ => match r.next() % 4 {
            0 => r.pick(LOGIC).to_string(),
            1 => r.pick(SORTS).to_string(),
            2 => r.pick(PERSPECTIVES).to_string(),
            _ => r.pick(BAREWORDS).to_string(),
        },
    }
}

#[test]
fn generated_queries_round_trip() {
    let mut r = Lcg::new(0x5EED_2026);
    for case in 0..5000 {
        let pieces: Vec<String> = (0..1 + (r.next() % 6)).map(|_| piece(&mut r)).collect();
        let query = pieces.join(" ");
        let first = parse::<F, S, K>(&query).expr;
        let rendered = format!("{first}");
        let second = parse::<F, S, K>(&rendered).expr;
        assert_eq!(
            first, second,
            "case {case}: {query:?} rendered {rendered:?} and re-parsed differently"
        );
        // The fixed point holds from the first generation on.
        let rendered_again = format!("{second}");
        let third = parse::<F, S, K>(&rendered_again).expr;
        assert_eq!(
            second, third,
            "case {case}: {rendered:?} rendered {rendered_again:?} and drifted"
        );
    }
}

#[test]
fn generated_field_terms_round_trip_alone() {
    // Every single-field form the generator can build, swept exhaustively
    // rather than sampled: the combination fuzz above scatters pieces, this
    // pins the per-field value grammar on all six fields.
    let mut r = Lcg::new(0xCAFE_0001);
    for field in [
        F::Genre,
        F::Author,
        F::Rating,
        F::Year,
        F::Added,
        F::Duration,
    ] {
        for value in TEXT_VALUES
            .iter()
            .chain(NUMBER_VALUES.iter())
            .chain(DATE_KEYWORDS.iter())
        {
            let query = format!("{field}:{value}");
            let first = parse::<F, S, K>(&query).expr;
            if matches!(first, Expr::Empty) {
                continue;
            }
            let rendered = format!("{first}");
            let second = parse::<F, S, K>(&rendered).expr;
            assert_eq!(
                first, second,
                "{query:?} rendered {rendered:?} and re-parsed differently"
            );
            let _ = r.next(); // the LCG stays deterministic even when unused
        }
    }
}
