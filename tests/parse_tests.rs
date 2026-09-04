use std::collections::HashMap;
use vir_search::ast::{
    Comparator, DateSpec, Expr, FieldType, MatchKind, ParseField, ParseSort, ParseState, SortSpec,
    Value,
};
use vir_search::parse::{Diagnostic, PerspectiveResolver, parse, parse_with_resolver};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestField {
    Genre,
    Artist,
    Album,
    Title,
    Rating,
    Year,
    Added,
    Format,
    Duration,
    Author,
    Narrator,
    Series,
}
impl std::fmt::Display for TestField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Genre => "genre",
            Self::Artist => "artist",
            Self::Album => "album",
            Self::Title => "title",
            Self::Rating => "rating",
            Self::Year => "year",
            Self::Added => "added",
            Self::Format => "format",
            Self::Duration => "duration",
            Self::Author => "author",
            Self::Narrator => "narrator",
            Self::Series => "series",
        };
        write!(f, "{s}")
    }
}
impl ParseField for TestField {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "genre" => Some(Self::Genre),
            "artist" => Some(Self::Artist),
            "album" => Some(Self::Album),
            "title" => Some(Self::Title),
            "rating" => Some(Self::Rating),
            "year" => Some(Self::Year),
            "added" => Some(Self::Added),
            "format" => Some(Self::Format),
            "duration" => Some(Self::Duration),
            "author" => Some(Self::Author),
            "narrator" => Some(Self::Narrator),
            "series" => Some(Self::Series),
            _ => None,
        }
    }
    fn field_type(&self) -> FieldType {
        match self {
            Self::Rating | Self::Year => FieldType::Int,
            Self::Duration => FieldType::Real,
            Self::Added => FieldType::Date,
            _ => FieldType::String,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestState {
    Starred,
    Finished,
}
impl std::fmt::Display for TestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Starred => "starred",
                Self::Finished => "finished",
            }
        )
    }
}
impl ParseState for TestState {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "starred" => Some(Self::Starred),
            "finished" => Some(Self::Finished),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestSort {
    Added,
}
impl std::fmt::Display for TestSort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "added")
    }
}
impl ParseSort for TestSort {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "added" => Some(Self::Added),
            _ => None,
        }
    }
}

fn round_trip(input: &str) {
    let first = parse::<TestField, TestState, TestSort>(input).expr;
    let rendered = format!("{first}");
    let second = parse::<TestField, TestState, TestSort>(&rendered).expr;
    assert_eq!(
        first, second,
        "round-trip changed {input:?} -> {rendered:?}"
    );
}

#[test]
fn round_trips() {
    for input in [
        "roygbiv",
        "artist:boards",
        "artist:=\"Boards of Canada\"",
        "title:~rx",
        "genre:?ambiant",
        "rating:>=4",
        "year:1990..2000",
        "added:thisweek",
        "format:flac genre:ambient",
        "genre:jazz OR genre:ambient",
        "NOT is:starred",
        "(genre:ambient OR genre:jazz) AND rating:>=4",
        "rating:true",
        "duration:>600",
        "author:sanderson",
        "narrator:=\"Kate Reading\"",
        "series:?stormlite",
        "is:finished",
        "author:tolkien AND NOT is:finished",
        "title:~\"^(a|b)$\"",
        "title:~\"live!\"",
        "artist:\"AC=DC\"",
        "album:\"What?\"",
        "title:\"a..b\"",
        "\"or\" boards",
        "\"not\"",
        "added:tomorrow",
        "added:lastweek",
        "added:nextweek",
        "NOT NOT is:starred",
        "genre:ambient AND",
        "author:>=Sanderson",
        "a AND !",
        "title:\"a\\\\b\"",
    ] {
        round_trip(input);
    }
}

#[test]
fn extracts_sort() {
    let p = parse::<TestField, TestState, TestSort>("genre:ambient sort:-added");
    assert_eq!(
        p.sorts,
        vec![SortSpec {
            key: TestSort::Added,
            descending: true
        }]
    );
    assert_eq!(
        p.expr,
        parse::<TestField, TestState, TestSort>("genre:ambient").expr
    );
}

#[test]
fn unknown_field_degrades_to_text() {
    let p = parse::<TestField, TestState, TestSort>("bogus:value");
    assert_eq!(p.expr, Expr::Text("bogus:value".into()));
    assert!(!p.warnings.is_empty());
}

#[test]
fn unbalanced_parens_keep_parsed_content() {
    let p = parse::<TestField, TestState, TestSort>("(genre:ambient");
    assert_eq!(
        p.expr,
        parse::<TestField, TestState, TestSort>("genre:ambient").expr
    );
    assert!(!p.warnings.is_empty());
}

#[test]
fn date_keywords_parse() {
    for (kw, spec) in [
        ("tomorrow", DateSpec::Tomorrow),
        ("lastweek", DateSpec::LastWeek),
        ("nextweek", DateSpec::NextWeek),
    ] {
        let p = parse::<TestField, TestState, TestSort>(&format!("added:{kw}"));
        assert_eq!(
            p.expr,
            Expr::Compare {
                field: TestField::Added,
                comp: Comparator::Eq,
                value: Value::Date(spec),
            },
            "date keyword {kw} did not parse"
        );
    }
}

#[test]
fn double_negation_negates_twice() {
    let p = parse::<TestField, TestState, TestSort>("NOT NOT is:starred");
    assert_eq!(
        p.expr,
        Expr::Not(Box::new(Expr::Not(Box::new(Expr::State(
            TestState::Starred
        )))))
    );
}

#[test]
fn dangling_operator_keeps_parsed_side() {
    for input in ["genre:ambient AND", "genre:ambient OR"] {
        let p = parse::<TestField, TestState, TestSort>(input);
        assert_eq!(
            p.expr,
            parse::<TestField, TestState, TestSort>("genre:ambient").expr,
            "trailing operator collapsed {input:?}"
        );
    }
}

#[test]
fn missing_value_degrades_to_visible_text() {
    for input in ["genre:", "title:=", "added:", "author:>="] {
        let p = parse::<TestField, TestState, TestSort>(input);
        assert_eq!(
            p.expr,
            Expr::Text(input.into()),
            "missing value collapsed {input:?}"
        );
        assert!(!p.warnings.is_empty());
    }
}

#[test]
fn quoted_bool_word_is_substring_not_presence() {
    let p = parse::<TestField, TestState, TestSort>("genre:\"true\"");
    assert_eq!(
        p.expr,
        Expr::Field {
            field: TestField::Genre,
            kind: MatchKind::Substring("true".into()),
        }
    );
    // The unquoted form is still the presence check.
    let p = parse::<TestField, TestState, TestSort>("genre:true");
    assert_eq!(
        p.expr,
        Expr::Field {
            field: TestField::Genre,
            kind: MatchKind::HasAny,
        }
    );
}

#[test]
fn relational_on_text_field_degrades_to_text() {
    let p = parse::<TestField, TestState, TestSort>("author:>=Sanderson");
    assert_eq!(p.expr, Expr::Text("author:>=Sanderson".into()));
    assert!(!p.warnings.is_empty());
    // The rest of the query survives; no whole-query collapse.
    let p = parse::<TestField, TestState, TestSort>("author:>=Sanderson genre:ambient");
    assert_eq!(
        p.expr,
        parse::<TestField, TestState, TestSort>("author:>=Sanderson AND genre:ambient").expr
    );
}

#[test]
fn standalone_punctuation_degrades_to_visible_text() {
    assert_eq!(
        parse::<TestField, TestState, TestSort>("?").expr,
        Expr::Text("?".into())
    );
    assert_eq!(
        parse::<TestField, TestState, TestSort>("..").expr,
        Expr::Text("..".into())
    );
    // A stray closer reads as nothing; that is its only sensible meaning.
    assert_eq!(
        parse::<TestField, TestState, TestSort>(")").expr,
        Expr::Empty
    );
}

#[test]
fn backslash_escapes_in_quoted_strings() {
    // `\\` is a literal backslash and no longer swallows the closing quote.
    let p = parse::<TestField, TestState, TestSort>("title:\"a\\\\b\"");
    assert_eq!(
        p.expr,
        Expr::Field {
            field: TestField::Title,
            kind: MatchKind::Substring("a\\b".into()),
        }
    );
    let p = parse::<TestField, TestState, TestSort>("album:\"c:\\\\\"");
    assert_eq!(
        p.expr,
        Expr::Field {
            field: TestField::Album,
            kind: MatchKind::Substring("c:\\".into()),
        }
    );
}

#[test]
fn empty_input_is_empty() {
    assert_eq!(
        parse::<TestField, TestState, TestSort>("").expr,
        Expr::Empty
    );
    assert_eq!(
        parse::<TestField, TestState, TestSort>("   ").expr,
        Expr::Empty
    );
}

#[test]
fn diagnostics_carry_byte_spans() {
    // The underline lands on the unknown field word, byte-accurate.
    let p = parse::<TestField, TestState, TestSort>("foo bogus:value");
    let d: &Diagnostic = &p.diagnostics[0];
    assert_eq!((d.start, d.end), (4, 9));
    assert!(d.message.contains("unknown field"));
    // Every diagnostic also appears in the flat warning log.
    assert!(p.warnings.iter().any(|w| w.contains("unknown field")));
}

#[test]
fn missing_value_diagnostics_point_at_the_field() {
    let p = parse::<TestField, TestState, TestSort>("genre:");
    assert_eq!((p.diagnostics[0].start, p.diagnostics[0].end), (0, 5));
}

#[test]
fn unclosed_paren_diagnostic_runs_to_the_end() {
    let p = parse::<TestField, TestState, TestSort>("(genre:ambient");
    let d = p
        .diagnostics
        .iter()
        .find(|d| d.message.contains("unclosed"))
        .expect("unclosed-paren diagnostic");
    assert_eq!((d.start, d.end), (0, 14));
}

#[test]
fn clean_parse_has_no_diagnostics() {
    let p = parse::<TestField, TestState, TestSort>("genre:ambient AND rating:>=4 sort:-added");
    assert!(p.diagnostics.is_empty());
}

struct Perspectives(HashMap<String, String>);
impl PerspectiveResolver<TestField, TestState> for Perspectives {
    fn expression(&self, name: &str) -> Option<String> {
        self.0.get(&name.to_lowercase()).cloned()
    }
}

#[test]
fn vl_expands_via_resolver() {
    let r = Perspectives(HashMap::from([("fav".into(), "genre:ambient".into())]));
    let p = parse_with_resolver::<TestField, TestState, TestSort, _>("vl:fav AND rating:>=4", &r);
    assert_eq!(
        p.expr,
        parse::<TestField, TestState, TestSort>("genre:ambient AND rating:>=4").expr
    );
    assert!(p.warnings.is_empty());
}

#[test]
fn vl_cycle_is_guarded() {
    let r = Perspectives(HashMap::from([
        ("a".into(), "vl:b".into()),
        ("b".into(), "vl:a".into()),
    ]));
    let p = parse_with_resolver::<TestField, TestState, TestSort, _>("vl:a", &r);
    assert_eq!(p.expr, Expr::Empty);
    assert!(p.warnings.iter().any(|w| w.contains("cycle")));
}

#[test]
fn vl_without_resolver_degrades_to_text() {
    assert_eq!(
        parse::<TestField, TestState, TestSort>("vl:fav").expr,
        Expr::Text("vl:fav".into())
    );
}
