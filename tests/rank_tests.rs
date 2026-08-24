use vir_search::ast::{FieldType, ParseField, ParseSort, ParseState};
use vir_search::parse::parse;
use vir_search::rank::{blend_relevance, collect_text_terms};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestField {
    Genre,
    Rating,
}
impl std::fmt::Display for TestField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Genre => "genre",
            Self::Rating => "rating",
        };
        write!(f, "{s}")
    }
}
impl ParseField for TestField {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "genre" => Some(Self::Genre),
            "rating" => Some(Self::Rating),
            _ => None,
        }
    }
    fn field_type(&self) -> FieldType {
        match self {
            Self::Rating => FieldType::Int,
            _ => FieldType::String,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestState {
    Starred,
}
impl std::fmt::Display for TestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "starred")
    }
}
impl ParseState for TestState {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "starred" => Some(Self::Starred),
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

#[test]
fn relevance_saturates_and_recency_decays() {
    assert!(blend_relevance(8.0, 0, 30.0) > blend_relevance(1.0, 0, 30.0));
    let fresh = blend_relevance(0.0, 0, 30.0);
    let half = blend_relevance(0.0, 30, 30.0);
    assert!((fresh - 0.25).abs() < 1e-9);
    assert!((half - 0.125).abs() < 1e-9);
}

#[test]
fn collects_only_bare_text() {
    let p = parse::<TestField, TestState, TestSort>("roygbiv boards genre:ambient rating:>=4");
    assert_eq!(collect_text_terms(&p.expr), vec!["roygbiv", "boards"]);
}
