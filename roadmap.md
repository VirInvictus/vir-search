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
- [x] **DateSpec Keywords Omission:** Parse `tomorrow`, `lastweek`, and `nextweek` correctly instead of silently degrading them to text nodes. *(Shipped 1.0.4: `parse_date_spec` produces the variants; the resolvers existed since 1.0.0. 1a821cd had added the variants without touching the parser.)*
- [x] **Non-Recursive Negation:** Fix `boolean_factor` so double negation (`NOT NOT a`) parses correctly instead of producing corrupted ASTs. *(Shipped 1.0.4: consecutive negations fold, `!` and `not` both.)*
- [x] **Catastrophic Degradation:** Stop returning `Err(())` on trailing tokens (e.g. `foo AND`). Handle partial syntax gracefully so the query doesn't completely collapse to `Expr::Empty`. *(Shipped 1.0.4: the error channel is gone entirely — the parse path returns `Expr` end to end; failures degrade locally (EOF, missing values, unclosed parens keep their content) and the combinators fold `Empty` out. The test that pinned the old whole-input flattening flipped deliberately.)*
- [x] **Standalone Punctuation Erasure:** Stop erasing standalone symbols (`?`, `!`, `>`, etc.) in the fallback predicate, which currently returns empty strings. *(Shipped 1.0.4: every operator token renders its literal text; a stray `)` degrades to `Empty`, its only sensible reading.)*
- [x] **Substring Interception:** Prevent `"true"` and `"false"` string matches (`genre:"true"`) from being falsely converted into `MatchKind::HasAny`. *(Shipped 1.0.4: quoted values are literal on text fields and numeric/date fields alike; the unquoted forms keep the presence check.)*
- [x] **Relational Operators on String Fields:** Support string comparisons (`author:>=Sanderson`) instead of dropping the entire query. *(Shipped 1.0.4 as degradation: the comparator + value become the visible text node `author:>=Sanderson` with a warning, the established bad-numeric-value pattern; true string ordering would be a semantic addition and stays unwanted until a consumer asks.)*
- [x] **Lexer Quote Escapes:** Properly recognize escaped backslashes (`\\`) so closing quotes aren't ignored. *(Shipped 1.0.4: `scan_quoted` handles `\\` alongside `\"`, and `quote_if_needed` escapes backslashes before quotes so `Display` round-trips.)*
- [x] **Negated FTS Terms:** Prevent `collect_text_terms` from extracting negated terms (`NOT ambient`) for positive SQLite FTS lookups. *(Shipped 1.0.4: `Not` subtrees are skipped.)*
- [x] **Docs Sync:** Correct `Cargo.toml` edition vs `spec.md`, update dependencies list, and clean up stale upstream comments in `dates.rs` and `fold.rs`. *(Shipped 1.0.4: spec says edition 2024 + full dep list; the phantom spec §3.4 references in `lex.rs`/`ast.rs` headers and the `eval`/`sql_translate`/`Phase 18a`/`norm_key` comments are gone; README grammar list names `sort:`/`vl:` and the new date keywords; MIT LICENSE file added — the crate declared MIT but shipped no text.)*
