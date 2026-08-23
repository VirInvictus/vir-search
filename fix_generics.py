import re

# AST.rs
with open("src/ast.rs", "r") as f:
    text = f.read()

# Replace Field enum/impl
text = re.sub(r'pub enum Field \{.*?impl Field \{.*?\}', 
"""pub enum FieldType {
    String, Int, Real, Date,
}

pub trait ParseField: Clone + std::fmt::Display + PartialEq {
    fn parse(name: &str) -> Option<Self>;
    fn field_type(&self) -> FieldType;
}
""", text, flags=re.DOTALL)

# Replace State enum/impl
text = re.sub(r'pub enum State \{.*?impl State \{.*?\}', 
"""pub trait ParseState: Clone + std::fmt::Display + PartialEq {
    fn parse(name: &str) -> Option<Self>;
}""", text, flags=re.DOTALL)

# Replace SortKey enum/impl
text = re.sub(r'pub enum SortKey \{.*?impl SortKey \{.*?\}', 
"""pub trait ParseSort: Clone + std::fmt::Display + PartialEq {
    fn parse(name: &str) -> Option<Self>;
}""", text, flags=re.DOTALL)

# Update SortSpec
text = re.sub(r'pub struct SortSpec \{.*?\}', 
"""pub struct SortSpec<K> {
    pub key: K,
    pub descending: bool,
}""", text, flags=re.DOTALL)

# Update Expr
text = re.sub(r'pub enum Expr \{.*?\}', 
"""pub enum Expr<F, S> {
    Empty,
    Text(String),
    Field { field: F, kind: MatchKind },
    Compare { field: F, comp: Comparator, value: Value },
    Range { field: F, low: Value, high: Value },
    State(S),
    Not(Box<Expr<F, S>>),
    And(Vec<Expr<F, S>>),
    Or(Vec<Expr<F, S>>),
}""", text, flags=re.DOTALL)

# Update Expr impl Display
text = re.sub(r'impl fmt::Display for Expr \{.*?fn write_field\(f: &mut fmt::Formatter<\'_\>, field: Field, kind: &MatchKind\) -> fmt::Result \{.*?\}',
"""impl<F: ParseField, S: ParseState> fmt::Display for Expr<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => Ok(()),
            Self::Text(s) => write!(f, "{}", quote_if_needed(s)),
            Self::Field { field, kind } => write_field(f, field, kind),
            Self::Compare { field, comp, value } => {
                write!(f, "{field}:{}{value}", comp.as_str())
            }
            Self::Range { field, low, high } => {
                write!(f, "{field}:{low}..{high}")
            }
            Self::State(state) => write!(f, "is:{state}"),
            Self::Not(inner) => write!(f, "NOT {}", paren(inner)),
            Self::And(items) => write_joined(f, items, "AND", false),
            Self::Or(items) => write_joined(f, items, "OR", true),
        }
    }
}

fn write_field<F: ParseField>(f: &mut fmt::Formatter<'_>, field: &F, kind: &MatchKind) -> fmt::Result {
    let name = field;
    match kind {
        MatchKind::Substring(v) => write!(f, "{name}:{}", quote_if_needed(v)),
        MatchKind::Exact(v) => write!(f, "{name}:={}", quote_if_needed(v)),
        MatchKind::Regex(v) => write!(f, "{name}:~{}", quote_if_needed(v)),
        MatchKind::Fuzzy(v) => write!(f, "{name}:?{}", quote_if_needed(v)),
        MatchKind::HasAny => write!(f, "{name}:true"),
        MatchKind::HasNone => write!(f, "{name}:false"),
    }
}""", text, flags=re.DOTALL)

text = text.replace("fn write_joined(f: &mut fmt::Formatter<'_>, items: &[Expr], op: &str, is_or: bool) -> fmt::Result {",
                    "fn write_joined<F: ParseField, S: ParseState>(f: &mut fmt::Formatter<'_>, items: &[Expr<F, S>], op: &str, is_or: bool) -> fmt::Result {")

text = text.replace("fn paren(expr: &Expr) -> String {", "fn paren<F: ParseField, S: ParseState>(expr: &Expr<F, S>) -> String {")

with open("src/ast.rs", "w") as f:
    f.write(text)

# PARSE.RS
with open("src/parse.rs", "r") as f:
    text = f.read()

text = text.replace("use crate::ast::{Comparator, DateSpec, Expr, Field, MatchKind, SortKey, SortSpec, State, Value};",
                    "use crate::ast::{Comparator, DateSpec, Expr, ParseField, ParseState, ParseSort, MatchKind, SortSpec, Value};")

text = text.replace("pub struct ParseResult {", "pub struct ParseResult<F, S, K> {")
text = text.replace("pub expr: Expr,", "pub expr: Expr<F, S>,")
text = text.replace("pub sorts: Vec<SortSpec>,", "pub sorts: Vec<SortSpec<K>>,")

text = text.replace("type PResult = Result<Expr, ()>;", "type PResult<F, S> = Result<Expr<F, S>, ()>;")

text = text.replace("pub trait PerspectiveResolver {", "pub trait PerspectiveResolver<F, S> {")

text = text.replace("pub fn parse(input: &str) -> ParseResult {", "pub fn parse<F: ParseField, S: ParseState, K: ParseSort>(input: &str) -> ParseResult<F, S, K> {")
text = text.replace("parse_with_resolver(input, &())", "parse_with_resolver(input, &())")

text = text.replace("impl PerspectiveResolver for () {", "impl<F, S> PerspectiveResolver<F, S> for () {")

text = text.replace("pub fn parse_with_resolver<R: PerspectiveResolver>(", "pub fn parse_with_resolver<F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>>(")
text = text.replace("input: &str, resolver: &R) -> ParseResult {", "input: &str, resolver: &R) -> ParseResult<F, S, K> {")
text = text.replace("parse_inner(input, Some(resolver), &[])", "parse_inner(input, Some(resolver), &[])")

text = text.replace("fn parse_inner<R: PerspectiveResolver>(", "fn parse_inner<F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>>(")
text = text.replace("resolver: Option<&R>,", "resolver: Option<&R>,")
text = text.replace(") -> ParseResult {", ") -> ParseResult<F, S, K> {")

text = text.replace("struct Parser<'a, R> {", "struct Parser<'a, F, S, K, R> {")
text = text.replace("resolver: Option<&'a R>,", "resolver: Option<&'a R>,\n    _marker: std::marker::PhantomData<(F, S, K)>,")

text = text.replace("let mut p = Parser {", "let mut p = Parser { _marker: std::marker::PhantomData,")

text = text.replace("impl<'a, R: PerspectiveResolver> Parser<'a, R> {", "impl<'a, F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>> Parser<'a, F, S, K, R> {")

# Inner Parser impl
text = text.replace("-> PResult {", "-> PResult<F, S> {")
text = text.replace("fn boolean_expr(&mut self) -> PResult", "fn boolean_expr(&mut self) -> PResult<F, S>")
text = text.replace("fn boolean_term(&mut self) -> PResult", "fn boolean_term(&mut self) -> PResult<F, S>")
text = text.replace("fn boolean_factor(&mut self) -> PResult", "fn boolean_factor(&mut self) -> PResult<F, S>")
text = text.replace("fn predicate(&mut self) -> PResult", "fn predicate(&mut self) -> PResult<F, S>")
text = text.replace("fn relational(&mut self, field: Field) -> PResult", "fn relational(&mut self, field: F) -> PResult<F, S>")
text = text.replace("fn text_match(&mut self, field: Field) -> PResult", "fn text_match(&mut self, field: F) -> PResult<F, S>")
text = text.replace("fn sort_spec(&mut self, key_str: String, descending: bool) -> PResult", "fn sort_spec(&mut self, key_str: String, descending: bool) -> PResult<F, S>")
text = text.replace("fn resolve_vl(&mut self, vl_name: &str) -> PResult", "fn resolve_vl(&mut self, vl_name: &str) -> PResult<F, S>")

text = text.replace("sorts: Vec<SortSpec>,", "sorts: Vec<SortSpec<K>>,")

text = text.replace("Field::parse(", "F::parse(")
text = text.replace("State::parse(", "S::parse(")
text = text.replace("SortKey::parse(", "K::parse(")

text = text.replace("field.is_numeric() || field.is_date()", "field.field_type() == crate::ast::FieldType::Int || field.field_type() == crate::ast::FieldType::Real || field.field_type() == crate::ast::FieldType::Date")

text = text.replace("fn parse_typed_value(field: Field,", "fn parse_typed_value<F: ParseField>(field: F,")
text = text.replace("field.is_date()", "field.field_type() == crate::ast::FieldType::Date")
text = text.replace("matches!(field, Field::Duration)", "field.field_type() == crate::ast::FieldType::Real")

text = text.replace("Expr::Empty", "Expr::<F, S>::Empty")

text = text.replace("fn text_or_empty(s: String) -> Expr {", "fn text_or_empty<F, S>(s: String) -> Expr<F, S> {")
text = text.replace("fn paren(expr: &Expr) -> String {", "fn paren<F: ParseField, S: ParseState>(expr: &Expr<F, S>) -> String {")

text = text.replace("let mut items: Vec<Expr> =", "let mut items: Vec<Expr<F, S>> =")
text = text.replace("let items: Vec<Expr> =", "let items: Vec<Expr<F, S>> =")

text = text.replace("fn expression(&self, name: &str) -> Option<String>", "fn expression(&self, name: &str) -> Option<String>")

with open("src/parse.rs", "w") as f:
    f.write(text)

