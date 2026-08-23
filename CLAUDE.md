# vir-search

**Stack:** Rust, `chrono`.
**Status:** Maintained. Standalone library.

## What is this?
A domain-agnostic Rust library for parsing Calibre-style search expressions into a typed AST. Extracted from the `vir-gtk` ecosystem (Atrium, Conservatory) to eliminate parser duplication and provide a unified query language across the desktop suite.

## Key Rules
- **No Domain Logic**: Do not add SQL generation (`sql_translate.rs`) or local object evaluation (`eval.rs`) to this crate. Consumers must translate the generic `Expr<F, S>` into their own domain logic themselves. This crate must remain exclusively a parser and AST definition.
- **`FieldType` Introspection**: Because the parser does not know what `F` is at compile time, consumers implement `ParseField::field_type()`. The parser relies on this to branch its parsing logic (e.g., treating `Duration` as `FieldType::Real` to allow float parsing, or `Added` as `FieldType::Date` to engage the `chrono` date parser).
- **Forgiving Parsing**: The lexer and parser must never fail or panic. Malformed syntax, unbalanced parentheses, or unknown fields must degrade gracefully into text-match (`Expr::Text`), allowing the user to search literally for exactly what they typed without encountering hard crashes.
- **Round-Trip Integrity**: The `Display` implementation for `Expr<F, S>` must exactly recreate a parseable string that produces an identical AST. This ensures that user perspectives can be serialized to disk and loaded without semantic drift.

## Development Commands
```bash
cargo check
cargo test
cargo fmt
```
