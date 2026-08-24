use std::collections::HashMap;
use vir_search::ast::{Expr, FieldType, MatchKind, ParseField, ParseSort, ParseState, SortSpec};
use vir_search::parse::{PerspectiveResolver, parse, parse_with_resolver};

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
fn unbalanced_parens_degrade_whole_input() {
    let p = parse::<TestField, TestState, TestSort>("(genre:ambient");
    assert_eq!(p.expr, Expr::Text("(genre:ambient".into()));
    assert!(!p.warnings.is_empty());
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
