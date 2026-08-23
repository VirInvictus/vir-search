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
