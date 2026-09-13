# vir-search Roadmap

## Phase 1: Core Parsing & Extraction (Completed)
- [x] Extract generic recursive-descent parser from Atrium and Conservatory.
- [x] Define strictly typed `Expr<F, S>` and AST primitives.
- [x] Implement generic sorting traits (`ParseSort`).
- [x] Port `chrono` date arithmetic and BM25 ranking heuristics.

## Phase 2: Engine Expansion & Downstream Optimization
*Context: Enhancing diagnostic feedback, AST manipulation, and optimizing downstream app evaluations (Atrium, Conservatory, Viaduct).*

- [x] **Diagnostic Source Spans:** Add byte offsets (`Span`) to `lex.rs` and `parse.rs` so GTK4 search bars in Atrium/Conservatory can draw red squiggly underlines on typos. *(Shipped 1.2.0: `lex_with_spans()` keeps byte spans per token and `ParseResult.diagnostics` pairs every degradation warning with the span to underline; `lex()`/`warnings` unchanged, purely additive.)*
- [x] **Hashable AST & LRU Query Cache:** Make `Expr` hashable and add `QueryCache<F, S, K>` to memoize parsing and SQL translation for rapid search-as-you-type in all apps. *(Shipped 1.4.1 per the 2026-09-11 decision: `Value::Real` is dropped from hashability, its payload never entering a `Hash` impl, and `Expr::contains_real()` recognizes real-carrying queries so caches skip them instead of hashing. The crate's `QueryCache` memoizes `parse` keyed by the query string; SQL-translation memoization stays consumer-side, keyed on the now-hashable `Expr`.)*
- [x] **Centralized Fuzzy Matching:** Move Damerau-Levenshtein and `fuzzy_threshold` logic into `vir_search::fuzzy` so Atrium and Conservatory can drop 130+ lines of duplicate code. *(Shipped 1.1.0: `damerau_levenshtein` + `within` (Atrium's early-exit form) + `threshold` + `hit` (fold-aware, whole-or-word). Consumer adoption rides the consumer wave: Conservatory is a straight swap; Atrium's copy gains accent folding as a deliberate improvement.)*
- [x] **Relative Date Grammar Expansion:** Add `in3days`, `lastmonth`, `nextmonth`, `+7d` offset parsing. (Impacts: Atrium forward-looking tasks, Conservatory audio timelines). *(Shipped 1.3.0: `InDays`/`LastMonth`/`NextMonth` variants with resolution and Display round-trips; `-14d` compact form rides along.)*
- [x] **Prefix, Suffix, and `IN` AST Matchers:** Add `MatchKind::Prefix` (`foo*`), `Suffix` (`*bar`), and `In` (`(a,b)`). Allows Atrium/Conservatory to use SQLite index scans instead of heavyweight regex queries. *(Shipped 1.3.0: unquoted single-word values only; quoted values stay literal substring; Display round-trips.)*
- [x] **Human Duration Unit Parsing:** Support `1h30m`, `320k`, `50MB` normalizations. (Impacts: Conservatory track lengths, Atrium `estimated` times). *(Shipped 1.3.0: h/m/s/d chains sum to seconds for Real fields; k/mb/gb magnitude suffixes scale; bare `m` is minutes.)*
- [x] **Generic AST Inspector Traits:** Introduce `Visitor` and `Folder` to quickly extract active states, fields, and check if an AST is SQL-eligible. *(Shipped 1.4.0: `Expr::visit` with subtree skipping, `Expr::fold_nodes` bottom-up; trait methods named `enter`/`fold_node` so they do not collide with the accent-folding `fold` module.)*
- [x] **Viaduct Integration Blueprint:** Map RSS concepts (`feed:`, `is:unread`) into `vir-search` traits and adopt it inside Viaduct's timeline filter. *(Waived 2026-09-11, decided: no Viaduct need for the grammar is expressed today, so the blueprint is not built on spec. Revisit the moment Viaduct asks; the traits are generic enough that the mapping is a proposal document, not a code change.)*

## Phase 3: Robustness & Logic Bug Fixes (2026-08-23)
*Context: Found critical AST parsing flaws, edge cases causing catastrophic parse failure, and documentation desyncs during a rigorous codebase sweep.*

### Bugs to Fix
- [x] **DateSpec Keywords Omission:** Parse `tomorrow`, `lastweek`, and `nextweek` correctly instead of silently degrading them to text nodes. *(Shipped 1.0.4: `parse_date_spec` produces the variants; the resolvers existed since 1.0.0. 1a821cd had added the variants without touching the parser.)*
- [x] **Month Keywords Omission:** Parse `lastmonth` and `nextmonth` instead of degrading them to text nodes. *(Shipped 1.4.1: the 1.3.0 release added the `DateSpec::LastMonth`/`NextMonth` variants, their resolvers, and Display forms but never the `parse_date_spec` keyword arms, so `added:lastmonth` degraded while Display emitted exactly the strings that failed to re-parse (the 1a821cd bug class recurring). The shape-stable round-trip harness could not catch it; the new semantic tests pin both keywords and warn-free parses.)*
- [x] **Non-Recursive Negation:** Fix `boolean_factor` so double negation (`NOT NOT a`) parses correctly instead of producing corrupted ASTs. *(Shipped 1.0.4: consecutive negations fold, `!` and `not` both.)*
- [x] **Catastrophic Degradation:** Stop returning `Err(())` on trailing tokens (e.g. `foo AND`). Handle partial syntax gracefully so the query doesn't completely collapse to `Expr::Empty`. *(Shipped 1.0.4: the error channel is gone entirely — the parse path returns `Expr` end to end; failures degrade locally (EOF, missing values, unclosed parens keep their content) and the combinators fold `Empty` out. The test that pinned the old whole-input flattening flipped deliberately.)*
- [x] **Standalone Punctuation Erasure:** Stop erasing standalone symbols (`?`, `!`, `>`, etc.) in the fallback predicate, which currently returns empty strings. *(Shipped 1.0.4: every operator token renders its literal text; a stray `)` degrades to `Empty`, its only sensible reading.)*
- [x] **Substring Interception:** Prevent `"true"` and `"false"` string matches (`genre:"true"`) from being falsely converted into `MatchKind::HasAny`. *(Shipped 1.0.4: quoted values are literal on text fields and numeric/date fields alike; the unquoted forms keep the presence check.)*
- [x] **Relational Operators on String Fields:** Support string comparisons (`author:>=Sanderson`) instead of dropping the entire query. *(Shipped 1.0.4 as degradation: the comparator + value become the visible text node `author:>=Sanderson` with a warning, the established bad-numeric-value pattern; true string ordering would be a semantic addition and stays unwanted until a consumer asks.)*
- [x] **Lexer Quote Escapes:** Properly recognize escaped backslashes (`\\`) so closing quotes aren't ignored. *(Shipped 1.0.4: `scan_quoted` handles `\\` alongside `\"`, and `quote_if_needed` escapes backslashes before quotes so `Display` round-trips.)*
- [x] **Negated FTS Terms:** Prevent `collect_text_terms` from extracting negated terms (`NOT ambient`) for positive SQLite FTS lookups. *(Shipped 1.0.4: `Not` subtrees are skipped.)*
- [x] **Repeated Term Dedup:** `collect_text_terms` keeps duplicates, so `ambient ambient` harvests the term twice. *(Recorded as intentional 2026-09-11, decided: repeated terms weighting rank higher is classic term-frequency behavior; dedup would silently change consumer ranking, so it stays until a consumer asks.)*
- [x] **Docs Sync:** Correct `Cargo.toml` edition vs `spec.md`, update dependencies list, and clean up stale upstream comments in `dates.rs` and `fold.rs`. *(Shipped 1.0.4: spec says edition 2024 + full dep list; the phantom spec §3.4 references in `lex.rs`/`ast.rs` headers and the `eval`/`sql_translate`/`Phase 18a`/`norm_key` comments are gone; README grammar list names `sort:`/`vl:` and the new date keywords; MIT LICENSE file added — the crate declared MIT but shipped no text.)*

## New findings 2026-09-12 (six-lens full audit; detail: audit/FULL-AUDIT-2026-09-12.md, Wave 11)

- [x] **HIGH: unbounded recursion.** "("*50000 overflows the stack in
      predicate/boolean_expr (an abort, not a panic - violating the
      never-fail contract), and "!"*100000 parses into a Not chain whose
      Drop/Display/visit overflow later. Add a parser depth cap that
      degrades the fragment to text (spanned warning), and bound negation
      wrapping the same way; thread the depth through resolve_vl. *(Shipped
      1.4.2: a depth budget of 128 shared by parentheses and `vl:` expansion
      degrades an over-deep `(` to its literal text with a spanned warning;
      one factor wraps at most 64 negation marks, dropping the rest with
      parity preserved so the capped chain negates exactly when the full
      run would have. Deep input parses to a shallow, warning-carrying
      tree; the new tests pin degradation, depth bounds, parity, and
      round-trips of the degraded trees.)*
- [x] **MEDIUM: chrono Days operators panic on out-of-range offsets**
      (dates.rs week/DaysAgo/InDays arms): added:4294967295daysago parses
      cleanly and aborts at resolve. Use checked_add/sub_days with
      saturating fallback (the next_day/prev_day pattern already in file). *(Shipped
      1.4.2: `add_days`/`sub_days` wrap the checked chrono ops with
      saturating fallbacks to `NaiveDate::MAX`/`MIN`, and every arm,
      including next_day/prev_day, rides them; an absurd offset clamps to
      the representable edge. Pinned by a dates unit test at both edges
      plus the reported repro end to end (parse then resolve).)*
- [x] **MEDIUM: a stray ) silently discards the rest of the query**
      (boolean_term breaks at RParen and the remainder is dropped without
      a warning - contradicting spec 3). Consume it, warn_spanned, and
      keep collecting. *(Shipped 1.4.2: a top-level `)` is consumed with a
      spanned "unmatched" warning in both places one is reached - the
      term loop and the predicate arm a leading stray hits - and parsing
      continues; `author:x ) title:y` now equals
      `author:x AND title:y` plus a warning.)*
- [x] **Docs/API:** README's "resolved during parsing using chrono" is
      false (symbolic DateSpec; consumer-invoked resolve_range); no
      crate-level rustdoc or missing_docs; parse/Expr/DateSpec/MatchKind/
      rank.rs undocumented while the 1.4.x additions are; no normative
      grammar table exists anywhere (add one to README or a spec
      appendix); clippy missing from CLAUDE.md commands. *(Done 1.4.2:
      crate-level rustdoc + `#![warn(missing_docs)]` (CI clippy enforces)
      with the full public surface documented; README grammar is now a
      normative table and the date claim is rewritten (symbolic DateSpec,
      consumer-invoked `resolve_range`, saturating edges); spec 3 records
      the stray-closer and bounded-recursion policy; CLAUDE.md names the
      clippy gate.)*
- [ ] **Consumer-doc drift found by the cross-repo lens (their lanes):**
      Atrium's spec documents Ndaysout and a quoted-exact form that do
      not exist; Conservatory's keyword list is missing seven families
      and its spec example teaches "is:finished false"; Conservatory's
      roadmap misattributes SQL translation to vir-search ("lives in
      vir-search's sql_translate" - no such module exists). Fix on the
      consumer side; the attribution error matters before any push-down
      work is routed.
- [ ] **Blitz candidates:** fuzz/property target pinning never-fail +
      Display round-trip (the 1.4.1 Display bug class recurred and the
      existing harness cannot catch it); diagnostic ergonomics (severity,
      suggestions, line/col - the spans shipped so search bars could draw
      squiggles and no consumer consumes them); the SQL push-down scaffold
      (MatchKind shapes + walker in-library after the boundary record is
      fixed; per-schema fragments stay consumer-side); QueryCache hit-path
      tuning only once a consumer adopts it.
- [ ] **GitHub presentation (workspace batch):** description empty,
      topics null, zero Releases, Cargo.toml lacks keywords/categories -
      proposals drafted in the ledger.
