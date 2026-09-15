# vir-search API

The public surface of the crate, for consumers (Atrium, Conservatory) and
reviewers. The rustdoc is the compile-checked reference; this file is the
orientation layer: what exists, what it is for, and the contracts that do
not fit on one doc line. Current for v1.4.3.

## Entry points (`parse` module)

- `parse<F: ParseField, S: ParseState, K: ParseSort>(input: &str) -> ParseResult<F, S, K>`:
  the front door. Never fails; degradations land in the tree as text nodes
  and in the logs.
- `parse_with_resolver<F, S, K, R: PerspectiveResolver<F, S>>(input, &resolver)`:
  same, plus `vl:name` perspective expansion (cycle-guarded, depth-bounded).
  Sub-parse warnings, diagnostics, and sorts merge into the outer result;
  merged diagnostics keep the perspective's stored text as their coordinate
  system, so a span can exceed the outer query's byte length (spec 2.3).
- `PerspectiveResolver`: one method, `expression(&self, name) -> Option<String>`,
  the named perspective's stored query text. `()` implements it as
  always-`None` (every perspective then degrades to `Empty` with a warning);
  plain `parse` attaches no resolver and degrades `vl:` silently to text.
- `ParseResult<F, S, K>`: `expr` (the tree), `sorts` (extracted `sort:`
  directives, in query order), `warnings` (flat log), `diagnostics`
  (spanned log, byte spans `start..end` exclusive). Both logs are bounded:
  at most 100 recorded entries, at most 3 copies of one message, tail entry
  "and N more degradations suppressed" (span `0..0`). Derives `Clone, Debug`.
- `Diagnostic { message, start, end }`.

## The AST (`ast` module, re-exported at the crate root)

- `Expr<F, S>`: `Empty`, `Text(String)`, `Field { field, kind }`,
  `Compare { field, comp, value }`, `Range { field, low, high }`,
  `State(S)`, `Not(Box<_>)`, `And(Vec<_>)`, `Or(Vec<_>)`. Named-field
  variants; derive-friendly (`Debug, Clone, PartialEq, Hash` + `Eq`).
- `Expr::contains_real()`: does the tree carry a `Value::Real`? Caches keyed
  on `Expr` must skip such queries (see the hashing contract below).
- `Expr::visit(&Visitor)` / `Expr::fold_nodes(&Folder)`: read-only walk
  (parents first; `enter` returning `false` skips a subtree) and bottom-up
  transformation (`fold_node` replaces each node after its children). The
  `enter`/`fold_node` names avoid colliding with the accent-folding `fold`
  module; keep the shape.
- `MatchKind`: `Substring`, `Exact`, `Regex` (a String the consumer
  interprets; this crate links no regex engine), `Fuzzy`, `Prefix`, `Suffix`,
  `In(Vec<String>)`, `HasAny`, `HasNone`.
- `Comparator` (`as_str()` gives the query spelling), `Value`
  (`Int`/`Real`/`Date`/`Text`), `DateSpec` (symbolic: `Today` ...
  `ThisYear`, `DaysAgo(u32)`, `InDays(u32)`, `Ymd(i32, Option<u32>, Option<u32>)`),
  `SortSpec<K> { key, descending }`.
- `FieldType`: `String`/`Int`/`Real`/`Date`; the value grammar a field's
  values parse under.
- `quote_if_needed(&str) -> String`: the Display-side quoting rule, shared
  boundary logic with the lexer (`lex::is_boundary`) so a rendered value
  re-lexes to the same node.

## Consumer traits

- `ParseField`: `parse(name) -> Option<Self>` (name resolution) and
  `field_type(&self) -> FieldType` (steers value parsing: `Date` engages the
  date grammar, `Real` floats/durations/magnitudes, `Int` integers;
  everything else is text).
- `ParseState`: `parse(name) -> Option<Self>` for `is:*`.
- `ParseSort`: `parse(name) -> Option<Self>` for `sort:*` (`-`/`+` prefixes
  set `descending`).

A compile-checked minimal consumer lives in `examples/mini_consumer.rs`.

## Supporting modules

- `dates`: `resolve_range(&DateSpec, today: NaiveDate) -> (i64, i64)`
  (a `[start, end)` epoch-seconds UTC window; saturates at the
  representable edges instead of panicking), `matches(comp, value, start, end)`
  (precision-aware comparator semantics), `today_utc()`. Dates stay symbolic
  until the consumer calls `resolve_range`; nothing bakes in a timestamp.
- `fold`: `fold(&str) -> String`, NFD + strip U+0300-036F + lowercase, the
  forgiving-match accent folding (verified against FTS5's
  `unicode61 remove_diacritics 2` for Latin and Greek text; see the module
  docs for the exact corners).
- `fuzzy`: `damerau_levenshtein`, `within` (bounded, early-exiting),
  `threshold` (up to 4 chars tolerate 1 edit, 5-7 two, longer three),
  `hit` (accent-folded, whole-candidate-or-any-word).
- `rank`: `blend_relevance(bm25, days_since, half_life_days)` and
  `collect_text_terms(&Expr)` (bare-text harvest, duplicates kept for
  term-frequency weighting, negated subtrees skipped).
- `cache`: `QueryCache<F, S, K>`: LRU memoization of `parse` keyed on the
  raw query string; real-carrying queries bypass it in both directions.
- `lex`: `lex`, `lex_with_spans` (tokens with byte spans), `Token`, `Spanned`.

## Contracts that span the surface

- **Never fail**: no error channel anywhere in the parse path. Malformed
  input degrades locally into `Text`/`Empty` with warnings; recursion and
  the degradation log are bounded, so hostile input cannot overflow a stack
  or flood a log.
- **Display round-trips**: `format!("{}", expr)` re-parses to an identical
  AST, guarded by the hand-written corpus and the generated-input fuzz
  (`tests/fuzz_round_trip.rs`). New grammar needs its own semantic
  assertions; shape stability alone cannot see meaning flips.
- **Hashing**: `Expr` implements `Eq`/`Hash`, but `Value::Real`'s payload is
  deliberately excluded (no f64 hazards); a cache keyed on `Expr` must skip
  queries where `contains_real()` is true. `QueryCache` does.
- **Boundary**: parsing and AST only. No SQL generation, no evaluation;
  consumers translate the tree into their own domain logic.
