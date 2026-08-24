# vir-search Roadmap

## Phase 1: Core Parsing & Extraction (Completed)
- [x] Extract generic recursive-descent parser from Atrium and Conservatory.
- [x] Define strictly typed `Expr<F, S>` and AST primitives.
- [x] Implement generic sorting traits (`ParseSort`).
- [x] Port `chrono` date arithmetic and BM25 ranking heuristics.

## Phase 2: Engine Expansion & Downstream Optimization
*Context: Enhancing diagnostic feedback, AST manipulation, and optimizing downstream app evaluations (Atrium, Conservatory, Viaduct).*

- [ ] **Diagnostic Source Spans:** Add byte offsets (`Span`) to `lex.rs` and `parse.rs` so GTK4 search bars in Atrium/Conservatory can draw red squiggly underlines on typos.
- [ ] **Hashable AST & LRU Query Cache:** Make `Expr` hashable and add `QueryCache<F, S, K>` to memoize parsing and SQL translation for rapid search-as-you-type in all apps.
- [ ] **Centralized Fuzzy Matching:** Move Damerau-Levenshtein and `fuzzy_threshold` logic into `vir_search::fuzzy` so Atrium and Conservatory can drop 130+ lines of duplicate code.
- [ ] **Relative Date Grammar Expansion:** Add `in3days`, `lastmonth`, `nextmonth`, `+7d` offset parsing. (Impacts: Atrium forward-looking tasks, Conservatory audio timelines).
- [ ] **Prefix, Suffix, and `IN` AST Matchers:** Add `MatchKind::Prefix` (`foo*`), `Suffix` (`*bar`), and `In` (`(a,b)`). Allows Atrium/Conservatory to use SQLite index scans instead of heavyweight regex queries.
- [ ] **Human Duration Unit Parsing:** Support `1h30m`, `320k`, `50MB` normalizations. (Impacts: Conservatory track lengths, Atrium `estimated` times).
- [ ] **Generic AST Inspector Traits:** Introduce `Visitor` and `Folder` to quickly extract active states, fields, and check if an AST is SQL-eligible.
- [ ] **Viaduct Integration Blueprint:** Map RSS concepts (`feed:`, `is:unread`) into `vir-search` traits and adopt it inside Viaduct's timeline filter.

## Phase 3: Robustness & Logic Bug Fixes (2026-08-23)
*Context: Found critical AST parsing flaws, edge cases causing catastrophic parse failure, and documentation desyncs during a rigorous codebase sweep.*

### Bugs to Fix
- [ ] **DateSpec Keywords Omission:** Parse `tomorrow`, `lastweek`, and `nextweek` correctly instead of silently degrading them to text nodes.
- [ ] **Non-Recursive Negation:** Fix `boolean_factor` so double negation (`NOT NOT a`) parses correctly instead of producing corrupted ASTs.
- [ ] **Catastrophic Degradation:** Stop returning `Err(())` on trailing tokens (e.g. `foo AND`). Handle partial syntax gracefully so the query doesn't completely collapse to `Expr::Empty`.
- [ ] **Standalone Punctuation Erasure:** Stop erasing standalone symbols (`?`, `!`, `>`, etc.) in the fallback predicate, which currently returns empty strings.
- [ ] **Substring Interception:** Prevent `"true"` and `"false"` string matches (`genre:"true"`) from being falsely converted into `MatchKind::HasAny`.
- [ ] **Relational Operators on String Fields:** Support string comparisons (`author:>=Sanderson`) instead of dropping the entire query.
- [ ] **Lexer Quote Escapes:** Properly recognize escaped backslashes (`\\`) so closing quotes aren't ignored.
- [ ] **Negated FTS Terms:** Prevent `collect_text_terms` from extracting negated terms (`NOT ambient`) for positive SQLite FTS lookups.
- [ ] **Docs Sync:** Correct `Cargo.toml` edition vs `spec.md`, update dependencies list, and clean up stale upstream comments in `dates.rs` and `fold.rs`.
