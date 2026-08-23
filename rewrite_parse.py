with open("src/parse.rs", "r") as f:
    text = f.read()

text = text.replace("use crate::ast::{Comparator, DateSpec, Expr, Field, MatchKind, SortKey, SortSpec, State, Value};", 
                    "use crate::ast::{Comparator, DateSpec, Expr, ParseField, ParseState, ParseSort, MatchKind, SortSpec, Value};")

text = text.replace("pub struct ParseResult {", "pub struct ParseResult<F, S, K> {")
text = text.replace("pub expr: Expr,", "pub expr: Expr<F, S>,")
text = text.replace("pub sorts: Vec<SortSpec>,", "pub sorts: Vec<SortSpec<K>>,")

text = text.replace("type PResult = Result<Expr, ()>;", "type PResult<F, S> = Result<Expr<F, S>, ()>;")

text = text.replace("pub trait PerspectiveResolver {", "pub trait PerspectiveResolver<F, S> {")
text = text.replace("fn expression(&self, name: &str) -> Option<String>;", "fn expression(&self, name: &str) -> Option<String>;")

text = text.replace("pub fn parse(q: &str) -> ParseResult {", 
                    "pub fn parse<F: ParseField, S: ParseState, K: ParseSort>(q: &str) -> ParseResult<F, S, K> {")
text = text.replace("parse_with_resolver(q, &())", "parse_with_resolver(q, &())")

text = text.replace("impl PerspectiveResolver for () {", "impl<F, S> PerspectiveResolver<F, S> for () {")

text = text.replace("pub fn parse_with_resolver<R: PerspectiveResolver>(", 
                    "pub fn parse_with_resolver<F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>>(")
text = text.replace("q: &str, resolver: &R) -> ParseResult {", 
                    "q: &str, resolver: &R) -> ParseResult<F, S, K> {")
text = text.replace("parse_inner(q, Some(resolver), &[])", 
                    "parse_inner(q, Some(resolver), &[])")

text = text.replace("fn parse_inner<R: PerspectiveResolver>(", 
                    "fn parse_inner<F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>>(")
text = text.replace("resolver: Option<&R>,", "resolver: Option<&R>,")
text = text.replace(") -> ParseResult {", ") -> ParseResult<F, S, K> {")

text = text.replace("struct Parser<'a, R> {", "struct Parser<'a, F, S, K, R> {")
text = text.replace("resolver: Option<&'a R>,", "resolver: Option<&'a R>,\n    _marker: std::marker::PhantomData<(F, S, K)>,")

text = text.replace("let mut p = Parser {", "let mut p = Parser { _marker: std::marker::PhantomData,")

text = text.replace("impl<'a, R: PerspectiveResolver> Parser<'a, R> {", 
                    "impl<'a, F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>> Parser<'a, F, S, K, R> {")

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

text = text.replace("Expr::Empty", "Expr::Empty")

text = text.replace("fn text_or_empty(s: String) -> Expr {", "fn text_or_empty<F, S>(s: String) -> Expr<F, S> {")

# Vectors
text = text.replace("let items: Vec<Expr> =", "let items: Vec<Expr<F, S>> =")
text = text.replace("let mut items: Vec<Expr> =", "let mut items: Vec<Expr<F, S>> =")

with open("src/parse.rs", "w") as f:
    f.write(text)
