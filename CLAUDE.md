# vir-search

**Stack:** Rust (edition 2021), `chrono`.
**Status:** Maintained. Standalone library.
**Versioning deviation:** there is no `VERSION` file; `Cargo.toml` is the single version source (a `VERSION` file would be a second carrier Cargo cannot consume). Every bump still gets a patchnotes entry and an annotated tag at the release commit.

## What is this?
A domain-agnostic Rust library for parsing Calibre-style search expressions into a typed AST. Extracted from the `vir-gtk` ecosystem (Atrium, Conservatory) to eliminate parser duplication and provide a unified query language across the desktop suite.

## Key Rules
- **No Domain Logic**: Do not add SQL generation (`sql_translate.rs`) or local object evaluation (`eval.rs`) to this crate. Consumers must translate the generic `Expr<F, S>` into their own domain logic themselves. This crate must remain exclusively a parser and AST definition.
- **`FieldType` Introspection**: Because the parser does not know what `F` is at compile time, consumers implement `ParseField::field_type()`. The parser relies on this to branch its parsing logic (e.g., treating `Duration` as `FieldType::Real` to allow float parsing, or `Added` as `FieldType::Date` to engage the `chrono` date parser).
- **Forgiving Parsing**: The lexer and parser must never fail or panic, and since 1.0.4 the parse path has no error channel at all: every failure degrades locally (a trailing operator or missing value keeps the parsed side; an unclosed parenthesis keeps its content; a quoted value is literal text, never a boolean presence check; a relational comparator on a text field degrades to its visible text form). Malformed input must always degrade into text-match (`Expr::Text`) or `Expr::Empty`, never a crash or a whole-query collapse.
- **Round-Trip Integrity**: The `Display` implementation for `Expr<F, S>` must exactly recreate a parseable string that produces an identical AST. This ensures that user perspectives can be serialized to disk and loaded without semantic drift.

## Development Commands
```bash
cargo check
cargo test
cargo fmt
```
