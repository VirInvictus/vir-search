# vir-search

## Overview
Domain-agnostic Rust library for parsing Calibre-style search expressions into a typed AST. Extracted from the `vir-gtk` ecosystem (Atrium, Conservatory) to eliminate parser duplication.

## Architecture
- **No Domain Logic**: `eval.rs` and `sql_translate.rs` were deliberately stripped during extraction. Consumers must translate the generic `Expr<F, S>` into their own domain (e.g. SQLite queries or in-memory evaluation) themselves.
- **`FieldType` Inspection**: Since the parser does not know what `F` is, consumers must implement `ParseField::field_type()` so the parser knows whether to parse a field value as a string, int, real, or date.
- **Forgiving Parsing**: The lexer and parser must never fail or panic. Malformed syntax or unknown fields degrade into text-match (`Expr::Text`), allowing the user to search literally for what they typed.

## Development
- **Build**: `cargo build`
- **Test**: `cargo test` (runs parser, lexer, dates, fold, and rank tests)

## House Style
- **Rust Edition**: 2021
- **Documentation**: All public APIs must be documented. Update `patchnotes.md` on every release.
