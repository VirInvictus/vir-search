//! # vir-search
//!
//! A domain-agnostic parser for Calibre-style search expressions
//! (`author:sanderson rating:>=4 added:thisweek`), yielding a typed
//! [`Expr`](ast::Expr) that the consumer translates into its own backend
//! (SQLite FTS, in-memory evaluation, anything). The crate deliberately
//! ships no SQL generation and no evaluation: parsing and AST only.
//!
//! Parsing never fails: malformed input degrades locally into text-match
//! nodes with plain warnings and byte-spanned
//! [`Diagnostic`]s, so a search bar can underline exactly what could not be
//! understood. Recursion is bounded (deep nesting and negation runs degrade
//! instead of overflowing), and dates stay symbolic ([`DateSpec`]) until the
//! consumer resolves them against a day with
//! [`resolve_range`](dates::resolve_range).
//!
//! Start at [`parse`]; implement the [`ParseField`](ast::ParseField),
//! [`ParseState`](ast::ParseState), and [`ParseSort`](ast::ParseSort) traits
//! on your domain enums first. The README carries the normative grammar.
#![warn(missing_docs)]

pub mod ast;
pub mod cache;
pub mod dates;
pub mod fold;
pub mod fuzzy;
pub mod lex;
pub mod parse;
pub mod rank;

pub use ast::{
    Comparator, DateSpec, Expr, FieldType, Folder, MatchKind, ParseField, ParseSort, ParseState,
    SortSpec, Value, Visitor,
};
pub use cache::QueryCache;
pub use fold::fold;
pub use parse::{Diagnostic, ParseResult, PerspectiveResolver, parse, parse_with_resolver};
pub use rank::{blend_relevance, collect_text_terms};
