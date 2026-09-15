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
        "genre:\"true\"",
        "genre:\"false\"",
        "genre:\"TRUE\"",
        "genre:\"ambient*\"",
        "genre:\"*metal\"",
        "(genre:ambient genre:jazz) (rating:>=4 is:starred)",
        "(genre:ambient OR genre:jazz) OR genre:rock",
        "genre:\"*metal\"",
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
    // A comparator rides along: one visible fragment, not three text nodes
    // the reader would have to reassemble.
    let p = parse::<TestField, TestState, TestSort>("bogus:>=5");
    assert_eq!(p.expr, Expr::Text("bogus:>=5".into()));
    assert!(!p.warnings.is_empty());
    round_trip("bogus:>=5");
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
fn month_keywords_parse() {
    for (kw, spec) in [
        ("lastmonth", DateSpec::LastMonth),
        ("nextmonth", DateSpec::NextMonth),
    ] {
        let p = parse::<TestField, TestState, TestSort>(&format!("added:{kw}"));
        assert_eq!(
            p.expr,
            Expr::Compare {
                field: TestField::Added,
                comp: Comparator::Eq,
                value: Value::Date(spec),
            },
            "month keyword {kw} did not parse"
        );
        // A clean parse: the keyword is resolved, not degraded with a warning.
        assert!(p.warnings.is_empty(), "{kw} should not warn");
    }
}

#[test]
fn negating_a_degraded_empty_operand_stays_empty() {
    // The round-trip fuzz's first catch: a `!` whose operand degraded to
    // Empty (a `sort:` directive extracts itself into no node) used to wrap
    // into Not(Empty), which renders a bare `NOT ` that steals the next
    // token on re-parse.
    let p = parse::<TestField, TestState, TestSort>("! sort:-added ambient");
    assert_eq!(
        p.expr,
        parse::<TestField, TestState, TestSort>("ambient").expr,
        "the vanished operand must not carry a negation onto its neighbor"
    );
    assert_eq!(p.sorts.len(), 1, "the sort directive is still extracted");
    for input in ["! sort:-added ambient", "!", "a AND !"] {
        round_trip(input);
    }
    // Real operands still negate.
    assert_eq!(
        parse::<TestField, TestState, TestSort>("NOT genre:ambient").expr,
        parse::<TestField, TestState, TestSort>("! genre:ambient").expr
    );
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
fn quoted_grammar_words_round_trip_as_substring() {
    // The 1.0.4 literal rule: a quoted value is a substring match, never the
    // presence or wildcard check the bare form means. Display used to render
    // these unquoted (`genre:"true"` -> `genre:true`), and the bare form
    // re-parses to HasAny/HasNone/Prefix/Suffix: a round-trip that flipped
    // meaning, so the corpus alone is not enough here; each case asserts the
    // semantics end to end.
    for (input, body) in [
        ("genre:\"true\"", "true"),
        ("genre:\"false\"", "false"),
        ("genre:\"TRUE\"", "TRUE"),
        ("genre:\"ambient*\"", "ambient*"),
        ("genre:\"*metal\"", "*metal"),
    ] {
        let first = parse::<TestField, TestState, TestSort>(input);
        assert_eq!(
            first.expr,
            Expr::Field {
                field: TestField::Genre,
                kind: MatchKind::Substring(body.into()),
            },
            "{input} must stay a literal substring"
        );
        let rendered = format!("{}", first.expr);
        assert!(
            rendered.contains('"'),
            "{input} must render quoted, got {rendered:?}"
        );
        assert_eq!(
            parse::<TestField, TestState, TestSort>(&rendered).expr,
            first.expr,
            "the rendered form must re-parse to the same node"
        );
    }
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

/// Recursive on purpose, but only ever called on trees whose depth the
/// parser's caps bound; the deep-input tests assert that bound.
fn expr_depth(expr: &Expr<TestField, TestState>) -> usize {
    match expr {
        Expr::Not(inner) => 1 + expr_depth(inner),
        Expr::And(items) | Expr::Or(items) => 1 + items.iter().map(expr_depth).max().unwrap_or(0),
        _ => 1,
    }
}

#[test]
fn deep_paren_nesting_degrades_instead_of_overflowing() {
    let input = "(".repeat(50_000);
    let p = parse::<TestField, TestState, TestSort>(&input);
    assert!(
        p.warnings.iter().any(|w| w.contains("nested too deeply")),
        "deep nesting must degrade with a warning"
    );
    // The tree stays shallow enough for Drop/Display/visit to walk.
    assert!(expr_depth(&p.expr) <= 200, "AST depth escaped the cap");
    // The degraded tree still round-trips (the 1.4.1 bug class).
    round_trip(&input);
}

#[test]
fn degradation_logs_are_capped_with_a_suppressed_summary() {
    // "("*50000 used to emit ~49.8k copies of the same warning.
    let input = "(".repeat(50_000);
    let p = parse::<TestField, TestState, TestSort>(&input);
    assert!(
        p.warnings.len() <= 101,
        "the flat log must be bounded, got {}",
        p.warnings.len()
    );
    assert_eq!(
        p.diagnostics.len(),
        p.warnings.len(),
        "the two lists stay paired under the cap"
    );
    let summary = p.warnings.last().expect("a summary must close the log");
    assert!(
        summary.contains("more degradations suppressed"),
        "the tail must count the suppressed entries, got {summary:?}"
    );
    assert!(
        p.diagnostics.last().unwrap().message == *summary,
        "the summary lands in both lists"
    );
    // A clean parse has no summary and no entries.
    let clean = parse::<TestField, TestState, TestSort>("genre:ambient AND rating:>=4");
    assert!(clean.warnings.is_empty() && clean.diagnostics.is_empty());
}

#[test]
fn negation_runs_are_capped_with_parity_kept() {
    use vir_search::ast::Folder;

    // Collapses each Not-Not pair during the walk, leaving the chain's
    // parity: the cap drops redundant marks, never the negation itself.
    struct CollapsePairs;
    impl Folder<TestField, TestState> for CollapsePairs {
        fn fold_node(&mut self, expr: Expr<TestField, TestState>) -> Expr<TestField, TestState> {
            match expr {
                Expr::Not(inner) => match *inner {
                    Expr::Not(inner2) => *inner2,
                    other => Expr::Not(Box::new(other)),
                },
                other => other,
            }
        }
    }

    // An even run of 100_000 marks must not negate; an odd one must.
    let even =
        parse::<TestField, TestState, TestSort>(&format!("{}genre:ambient", "!".repeat(100_000)));
    assert!(
        even.warnings
            .iter()
            .any(|w| w.contains("negations exceed the cap")),
        "the capped run must warn"
    );
    assert_eq!(
        even.expr.clone().fold_nodes(&mut CollapsePairs),
        parse::<TestField, TestState, TestSort>("genre:ambient").expr,
        "an even run of negations must not negate"
    );
    let odd =
        parse::<TestField, TestState, TestSort>(&format!("{}genre:ambient", "!".repeat(99_999)));
    assert_eq!(
        odd.expr.clone().fold_nodes(&mut CollapsePairs),
        parse::<TestField, TestState, TestSort>("NOT genre:ambient").expr,
        "an odd run of negations must negate"
    );
    assert!(expr_depth(&odd.expr) <= 70, "Not chain escaped the cap");
    // Both capped trees round-trip, and so does a bare run with no operand.
    round_trip(&format!("{}genre:ambient", "!".repeat(100_000)));
    round_trip(&"!".repeat(100_000));
}

#[test]
fn deep_perspective_chains_are_refused_not_crashed() {
    let mut map = HashMap::new();
    for i in 0..300 {
        map.insert(format!("p{i}"), format!("vl:p{}", i + 1));
    }
    map.insert("p300".into(), "genre:ambient".into());
    let r = Perspectives(map);
    let p = parse_with_resolver::<TestField, TestState, TestSort, _>("vl:p0", &r);
    assert!(
        p.warnings.iter().any(|w| w.contains("nested too deeply")),
        "the over-deep expansion must be refused with a warning"
    );
}

#[test]
fn stray_close_paren_warns_and_keeps_the_rest() {
    // The stray closer used to discard everything after it, silently.
    let p = parse::<TestField, TestState, TestSort>("author:x ) title:y");
    assert_eq!(
        p.expr,
        parse::<TestField, TestState, TestSort>("author:x AND title:y").expr,
        "content after a stray ')' must survive"
    );
    assert!(p.warnings.iter().any(|w| w.contains("unmatched")));
    assert_eq!((p.diagnostics[0].start, p.diagnostics[0].end), (9, 10));

    // A leading stray reads as nothing, but still warns, and what follows
    // parses.
    let p = parse::<TestField, TestState, TestSort>(") genre:ambient");
    assert_eq!(
        p.expr,
        parse::<TestField, TestState, TestSort>("genre:ambient").expr
    );
    assert!(p.warnings.iter().any(|w| w.contains("unmatched")));

    // Balanced groups still claim their closers: no new warnings there.
    let clean =
        parse::<TestField, TestState, TestSort>("(genre:ambient OR genre:jazz) AND rating:>=4");
    assert!(clean.warnings.is_empty());

    // The degraded tree round-trips.
    round_trip("author:x ) title:y");
    round_trip("genre:ambient )) genre:jazz");
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

#[test]
fn grammar_expansion_round_trips() {
    for input in [
        "added:in3days",
        "added:lastmonth",
        "added:nextmonth",
        "added:+7d",
        "added:-14d",
        "genre:ambient*",
        "genre:*metal",
        "genre:(rock,jazz)",
        "duration:>=1h30m",
        "duration:>90m",
    ] {
        round_trip(input);
    }
}

#[test]
fn relative_date_semantics() {
    let p = parse::<TestField, TestState, TestSort>("added:in3days");
    assert_eq!(
        p.expr,
        Expr::Compare {
            field: TestField::Added,
            comp: Comparator::Eq,
            value: Value::Date(DateSpec::InDays(3)),
        }
    );
    let p = parse::<TestField, TestState, TestSort>("added:+7d");
    assert_eq!(
        p.expr,
        Expr::Compare {
            field: TestField::Added,
            comp: Comparator::Eq,
            value: Value::Date(DateSpec::InDays(7)),
        }
    );
    let p = parse::<TestField, TestState, TestSort>("added:-14d");
    assert_eq!(
        p.expr,
        Expr::Compare {
            field: TestField::Added,
            comp: Comparator::Eq,
            value: Value::Date(DateSpec::DaysAgo(14)),
        }
    );
}

#[test]
fn huge_date_offset_parses_and_resolves_without_panicking() {
    use vir_search::dates::{resolve_range, today_utc};

    // The reported repro: this parsed cleanly and aborted at resolve, before
    // resolution learned to saturate.
    let p = parse::<TestField, TestState, TestSort>("added:4294967295daysago");
    assert_eq!(
        p.expr,
        Expr::Compare {
            field: TestField::Added,
            comp: Comparator::Eq,
            value: Value::Date(DateSpec::DaysAgo(u32::MAX)),
        }
    );
    let (start, end) = resolve_range(&DateSpec::DaysAgo(u32::MAX), today_utc());
    assert!(start <= end, "the saturated range must stay ordered");
}

#[test]
fn ymd_years_outside_0_to_9999_degrade_with_a_warning() {
    // added:262143 used to parse into a DateSpec resolve_range could not
    // order: the 1970 fallback inverted the range silently
    // (`added:<262143` matched everything). The parser now rejects the
    // year at parse time (decided 2026-09-15: valid range 0..=9999), so
    // the query degrades to visible text with a warning instead.
    for input in ["added:262143", "added:99999-06-08", "added:-50"] {
        let p = parse::<TestField, TestState, TestSort>(input);
        assert_eq!(
            p.expr,
            Expr::Text((*input).into()),
            "{input} must degrade to visible text"
        );
        assert!(
            p.warnings
                .iter()
                .any(|w| w.contains("bad numeric/date value")),
            "{input} must warn"
        );
        round_trip(input);
    }
    // The whole valid edge stays parseable and orderable, including the
    // year+1 arithmetic the resolution arms perform.
    use vir_search::dates::{resolve_range, today_utc};
    let p = parse::<TestField, TestState, TestSort>("added:9999-12-31");
    assert_eq!(
        p.expr,
        Expr::Compare {
            field: TestField::Added,
            comp: Comparator::Eq,
            value: Value::Date(DateSpec::Ymd(9999, Some(12), Some(31))),
        }
    );
    let (start, end) = resolve_range(&DateSpec::Ymd(9999, Some(12), Some(31)), today_utc());
    assert!(start < end, "the top of the valid range must stay ordered");
    round_trip("added:2024");
    round_trip("added:2024-06");
    round_trip("added:2024-06-08");
}

#[test]
fn wildcard_matchers() {
    let p = parse::<TestField, TestState, TestSort>("genre:ambient*");
    assert_eq!(
        p.expr,
        Expr::Field {
            field: TestField::Genre,
            kind: MatchKind::Prefix("ambient".into()),
        }
    );
    let p = parse::<TestField, TestState, TestSort>("genre:*metal");
    assert_eq!(
        p.expr,
        Expr::Field {
            field: TestField::Genre,
            kind: MatchKind::Suffix("metal".into()),
        }
    );
    // A quoted star is still literal.
    let p = parse::<TestField, TestState, TestSort>("genre:\"*\"");
    assert_eq!(
        p.expr,
        Expr::Field {
            field: TestField::Genre,
            kind: MatchKind::Substring("*".into()),
        }
    );
}

#[test]
fn in_matcher_takes_a_comma_list() {
    let p = parse::<TestField, TestState, TestSort>("genre:(rock,jazz)");
    assert_eq!(
        p.expr,
        Expr::Field {
            field: TestField::Genre,
            kind: MatchKind::In(vec!["rock".into(), "jazz".into()]),
        }
    );
    // A malformed list degrades to visible text without collapsing.
    let p = parse::<TestField, TestState, TestSort>("genre:(rock");
    assert_eq!(
        p.expr,
        parse::<TestField, TestState, TestSort>("genre:(rock").expr
    );
    assert!(!p.warnings.is_empty());
}

#[test]
fn quoted_list_body_stays_literal_text() {
    // Decided 2026-09-15: a quoted body after `(` is literal text, never
    // list syntax (the quoted-is-literal rule); the whole visible fragment
    // degrades with a warning. `genre:("rock,jazz")` used to parse as
    // In(["rock","jazz"]), contradicting the documented rule, and a
    // multi-word body broke Display round-trip outright.
    for input in ["genre:(\"rock,jazz\")", "genre:(\"hip hop,ambient\")"] {
        let p = parse::<TestField, TestState, TestSort>(input);
        let body = input
            .trim_start_matches("genre:(\"")
            .trim_end_matches("\")")
            .to_string();
        assert_eq!(
            p.expr,
            Expr::Text(format!("genre:({body})")),
            "{input} must degrade to its visible fragment"
        );
        assert!(
            p.warnings.iter().any(|w| w.contains("quoted list")),
            "{input} must warn"
        );
        // The degraded text node round-trips: it renders fully quoted, and a
        // quoted term is literal, so the re-parse lands on the same node
        // instead of the malformed re-parse the old Display produced.
        round_trip(input);
    }
    // The unquoted list is untouched.
    let p = parse::<TestField, TestState, TestSort>("genre:(rock,jazz)");
    assert_eq!(
        p.expr,
        Expr::Field {
            field: TestField::Genre,
            kind: MatchKind::In(vec!["rock".into(), "jazz".into()]),
        }
    );
}

#[test]
fn duration_values_parse_for_real_fields() {
    let p = parse::<TestField, TestState, TestSort>("duration:>=1h30m");
    assert_eq!(
        p.expr,
        Expr::Compare {
            field: TestField::Duration,
            comp: Comparator::Ge,
            value: Value::Real(5400.0),
        }
    );
    let p = parse::<TestField, TestState, TestSort>("duration:>90m");
    assert_eq!(
        p.expr,
        Expr::Compare {
            field: TestField::Duration,
            comp: Comparator::Gt,
            value: Value::Real(5400.0),
        }
    );
}

#[test]
fn visitor_walks_and_can_skip_subtrees() {
    use vir_search::ast::Visitor;

    struct Count {
        fields: usize,
        states: usize,
        prefix: Option<String>,
    }
    impl Visitor<TestField, TestState> for Count {
        fn enter(&mut self, expr: &Expr<TestField, TestState>) -> bool {
            match expr {
                Expr::Field { kind, .. } => {
                    self.fields += 1;
                    if let MatchKind::Prefix(base) = kind {
                        self.prefix = Some(base.clone());
                    }
                }
                Expr::State(_) => self.states += 1,
                _ => {}
            }
            true
        }
    }

    let p = parse::<TestField, TestState, TestSort>(
        "genre:ambient* AND rating:>=4 AND NOT is:finished",
    );
    let mut c = Count {
        fields: 0,
        states: 0,
        prefix: None,
    };
    p.expr.visit(&mut c);
    assert_eq!(c.fields, 1); // genre Prefix (rating:>=4 is a Compare node)
    assert_eq!(c.states, 1);
    assert_eq!(c.prefix.as_deref(), Some("ambient"));

    // enter=false skips the subtree: walking under a NOT is optional.
    struct SkipNot {
        states: usize,
    }
    impl Visitor<TestField, TestState> for SkipNot {
        fn enter(&mut self, expr: &Expr<TestField, TestState>) -> bool {
            if matches!(expr, Expr::Not(_)) {
                return false;
            }
            if matches!(expr, Expr::State(_)) {
                self.states += 1;
            }
            true
        }
    }
    let mut s = SkipNot { states: 0 };
    p.expr.visit(&mut s);
    assert_eq!(s.states, 0); // the NOT's is:finished was skipped
}

#[test]
fn folder_transforms_bottom_up() {
    use vir_search::ast::Folder;

    struct UnNot;
    impl Folder<TestField, TestState> for UnNot {
        fn fold_node(&mut self, expr: Expr<TestField, TestState>) -> Expr<TestField, TestState> {
            match expr {
                // NOT NOT x folds to x during the walk.
                Expr::Not(inner) => match *inner {
                    Expr::Not(inner2) => *inner2,
                    other => Expr::Not(Box::new(other)),
                },
                other => other,
            }
        }
    }

    let p = parse::<TestField, TestState, TestSort>("NOT NOT is:starred");
    let folded = p.expr.fold_nodes(&mut UnNot);
    assert_eq!(folded, Expr::State(TestState::Starred));
}
