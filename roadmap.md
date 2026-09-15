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
- [x] **GitHub presentation (workspace batch):** description empty,
      topics null, zero Releases, Cargo.toml lacks keywords/categories -
      proposals drafted in the ledger. *(Applied 2026-09-13 per decision
      60: description set from the README lead, 10 discovery topics
      (rust, parser, lexer, query-language, abstract-syntax-tree, search,
      calibre, fuzzy-search, text-processing, bm25), Releases created for
      v1.0.4 through v1.4.2 from the tag annotations (the verbatim
      patchnotes entries), wiki off, discussions on; Cargo.toml gained
      keywords/categories in the 1.4.2 bump.)*

### Final audit 2026-09-13 (THE FINAL AUDIT: NEW findings, one line each; full detail in audit-final/vir-search/FINAL-REPORT.md)
- [ ] MED — Display round-trip flips semantics for grammar-intercepted bare words: quote_if_needed doesn't quote true/false/`*` values, so `genre:"true"` (Substring by the 1.0.4 rule) renders bare and re-parses as HasAny/HasNone/Prefix/Suffix (ast.rs:399-416). Add the cases + corpus + semantic assertion; extract the shared boundary helper (is_boundary vs quote_if_needed's closure are hand-synced).
- [ ] MED — Quoted In-list contradicts four "quoted is always literal" doc statements (README:42/:26, spec:53, roadmap:16) and breaks round-trip when an item contains a boundary (`genre:("hip hop,ambient")` renders unquoted, re-parses to malformed Text). Decide reject-vs-document; per-item quoting in Display.
- [ ] MED — resolve_vl merges sub-parse diagnostics whose byte spans index the perspective's stored text, not the outer query (spans can exceed the query length; parse.rs:660-664). Document the coordinate system before the diagnostic-ergonomics lane builds on spans.
- [ ] MED — regex dependency has zero use sites over the crate's entire life (structural: parser+AST-only charter). Removal needs sign-off + 1.4.3-class release + doc sync (spec:5, CLAUDE:3) + consumer lock hop.
- [ ] MED — Consumer drift (CONFIRMED, their lanes): Atrium spec teaches Ndaysout (unparseable) and quoted-exact `tag:"=x y"` (parses literal); Conservatory spec.md:199 is:finished false still live + sql_translate misattribution + search-grammar release mis-attribution (1.0.4/1.4.1, not 1.3.0).
- [ ] MED — GitHub: main unprotected while both consumers pin by rev (block force-push + deletion, nothing else); declared MSRV 1.85 verified by nothing (add a pinned 1.85.0 CI job); no tag trigger, no CI badge, private vulnerability reporting off.
- [ ] LOW — ymd 1970 fallback can invert ranges at the representable edge (`added:262143` → start > end; `added:<262143` matches everything, warning-free). Clamp the year at parse (range decision: 0..=9999 vs chrono max) or saturate the fallback.
- [ ] LOW — Negation-cap off-by-one (65 marks says "exceed the cap; keeping 65" vs the at-most-64 doc); balanced-empty group eats its closer then warns "unclosed parenthesis"; And-under-And renders unparenthesized (shape-level round-trip break, semantics preserved).
- [ ] LOW — PerspectiveResolver trait doc false about plain parse (no resolver attached; vl: degrades silently); module header "every degradation reported twice" false for the resolver-less arm; rank.rs:33 comment on the wrong arm (dissolve via Expr::visit rewrite); fold.rs norm_key cross-repo cite + dates.rs CalibreQuarry attribution + today_utc doc on the wrong function + add_month precondition doc missing; In-list "quoted is literal" comment invites the false reading at that position; stray-closer test comment predates the 1.4.2 warning; fuzzy threshold doc "1-4" vs the 0..=4 arm; lex.rs Token doc duplicates the module header.
- [ ] LOW — Normative grammar table gaps: date-keyword examples need the field prefix on every row (bare words parse as Text); juxtaposition (implicit AND) row missing; kb suffix and sort:+key forms missing; spec AST notation is tuple-style vs named-field variants; vacuous malformed-list test assertion (compares parse(x) to itself); CLAUDE.md chrono claim (chrono enters only at resolve_range).
- [ ] LOW — fold's U+0300-036F strip may not match FTS5 unicode61 remove_diacritics=2 beyond Latin (verify or soften the dual-path claim); ParseResult/QueryCache missing Debug derives; unknown-field degradation eats only the plain value (bogus:>=5 → three text nodes); warnings uncapped on pathological input ("("*50000 → ~49.8k duplicates).
- [ ] LOW — Housekeeping: logo.svg absent vs house standard (add or declare); CI lacks a permissions block; Cargo.toml missing readme/publish=false (pin the git-deps-only model); roadmap consumer-drift box (:81-88) itself stale post today's Conservatory fixes.
- [ ] Feature candidates logged (FINAL-REPORT L4, ranked): generated-input round-trip fuzz (no-dep, closes the twice-shipped bug class); regex removal + Ymd validation + examples/mini_consumer + warnings cap as one 1.4.3 hop; SQL push-down scaffold (sequenced with consumer waves, after the Conservatory record fix); diagnostic ergonomics (prepared lane); QueryCache tuning + streaming lexer (parked). Declined-by-default: bare 7d, Calibre-parity operators until a consumer asks.
- [ ] Prose pass: README register stratum (:5 "powerful … entire suite", :13 "gracefully degrade", :46 "Crucially"); roadmap.md:22 "rigorous" self-grade; roadmap em-dashes :28/:35 + six spaced-hyphen dashes in the 2026-09-12 block; spec.md:27 "crucially". Tag-frozen patchnotes dashes stay as-is.

**CONFIRMED-prior (final-audit verification):** missing-value span cosmetics, bare 7d rejection, lexer Vec, QueryCache O(n) hit path (self-gated), Atrium/Conservatory drift items, fold.rs norm_key survivor. SUPERSEDED (verified shipped): depth cap, negation cap, date saturation, stray-closer handling, the whole Wave-11 docs pass, GitHub presentation. Audit-side correction: the sheet says 49 tests; the suite is 64. Slop-reader verdict: human end to end outside the README's opening register and the tag-frozen v1.0.0 entry.
