//! The smallest complete consumer of `vir-search`, compile-checked by CI
//! (`cargo test`/`cargo clippy --all-targets` build examples) and meant as
//! the onboarding doc: implement the three traits on your domain enums,
//! then call [`vir_search::parse`] and translate the tree yourself.
//!
//! Run with `cargo run --example mini_consumer`.

use vir_search::ast::{Expr, FieldType, MatchKind, ParseField, ParseSort, ParseState};
use vir_search::parse::parse;

/// A library's metadata fields. `parse` maps the names a query can write
/// (`author:sanderson`); `field_type` steers value parsing (a `Date` field
/// engages the date grammar, an `Int` field the integer grammar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Author,
    Rating,
}

impl std::fmt::Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Field::Author => "author",
            Field::Rating => "rating",
        })
    }
}

impl ParseField for Field {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "author" => Some(Field::Author),
            "rating" => Some(Field::Rating),
            _ => None,
        }
    }

    fn field_type(&self) -> FieldType {
        match self {
            Field::Rating => FieldType::Int,
            Field::Author => FieldType::String,
        }
    }
}

/// The `is:*` boolean states (`is:read`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Read,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("read")
    }
}

impl ParseState for State {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "read" => Some(State::Read),
            _ => None,
        }
    }
}

/// The `sort:*` keys (`sort:-rating`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sort {
    Rating,
}

impl std::fmt::Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("rating")
    }
}

impl ParseSort for Sort {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "rating" => Some(Sort::Rating),
            _ => None,
        }
    }
}

fn main() {
    let result = parse::<Field, State, Sort>("sanderson rating:>=4 NOT is:read");
    println!("tree:      {}", result.expr);
    println!("sorts:     {:?}", result.sorts);
    println!("warnings:  {:?}", result.warnings);

    // Translate the tree into your own domain logic with a Visitor: this
    // crate ships no evaluation and no SQL generation, so walking the
    // variants and producing your own predicates is the consumer's job.
    let mut describe = Describe;
    result.expr.visit(&mut describe);
}

/// Read-only walk, parents before children (`enter` returning `false` skips
/// a node's subtree).
struct Describe;

impl vir_search::ast::Visitor<Field, State> for Describe {
    fn enter(&mut self, expr: &Expr<Field, State>) -> bool {
        match expr {
            Expr::Field { field, kind } => match kind {
                MatchKind::Substring(v) => println!("{field} contains {v:?}"),
                MatchKind::HasAny => println!("{field} is present"),
                _ => println!("{field} {kind:?}"),
            },
            Expr::Compare { field, comp, value } => println!("{field} {} {value}", comp.as_str()),
            Expr::Text(s) => println!("free text {s:?}"),
            Expr::State(s) => println!("state {s}"),
            Expr::Not(_) => println!("negation:"),
            _ => {}
        }
        true
    }
}
