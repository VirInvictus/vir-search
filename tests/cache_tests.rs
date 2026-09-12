use std::collections::HashSet;
use vir_search::QueryCache;
use vir_search::ast::{Expr, FieldType, ParseField, ParseSort, ParseState, Value};
use vir_search::parse::parse;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TestField {
    Genre,
    Rating,
    Duration,
}
impl std::fmt::Display for TestField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Genre => "genre",
            Self::Rating => "rating",
            Self::Duration => "duration",
        };
        write!(f, "{s}")
    }
}
impl ParseField for TestField {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "genre" => Some(Self::Genre),
            "rating" => Some(Self::Rating),
            "duration" => Some(Self::Duration),
            _ => None,
        }
    }
    fn field_type(&self) -> FieldType {
        match self {
            Self::Rating => FieldType::Int,
            Self::Duration => FieldType::Real,
            _ => FieldType::String,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

type Parsed = vir_search::parse::ParseResult<TestField, TestState, TestSort>;

fn parsed(input: &str) -> Parsed {
    parse::<TestField, TestState, TestSort>(input)
}

#[test]
fn real_literal_queries_bypass_the_cache() {
    let mut cache: QueryCache<TestField, TestState, TestSort> = QueryCache::new(8);
    // A decimal literal, a duration chain, and a range over reals: all carry
    // Value::Real, so none is stored, but all still parse correctly.
    for input in ["duration:>600.5", "duration:>=1h30m", "duration:90m..2h"] {
        let r = cache.parse(input);
        assert!(
            matches!(r.expr, Expr::Compare { .. } | Expr::Range { .. }),
            "{input} should parse to a typed node"
        );
        assert!(!cache.cached(input), "{input} must not be cached");
    }
    assert_eq!(cache.len(), 0);
}

#[test]
fn plain_queries_cache_and_replay_identically() {
    let mut cache: QueryCache<TestField, TestState, TestSort> = QueryCache::new(8);
    let first = cache.parse("genre:ambient rating:>=4");
    assert!(cache.cached("genre:ambient rating:>=4"));
    let second = cache.parse("genre:ambient rating:>=4");
    assert_eq!(first.expr, second.expr);
    assert_eq!(first.warnings, second.warnings);
    assert_eq!(first.diagnostics, second.diagnostics);
    assert_eq!(cache.len(), 1);
    // A different query gets its own entry.
    cache.parse("genre:jazz");
    assert_eq!(cache.len(), 2);
}

#[test]
fn lru_evicts_the_least_recently_used() {
    let mut cache: QueryCache<TestField, TestState, TestSort> = QueryCache::new(2);
    cache.parse("genre:ambient");
    cache.parse("genre:jazz");
    // Touching the oldest entry makes the middle one the eviction victim.
    cache.parse("genre:ambient");
    cache.parse("genre:metal");
    assert_eq!(cache.len(), 2);
    assert!(cache.cached("genre:ambient"));
    assert!(cache.cached("genre:metal"));
    assert!(!cache.cached("genre:jazz"));
    // Eviction loses nothing: the evicted query still parses.
    assert_eq!(cache.parse("genre:jazz").expr, parsed("genre:jazz").expr);
}

#[test]
fn real_carrying_cache_entries_free_capacity() {
    // A capacity-1 cache: a real query consumes no slot, so the plain query
    // that follows still owns the whole cache.
    let mut cache: QueryCache<TestField, TestState, TestSort> = QueryCache::new(1);
    cache.parse("duration:>600.5");
    assert_eq!(cache.len(), 0);
    cache.parse("genre:ambient");
    assert_eq!(cache.len(), 1);
    assert!(cache.cached("genre:ambient"));
}

#[test]
fn hashable_asts_dedupe_in_a_set() {
    let mut set = HashSet::new();
    set.insert(parsed("genre:ambient AND rating:>=4").expr);
    set.insert(parsed("genre:ambient AND rating:>=4").expr);
    assert_eq!(set.len(), 1);
    set.insert(parsed("genre:jazz AND rating:>=4").expr);
    assert_eq!(set.len(), 2);
}

#[test]
fn real_queries_hash_and_stay_distinct() {
    // Hashing a real-carrying tree never panics (only the discriminant is
    // hashed), and equality still tells distinct reals apart.
    let mut set = HashSet::new();
    set.insert(parsed("duration:>600.5").expr);
    set.insert(parsed("duration:>90m").expr);
    assert_eq!(set.len(), 2);
    // The identical query dedupes as usual.
    set.insert(parsed("duration:>600.5").expr);
    assert_eq!(set.len(), 2);
}

#[test]
fn contains_real_flags_the_whole_tree() {
    assert!(!parsed("genre:ambient").expr.contains_real());
    assert!(!parsed("rating:>=4").expr.contains_real()); // Int is not Real
    assert!(parsed("duration:>90m").expr.contains_real());
    assert!(parsed("duration:90m..2h").expr.contains_real());
    assert!(
        parsed("genre:ambient OR duration:>90m")
            .expr
            .contains_real()
    );
    assert!(parsed("NOT duration:>90m").expr.contains_real());
    // nan parses as a real literal, which is exactly why Eq on the tree is
    // documented as bit equality and real queries stay out of caches.
    let p = parsed("duration:nan");
    assert!(matches!(
        p.expr,
        Expr::Compare {
            value: Value::Real(_),
            ..
        }
    ));
    assert!(p.expr.contains_real());
}

#[test]
fn int_and_date_comparators_hash_and_dedupe() {
    let mut set = HashSet::new();
    set.insert(parsed("rating:>=4").expr);
    set.insert(parsed("added:thisweek").expr);
    assert_eq!(set.len(), 2);
    assert!(set.contains(&parsed("rating:>=4").expr));
}
