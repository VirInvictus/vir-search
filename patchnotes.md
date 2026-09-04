# vir-search Patch Notes

## v1.0.4 (2026-09-04)

**Phase 3 robustness: the parse path has no error channel any more.** The
nine logic bugs from the 2026-08-23 sweep, each verified against the source
before fixing:

*   **Catastrophic degradation is structurally impossible.** The internal
    `Result` plumbing is gone; every failure degrades locally instead of
    collapsing the whole query to `Expr::Empty`. `foo AND` parses as `foo`;
    an unclosed parenthesis keeps the successfully parsed content
    (`(genre:ambient` now yields the genre field match with a warning, not
    a flat text blob — the round-trip test that pinned the old flattening
    flipped deliberately); a missing value (`genre:`, `title:=`) degrades to
    a visible text node carrying the partial expression.
*   **`NOT NOT x` negates twice.** `boolean_factor` handled a single `NOT`,
    so the second one was consumed as the literal text "not". Consecutive
    negations now fold.
*   **`tomorrow`, `lastweek`, `nextweek` parse.** The `DateSpec` variants and
    resolvers existed since 1.0.0, but `parse_date_spec` never produced them
    (1a821cd added the variants without touching the parser — a lesson in
    closing bugs from the log rather than the code).
*   **Quoted values are literal text.** `genre:"true"` was intercepted into
    `MatchKind::HasAny`; the boolean presence check now requires the unquoted
    form, on text fields and numeric/date fields alike.
*   **Relational comparators on text fields degrade to text.**
    `author:>=Sanderson` discarded the entire query through the error path;
    it now warns and degrades to the text node `author:>=Sanderson`, matching
    the existing bad-numeric-value pattern, and the rest of the query
    survives.
*   **Standalone punctuation survives.** The fallback node rebuilt only
    `:`/`=`/`~` and silently dropped `?`, `!`, `!=`, `<`, `<=`, `>`, `>=`,
    and `..`; every operator token now degrades to its literal text. A stray
    `)` reads as nothing, its only sensible meaning.
*   **The lexer understands `\\`.** `scan_quoted` handled only `\"`, so a
    literal backslash swallowed the closing quote and ran the string to EOF.
    `quote_if_needed` now escapes backslashes before quotes so `Display`
    round-trips values containing either.
*   **Negated terms stay out of FTS harvesting.** `collect_text_terms`
    recursed into `Not`, so `NOT ambient` was harvested as a positive term.
*   **Docs sync:** spec edition (2021 → 2024) and dependency list corrected;
    stale `atrium-search` section references removed from file headers; the
    `eval`/`sql_translate` and `Phase 18a` upstream comments rewritten; the
    README grammar list names `sort:`, `vl:`, and the new date keywords; and
    the MIT LICENSE file exists now — the crate declared MIT in `Cargo.toml`
    but shipped no license text.

Suite: 30 green (11 unit + 16 parse integration + 3 rank), clippy `-D
warnings` clean, `cargo fmt --check` clean.

## v1.0.3 (2026-08-25)

- **Housekeeping:** Removed the AI-porting scratch scripts (`rewrite_ast.py`, `rewrite_parse.py`, `rewrite_parse_lex.py`, `rewrite_rank.py`, `vir-search-port.py`) that were accidentally committed to the repository root during extraction.

## v1.0.2 (2026-08-24)

- **Build:** Removed unused MatchKind import to fix clippy warnings blocking the build.

## v1.0.1 (2026-08-23)

- **Build:** add GitHub Actions CI workflow

## v1.0.0 (2026-08-23)

**Initial Release (Library Extraction)**
`vir-search` has been extracted from `atrium-search` and `conservatory-search` into a standalone, domain-agnostic Rust library. This extraction eliminates the parser duplication that existed across the VirInvictus desktop suite, unifying the search grammar under a single robust engine.

*   **Generic AST Parameterization**: Replaced the previously hardcoded `Field`, `State`, and `SortKey` enums with fully generic types `F`, `S`, and `K`. The AST is now shaped as `Expr<F, S>`, allowing it to power book, audiobook, podcast, and music queries interchangeably without any internal domain-coupling.
*   **Trait Boundaries**: Introduced the `ParseField`, `ParseState`, and `ParseSort` traits. Consumers of the library must now implement these traits on their own local enums to define their supported vocabulary.
*   **Field Type Introspection**: Added the `FieldType` enum (`String`, `Int`, `Real`, `Date`) and the `field_type()` introspection method to the `ParseField` trait. This architectural change allows the generic parser to dynamically identify when to parse raw token values into exact numerics or dynamic date specifications (`chrono`) based on the consumer's metadata layout.
*   **Lexer and Parser Porting**: Retained the forgiving, never-panic recursive descent parser and custom lexer. Malformed syntax, unknown fields, and unbalanced logical operators continue to degrade into text-matches seamlessly, preserving the user experience.
*   **Domain Code Exclusion**: Deliberately excluded `eval.rs` and `sql_translate.rs` from the extraction process. Those implementations are inherently tied to consumer SQLite schemas and data models, meaning they remain the responsibility of the calling application.
