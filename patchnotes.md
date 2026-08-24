## v1.0.2 (2026-08-24)

- **Build:** Removed unused MatchKind import to fix clippy warnings blocking the build.
## v1.0.1 (2026-08-23)

- **Build:** build: add GitHub Actions CI workflow

# vir-search Patch Notes

## v1.0.0 (2026-08-23)

**Initial Release (Library Extraction)**
`vir-search` has been extracted from `atrium-search` and `conservatory-search` into a standalone, domain-agnostic Rust library. This extraction eliminates the parser duplication that existed across the VirInvictus desktop suite, unifying the search grammar under a single robust engine.

*   **Generic AST Parameterization**: Replaced the previously hardcoded `Field`, `State`, and `SortKey` enums with fully generic types `F`, `S`, and `K`. The AST is now shaped as `Expr<F, S>`, allowing it to power book, audiobook, podcast, and music queries interchangeably without any internal domain-coupling.
*   **Trait Boundaries**: Introduced the `ParseField`, `ParseState`, and `ParseSort` traits. Consumers of the library must now implement these traits on their own local enums to define their supported vocabulary.
*   **Field Type Introspection**: Added the `FieldType` enum (`String`, `Int`, `Real`, `Date`) and the `field_type()` introspection method to the `ParseField` trait. This architectural change allows the generic parser to dynamically identify when to parse raw token values into exact numerics or dynamic date specifications (`chrono`) based on the consumer's metadata layout.
*   **Lexer and Parser Porting**: Retained the forgiving, never-panic recursive descent parser and custom lexer. Malformed syntax, unknown fields, and unbalanced logical operators continue to degrade into text-matches seamlessly, preserving the user experience.
*   **Domain Code Exclusion**: Deliberately excluded `eval.rs` and `sql_translate.rs` from the extraction process. Those implementations are inherently tied to consumer SQLite schemas and data models, meaning they remain the responsibility of the calling application.
