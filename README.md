# vir-search

A domain-agnostic Rust library for parsing Calibre-style search expressions into a typed Abstract Syntax Tree (AST).

Extracted from `atrium-search` and `conservatory-search`, `vir-search` provides the lexer, generic recursive-descent parser, ranking heuristics, and date-range resolvers that underpin the VirInvictus ecosystem. By parameterizing the AST over the consumer's `Field`, `State`, and `SortKey` types, it avoids domain-coupling while maintaining a unified, powerful search grammar across the entire suite of desktop applications.

## Architecture and Scope

The library is explicitly designed to handle the parsing stage of a query pipeline. It does *not* provide SQL translation or in-memory evaluation logic; consumers must translate the resulting `Expr<F, S>` into their own domain logic (e.g., SQLite FTS queries for Conservatory, or `SearchItem` evaluations for Atrium).

### The Parser (`vir_search::parse`)

The core parser uses recursive descent to produce an AST. It is designed around a **"never fail"** philosophy: syntax errors, unrecognized fields, or malformed constraints do not panic or return hard errors. Instead, they gracefully degrade into raw text-matching nodes. This ensures that users can always fall back to standard full-text search behavior even if they type something the parser doesn't natively understand.

### The Grammar

The library understands a deep, Calibre-compatible grammar:
- **Field Matches**: `author:sanderson` (substring), `author:=Brandon` (exact), `title:~regex` (regex), `title:?fuzzy` (fuzzy).
- **Relational Constraints**: `rating:>=4`, `duration:>600`.
- **Date Arithmetic**: `added:thisweek`, `year:2020..2023`, `added:3daysago`. Date arithmetic is dynamically resolved against the current epoch during parsing using `chrono`.
- **States**: `is:finished`, `is:starred`.
- **Logic**: Parentheses, `AND`, `OR`, `NOT`.

## Usage

To use `vir-search`, you must implement `ParseField`, `ParseState`, and `ParseSort` on your domain enums. Crucially, your `ParseField` implementation must return a `FieldType` so the parser knows whether to attempt numeric/date parsing or fall back to strings.

```rust
use vir_search::parse::parse;
use vir_search::ast::{FieldType, ParseField, ParseState, ParseSort};

#[derive(Clone, PartialEq, Eq)]
enum MyField { Author, Rating }

impl std::fmt::Display for MyField { /* ... */ }

impl ParseField for MyField {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "author" => Some(Self::Author),
            "rating" => Some(Self::Rating),
            _ => None,
        }
    }
    
    fn field_type(&self) -> FieldType {
        match self {
            Self::Rating => FieldType::Int,
            Self::Author => FieldType::String,
        }
    }
}

// ... implement MyState and MySort ...

// Parse the user's input into an AST
let result = parse::<MyField, MyState, MySort>("author:sanderson rating:>=4");
println!("Parsed expression: {}", result.expr);
```
