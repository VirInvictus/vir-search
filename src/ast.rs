//! The typed AST and its round-trippable `Display`.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    Int,
    Real,
    Date,
}

pub trait ParseField: Clone + std::fmt::Display + PartialEq {
    fn parse(name: &str) -> Option<Self>;
    fn field_type(&self) -> FieldType;
}

pub trait ParseState: Clone + std::fmt::Display + PartialEq {
    fn parse(name: &str) -> Option<Self>;
}

pub trait ParseSort: Clone + std::fmt::Display + PartialEq {
    fn parse(name: &str) -> Option<Self>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchKind {
    Substring(String),
    Exact(String),
    Regex(String),
    Fuzzy(String),
    HasAny,
    HasNone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparator {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Comparator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateSpec {
    Today,
    Yesterday,
    Tomorrow,
    ThisWeek,
    LastWeek,
    NextWeek,
    ThisMonth,
    ThisYear,
    DaysAgo(u32),
    Ymd(i32, Option<u32>, Option<u32>),
}

impl fmt::Display for DateSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Today => write!(f, "today"),
            Self::Yesterday => write!(f, "yesterday"),
            Self::Tomorrow => write!(f, "tomorrow"),
            Self::ThisWeek => write!(f, "thisweek"),
            Self::LastWeek => write!(f, "lastweek"),
            Self::NextWeek => write!(f, "nextweek"),
            Self::ThisMonth => write!(f, "thismonth"),
            Self::ThisYear => write!(f, "thisyear"),
            Self::DaysAgo(n) => write!(f, "{n}daysago"),
            Self::Ymd(y, None, _) => write!(f, "{y:04}"),
            Self::Ymd(y, Some(m), None) => write!(f, "{y:04}-{m:02}"),
            Self::Ymd(y, Some(m), Some(d)) => write!(f, "{y:04}-{m:02}-{d:02}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Real(f64),
    Date(DateSpec),
    Text(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(n) => write!(f, "{n}"),
            Self::Real(x) => write!(f, "{x}"),
            Self::Date(d) => write!(f, "{d}"),
            Self::Text(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortSpec<K> {
    pub key: K,
    pub descending: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<F, S> {
    Empty,
    Text(String),
    Field {
        field: F,
        kind: MatchKind,
    },
    Compare {
        field: F,
        comp: Comparator,
        value: Value,
    },
    Range {
        field: F,
        low: Value,
        high: Value,
    },
    State(S),
    Not(Box<Expr<F, S>>),
    And(Vec<Expr<F, S>>),
    Or(Vec<Expr<F, S>>),
}

impl<F: ParseField, S: ParseState> fmt::Display for Expr<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => Ok(()),
            Self::Text(s) => write!(f, "{}", quote_if_needed(s)),
            Self::Field { field, kind } => write_field(f, field, kind),
            Self::Compare { field, comp, value } => write!(f, "{field}:{}{value}", comp.as_str()),
            Self::Range { field, low, high } => write!(f, "{field}:{low}..{high}"),
            Self::State(state) => write!(f, "is:{state}"),
            Self::Not(inner) => write!(f, "NOT {}", paren(inner)),
            Self::And(items) => write_joined(f, items, "AND", false),
            Self::Or(items) => write_joined(f, items, "OR", true),
        }
    }
}

fn write_field<F: ParseField>(
    f: &mut fmt::Formatter<'_>,
    field: &F,
    kind: &MatchKind,
) -> fmt::Result {
    let name = field;
    match kind {
        MatchKind::Substring(v) => write!(f, "{name}:{}", quote_if_needed(v)),
        MatchKind::Exact(v) => write!(f, "{name}:={}", quote_if_needed(v)),
        MatchKind::Regex(v) => write!(f, "{name}:~{}", quote_if_needed(v)),
        MatchKind::Fuzzy(v) => write!(f, "{name}:?{}", quote_if_needed(v)),
        MatchKind::HasAny => write!(f, "{name}:true"),
        MatchKind::HasNone => write!(f, "{name}:false"),
    }
}

fn write_joined<F: ParseField, S: ParseState>(
    f: &mut fmt::Formatter<'_>,
    items: &[Expr<F, S>],
    op: &str,
    is_or: bool,
) -> fmt::Result {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            write!(f, " {op} ")?;
        }
        let needs = match item {
            Expr::Or(_) if !is_or => true,
            Expr::And(_) if is_or => false,
            _ => false,
        };
        if needs {
            write!(f, "({item})")?;
        } else {
            write!(f, "{item}")?;
        }
    }
    Ok(())
}

fn paren<F: ParseField, S: ParseState>(expr: &Expr<F, S>) -> String {
    match expr {
        Expr::And(_) | Expr::Or(_) => format!("({expr})"),
        _ => format!("{expr}"),
    }
}

pub fn quote_if_needed(s: &str) -> String {
    let boundary = |c: char| {
        c.is_whitespace() || matches!(c, '(' | ')' | ':' | '"' | '~' | '?' | '!' | '<' | '>' | '=')
    };
    let needs = s.is_empty()
        || s.chars().any(boundary)
        || s.contains("..")
        || s.eq_ignore_ascii_case("and")
        || s.eq_ignore_ascii_case("or")
        || s.eq_ignore_ascii_case("not");
    if needs {
        // Backslash first, so an escaped quote's own backslash is not doubled.
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}
