# vir-search Specification

**Domain**: Expression parsing and AST generation.
**Language**: Rust (edition 2021).
**Dependencies**: `chrono`.

## 1. Scope and Architecture

`vir-search` provides a generic, domain-agnostic parser for Calibre-style search expressions, yielding an `Expr<F, S>` abstract syntax tree. It is designed specifically for integration with `vir-gtk` applications like Atrium and Conservatory, centralizing the complexity of recursive-descent parsing without tightly coupling to their local database schemas.

## 2. API Contract and Generics

The library exposes the following core entrypoint:
`pub fn parse<F: ParseField, S: ParseState, K: ParseSort>(input: &str) -> ParseResult<F, S, K>`

### 2.1 AST Nodes
- `Empty`: Identity node for empty input or degraded cycles.
- `Text(String)`: Bare free text, matched via FTS or substring.
- `Field(F, MatchKind)`: A direct metadata field constraint.
- `Compare(F, Comparator, Value)`: A relational constraint (e.g., `rating:>=4`).
- `Range(F, Value, Value)`: A bounded range (e.g., `year:2020..2023`).
- `State(S)`: A boolean binary state (e.g., `is:read`).
- Logic: `Not(Box<Expr>)`, `And(Vec<Expr>)`, `Or(Vec<Expr>)`.

### 2.2 Traits
Consumers must implement:
- `ParseField`: Provides name resolution and crucially `field_type()` to distinguish parsing paths (String, Int, Real, Date).
- `ParseState`: Resolves `is:*` boolean states.
- `ParseSort`: Resolves `sort:*` directives extracted during parsing.

## 3. Fallback and Degradation Policies

The parser enforces a strict "never fail" policy.
- If a token resembles a field syntax (`unknown:value`) but the domain's `ParseField` implementation returns `None`, the parser emits a warning in the `ParseResult` and degrades the node to `Expr::Text("unknown:value")`.
- Unbalanced parentheses, trailing logical operators, and malformed numeric values all degrade safely into text matches rather than terminating execution.
- The lexer handles quotes and unicode safely without panicking.

## 4. Relevance Ranking

The library provides `blend_relevance`, a mathematical heuristic combining a `bm25` Full-Text Search score with an exponential recency decay. It also provides `collect_text_terms`, which traverses the generic AST to harvest bare-text components. Consumers use these extracted strings to supply their underlying SQLite FTS queries while bypassing the strictly fielded constraints.
