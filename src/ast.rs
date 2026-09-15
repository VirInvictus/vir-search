//! The typed AST and its round-trippable `Display`.

use std::fmt;

use crate::lex::is_boundary;

/// The value grammar a field's values parse under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// Free text: substring, exact, regex, fuzzy, wildcard, and list matches
    /// all apply; a relational comparator degrades to visible text.
    String,
    /// Signed 64-bit integer values (`rating:>=4`).
    Int,
    /// Floating-point values, also fed by duration and magnitude parsing
    /// (`duration:>90m`, `size:<50mb`).
    Real,
    /// Symbolic [`DateSpec`] values resolved later against a day
    /// (`added:thisweek`).
    Date,
}

/// The consumer's metadata-field enum: name resolution plus the type that
/// steers value parsing.
pub trait ParseField: Clone + std::fmt::Display + PartialEq {
    /// Parse a field name as written in a query (`author`), or `None` when
    /// the name is unknown (the node then degrades to text with a warning).
    fn parse(name: &str) -> Option<Self>;
    /// Which value grammar applies to this field's values.
    fn field_type(&self) -> FieldType;
}

/// The consumer's `is:*` boolean-state enum.
pub trait ParseState: Clone + std::fmt::Display + PartialEq {
    /// Parse a state name as written (`is:finished`), or `None` when unknown
    /// (the node degrades to text with a warning).
    fn parse(name: &str) -> Option<Self>;
}

/// The consumer's `sort:*` key enum.
pub trait ParseSort: Clone + std::fmt::Display + PartialEq {
    /// Parse a sort key as written (`sort:-added`), or `None` when unknown
    /// (the directive degrades to text with a warning).
    fn parse(name: &str) -> Option<Self>;
}

/// How a field constraint matches its value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MatchKind {
    /// Plain containment (`author:sanderson`).
    Substring(String),
    /// Whole-value equality (`author:=Brandon`).
    Exact(String),
    /// Regular-expression match (`title:~^Live`); applied consumer-side.
    Regex(String),
    /// Accent-folded fuzzy match (`title:?stromlite`); applied consumer-side
    /// via the [`fuzzy`](crate::fuzzy) module.
    Fuzzy(String),
    /// `foo*`: the value starts with the base.
    Prefix(String),
    /// `*bar`: the value ends with the base.
    Suffix(String),
    /// `(a,b)`: the value equals any of the list.
    In(Vec<String>),
    /// The field is present at all (`genre:true`).
    HasAny,
    /// The field is absent (`genre:false`).
    HasNone,
}

/// A relational comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Comparator {
    /// `=`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

impl Comparator {
    /// The operator as written in a query, so `Display` renders exactly what
    /// re-parses to the same tree.
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

/// A symbolic date expression. Resolution to a concrete `[start, end)`
/// epoch-seconds range happens when the consumer calls
/// [`resolve_range`](crate::dates::resolve_range), so a parsed tree never
/// bakes in a timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DateSpec {
    /// `today`: the current UTC day.
    Today,
    /// `yesterday`.
    Yesterday,
    /// `tomorrow`.
    Tomorrow,
    /// `thisweek`: Monday through Sunday of the current week.
    ThisWeek,
    /// `lastweek`.
    LastWeek,
    /// `nextweek`.
    NextWeek,
    /// `thismonth`.
    ThisMonth,
    /// `lastmonth`.
    LastMonth,
    /// `nextmonth`.
    NextMonth,
    /// `thisyear`.
    ThisYear,
    /// `3daysago` / `-3d`: the single day `n` days back.
    DaysAgo(u32),
    /// `in3days` / `+3d`: the single day `n` days forward.
    InDays(u32),
    /// `2024`, `2024-06`, or `2024-06-08`: a year, month, or single day
    /// (`None` = "any").
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
            Self::LastMonth => write!(f, "lastmonth"),
            Self::NextMonth => write!(f, "nextmonth"),
            Self::ThisYear => write!(f, "thisyear"),
            Self::DaysAgo(n) => write!(f, "{n}daysago"),
            Self::InDays(n) => write!(f, "in{n}days"),
            Self::Ymd(y, None, _) => write!(f, "{y:04}"),
            Self::Ymd(y, Some(m), None) => write!(f, "{y:04}-{m:02}"),
            Self::Ymd(y, Some(m), Some(d)) => write!(f, "{y:04}-{m:02}-{d:02}"),
        }
    }
}

/// A parsed comparison value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// An integer literal (`rating:4`).
    Int(i64),
    /// A real-number literal (`rating:>=4.5`, `duration:>90m`). Deliberately
    /// excluded from hashing (decided 2026-09-11): its payload never enters a
    /// [`Hash`] impl, so queries carrying a real literal must be recognized
    /// with [`Expr::contains_real`] and skipped by caches instead of hashed.
    /// There is no f64 bit-pattern story to get wrong.
    Real(f64),
    /// A symbolic date (`added:thisweek`).
    Date(DateSpec),
    /// A literal string, as in a degraded fragment or a text range bound.
    Text(String),
}

impl Eq for Value {}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Int(n) => n.hash(state),
            // The payload stays out (see the variant's docs): only the
            // discriminant reaches the hasher. Equal values still hash equal,
            // so the Hash/Eq contract holds; distinct reals simply collide.
            Value::Real(_) => {}
            Value::Date(d) => d.hash(state),
            Value::Text(s) => s.hash(state),
        }
    }
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

/// A sort directive extracted during parsing (`sort:-added`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SortSpec<K> {
    /// The consumer's sort key.
    pub key: K,
    /// `true` when written with a leading `-`.
    pub descending: bool,
}

/// The typed search-expression tree the parser produces.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Expr<F, S> {
    /// Identity node: empty input, or a degraded fragment folded away.
    Empty,
    /// Bare free text, matched via FTS or substring.
    Text(String),
    /// A direct metadata-field constraint (`author:sanderson`).
    Field {
        /// The consumer's field.
        field: F,
        /// How the value matches.
        kind: MatchKind,
    },
    /// A relational constraint (`rating:>=4`).
    Compare {
        /// The consumer's field.
        field: F,
        /// The operator.
        comp: Comparator,
        /// The parsed value.
        value: Value,
    },
    /// A bounded range (`year:2020..2023`).
    Range {
        /// The consumer's field.
        field: F,
        /// Inclusive lower bound.
        low: Value,
        /// Exclusive upper bound.
        high: Value,
    },
    /// A boolean binary state (`is:read`).
    State(S),
    /// Logical negation (`NOT x`, `!x`).
    Not(Box<Expr<F, S>>),
    /// Conjunction of the operands.
    And(Vec<Expr<F, S>>),
    /// Disjunction of the operands.
    Or(Vec<Expr<F, S>>),
}

/// Marker companions to the derived [`PartialEq`]. The derived [`Hash`] adds
/// `Hash` bounds on the consumer's `F`/`S` (plain derived enums have them),
/// while this `Eq` adds none; [`Value::Real`] is the one payload excluded
/// from hashing, so a cache keyed on `Expr` must skip queries where
/// [`Expr::contains_real`] is true rather than hash them. (`Eq` is bit
/// equality via `PartialEq`; a `nan` literal, reachable as `duration:nan`,
/// is never equal to itself, which is one more reason real-carrying queries
/// do not belong in a hash-keyed cache.)
impl<F: ParseField, S: ParseState> Eq for Expr<F, S> {}

/// Read-only AST inspection. `enter` is called for every node, parents before
/// children; returning `false` skips that node's children. Implement only the
/// hooks you need — the node itself carries everything, so `enter` alone is
/// enough for counting, collection, and SQL-eligibility checks.
pub trait Visitor<F, S> {
    /// Called for every node, parents before children. Return `false` to
    /// skip that node's children.
    fn enter(&mut self, _expr: &Expr<F, S>) -> bool {
        true
    }
}

/// Bottom-up transformation: `fold_node` receives each node with its children
/// already folded, and its return replaces the node. A no-op `fold_node`
/// rebuilds the tree unchanged.
pub trait Folder<F, S> {
    /// Replace (or keep) a node whose children are already folded.
    fn fold_node(&mut self, expr: Expr<F, S>) -> Expr<F, S> {
        expr
    }
}

impl<F: ParseField, S: ParseState> Expr<F, S> {
    /// Does the tree carry any [`Value::Real`] literal? Caches keyed on
    /// `Expr` call this before hashing: real payloads are excluded from
    /// hashing by design, so a real-carrying query must skip the cache
    /// instead of entering it (the crate's own
    /// [`QueryCache`](crate::cache::QueryCache) does exactly that).
    pub fn contains_real(&self) -> bool {
        match self {
            Expr::Compare { value, .. } => matches!(value, Value::Real(_)),
            Expr::Range { low, high, .. } => {
                matches!(low, Value::Real(_)) || matches!(high, Value::Real(_))
            }
            Expr::Not(inner) => inner.contains_real(),
            Expr::And(items) | Expr::Or(items) => items.iter().any(Expr::contains_real),
            _ => false,
        }
    }

    /// Depth-first read-only walk (parents before children).
    pub fn visit<V: Visitor<F, S>>(&self, v: &mut V) {
        if !v.enter(self) {
            return;
        }
        match self {
            Expr::Not(inner) => inner.visit(v),
            Expr::And(items) | Expr::Or(items) => items.iter().for_each(|e| e.visit(v)),
            _ => {}
        }
    }

    /// Depth-first transformation (children before the parent).
    pub fn fold_nodes<V: Folder<F, S>>(self, f: &mut V) -> Expr<F, S> {
        let expr = match self {
            Expr::Not(inner) => Expr::Not(Box::new(inner.fold_nodes(f))),
            Expr::And(items) => Expr::And(items.into_iter().map(|e| e.fold_nodes(f)).collect()),
            Expr::Or(items) => Expr::Or(items.into_iter().map(|e| e.fold_nodes(f)).collect()),
            leaf => leaf,
        };
        f.fold_node(expr)
    }
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
        // The wildcard forms are single-word by construction (the parser only
        // honors them on unquoted barewords), so they re-lex to themselves.
        MatchKind::Prefix(v) => write!(f, "{name}:{v}*"),
        MatchKind::Suffix(v) => write!(f, "{name}:*{v}"),
        MatchKind::In(items) => write!(f, "{name}:({})", items.join(",")),
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
        // A nested combinator parenthesizes whenever the flat form would
        // re-associate into a different shape: an `And` or `Or` under an
        // `And`, and an `Or` under an `Or`. A flat `And` under an `Or` is
        // exact (`a b OR c` re-parses to the same tree; AND binds tighter).
        let needs = match item {
            Expr::Or(_) => true,
            Expr::And(_) => !is_or,
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

/// Quote a rendered term when re-lexing it bare would mis-split (whitespace,
/// operators, reserved words) or would be intercepted by the field-value
/// grammar (presence checks, wildcards), escaping backslashes and quotes so
/// `Display` round-trips. The interception cases here mirror the parser's
/// unquoted-value arms exactly: a quoted value that this function quotes
/// always re-parses to the same node it rendered from.
pub fn quote_if_needed(s: &str) -> String {
    let needs = s.is_empty()
        || s.chars().any(is_boundary)
        || s.contains("..")
        || s.eq_ignore_ascii_case("and")
        || s.eq_ignore_ascii_case("or")
        || s.eq_ignore_ascii_case("not")
        || bool_word(s).is_some()
        || wildcard_intercepted(s);
    if needs {
        // Backslash first, so an escaped quote's own backslash is not doubled.
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}

/// Would a bareword value be intercepted by the prefix/suffix wildcard arms
/// (`foo*`, `*bar`)? The same shape the parser honors: one star on one end,
/// a non-empty base without another star.
fn wildcard_intercepted(s: &str) -> bool {
    let one_end_star = |base: &str| !base.is_empty() && !base.contains('*');
    s.strip_suffix('*').is_some_and(one_end_star) || s.strip_prefix('*').is_some_and(one_end_star)
}

/// Is `w` a presence-check word (`true` / `false`, any case)? Shared by the
/// parser's unquoted-value arms and by [`quote_if_needed`], which must quote
/// exactly the words the parser would otherwise intercept.
pub(crate) fn bool_word(w: &str) -> Option<bool> {
    match w.to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}
