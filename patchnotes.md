# vir-search Patch Notes

## v1.0.0 (2026-08-23)

**Initial Release (Library Extraction)**
`vir-search` has been extracted from `atrium-search` and `conservatory-search` into a standalone, domain-agnostic Rust library.

*   **Generic AST**: Replaced hardcoded `Field`, `State`, and `SortKey` enums with generic types `F`, `S`, and `K`. The AST is now `Expr<F, S>`, allowing it to power book, audiobook, podcast, and music queries interchangeably without domain-coupling.
*   **Trait Boundaries**: Introduced `ParseField`, `ParseState`, and `ParseSort` traits. Consumers now implement these on their own enums.
*   **Field Type Introspection**: Added `FieldType` (`String`, `Int`, `Real`, `Date`) and the `field_type()` method to `ParseField`. This allows the generic parser to know when to parse raw values into numerics or date specifications.
*   **Lexer and Parser**: Retained the forgiving, never-panic recursive descent parser and lexer that degrades malformed syntax into text-matches seamlessly.
*   **Removed Domain Code**: Specifically excluded `eval.rs` and `sql_translate.rs` from extraction, as those implementations are inherently tied to consumer SQLite schemas and data models.
