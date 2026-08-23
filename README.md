# vir-search

A domain-agnostic Rust library for parsing Calibre-style search expressions into a typed Abstract Syntax Tree (AST).

Extracted from `atrium-search` and `conservatory-search`, `vir-search` provides the lexer, generic recursive-descent parser, ranking heuristics, and date-range resolvers that underpin the `vir-gtk` ecosystem. By parameterizing the AST over the consumer's `Field`, `State`, and `SortKey` enums, it avoids domain-coupling while maintaining a unified, powerful search grammar.

## Features

- **Generic AST (`Expr<F, S>`)**: Consumer defines the fields and states.
- **Calibre Search Grammar**: Supports `field:value`, `field:=exact`, `field:~regex`, `field:>10`, `is:state`, `NOT`, `AND`, `OR`, and parentheses.
- **Date Arithmetic**: Resolves `added:thisweek` or `year:2020..2023` to epoch ranges using `chrono`.
- **Relevance Ranking**: Provides a `blend_relevance` heuristic combining `bm25` FTS scores with recency decay.
- **Round-Trippable**: `parse(input).expr.to_string()` reproduces a stable text representation, allowing queries to be saved and re-parsed losslessly.
- **Forgiving Parser**: Unrecognized fields or syntax degrade gracefully to bare-text matches rather than erroring out, prioritizing a "never fail" user experience.

## Usage

Implement `ParseField`, `ParseState`, and `ParseSort` on your enums, then call `parse`:

```rust
use vir_search::parse::parse;
use vir_search::ast::{FieldType, ParseField, ParseState, ParseSort};

#[derive(Clone, PartialEq, Eq)]
enum MyField { Author, Rating }
// ... implement Display and ParseField ...

let query = parse::<MyField, MyState, MySort>("author:sanderson rating:>=4");
```
