# vir-search Patch Notes

## v1.4.3 (2026-09-15)

**The final-audit hop.** The Display/round-trip contract's semantic holes
close, the dead regex dependency goes, and a generated-input fuzz now
guards the bug class that shipped twice.

*   **`genre:"true"` renders quoted (MEDIUM).** `quote_if_needed` did not
    quote values the grammar intercepts bare, so a quoted-bool or quoted
    wildcard (`genre:"true"`, `genre:"ambient*"`) displayed bare and
    re-parsed to `HasAny`/`HasNone`/`Prefix`/`Suffix`: a round-trip that
    flipped meaning, the same class as 1.4.1's month-keyword gap. The
    boundary predicate and `bool_word` are now single shared helpers, and
    the semantic assertions cover the cases end to end.
*   **A quoted In-list body is literal text (MEDIUM, decided 2026-09-15).**
    `genre:("rock,jazz")` parsed as an `In` matcher, contradicting the four
    "quoted is always literal" doc statements, and a multi-word body broke
    Display round-trip outright. Rejected over documenting the form: the
    quotes never become list syntax, so the visible fragment degrades to
    text with a warning; the unquoted list is untouched.
*   **Ymd years outside 0..=9999 degrade at parse (decided 2026-09-15).**
    `added:262143` parsed into a `DateSpec` whose resolution silently fell
    back to 1970, inverting ranges warning-free. The year is validated at
    parse; out-of-range degrades to visible text with the bad-value warning.
*   **Two more shape-stability holes, caught by the new fuzz on its first
    run.** A negation whose operand degraded to `Empty` (EOF, or a `sort:`
    that extracted itself) wrapped into `Not(Empty)`, whose Display renders
    a bare `NOT ` that steals the next token on re-parse; negating nothing
    is now nothing. And nested combinators (`And` under `And`, `Or` under
    `Or`) rendered flat and re-associated into a different shape; Display
    now parenthesizes them.
*   **Degradation polish.** A balanced empty group `()` claims its closer
    instead of warning "unclosed" about a paren that closed; `bogus:>=5`
    degrades to one visible fragment instead of three text nodes; and the
    degradation log is bounded (100 entries, 3 copies per message) with a
    tail "and N more suppressed" summary in both lists.
*   **The regex dependency is removed (decided 2026-09-15).** Zero use
    sites over the crate's entire life: parser+AST-only by charter, with
    `MatchKind::Regex` a String consumers interpret. The lock shrinks five
    packages; the docs stop advertising a dependency the code never had.
*   **Generated-input round-trip fuzz.** 5000 seeded LCG cases over grammar
    pieces, each shape-stable through two render/re-parse generations, plus
    an exhaustive per-field value sweep: the hand-written corpus could not
    see the month-keyword bug or the quoted-bool flip because neither input
    was in the list.
*   **Onboarding and API hygiene.** `examples/mini_consumer.rs` is the
    compile-checked minimal consumer (three traits, parse, a Visitor),
    built by CI as a trait-bound canary; `ParseResult` and `QueryCache`
    derive `Debug`.
*   **Comment/doc truth batch.** The `PerspectiveResolver` doc no longer
    claims plain parse uses the `()` resolver; the module header names the
    reported-twice boundaries; sub-diagnostic spans are documented as
    indexing the perspective's stored text (spec 2.3, `Diagnostic`,
    `parse_with_resolver`); the negation-cap rustdoc states the parity
    rounding (at most 65 kept); the fold.rs FTS5 claim is softened to the
    verified truth (fold strips all of U+0300-036F, FTS5's table is
    narrower for Cyrillic breve, neither side touches Hebrew niqqud or
    Arabic harakat); the cross-repo cites are self-contained; the README
    grammar table gains the juxtaposition row, `kb`, `sort:+key`, and
    fully prefixed date-keyword examples; spec 2.1 writes named-field
    variants; the README/spec/roadmap prose passes cut the superlatives
    and recast the em-dashes.

Suite: 72 green (20 unit + 39 parse + 3 rank + 8 cache + 2 fuzz), clippy
`-D warnings` clean, `cargo fmt --check` clean.

## v1.4.2 (2026-09-13)

**Robustness release.** Three never-fail holes close and the docs catch up
to the code; no grammar changes.

*   **Bounded recursion (HIGH).** `"("*50000` used to overflow the stack
    inside the parser (an abort no caller can catch), and `"!"*100000`
    parsed into a `Not` chain whose `Drop`/`Display`/`visit` overflowed
    later. The parser now carries a depth budget of 128: a `(` past the
    budget degrades to its literal text with a spanned warning, the budget
    threads through `vl:` perspective expansion, and one factor wraps at
    most 64 negation marks, dropping the rest with parity preserved so the
    capped chain negates exactly when the full run would have. Deep input
    parses to a shallow, warning-carrying tree that still round-trips.
*   **A stray `)` no longer discards the rest of the query (MEDIUM).**
    `author:x ) title:y` used to drop `title:y` silently, contradicting the
    degrade-and-record policy. A top-level closer is now consumed with a
    spanned "unmatched" warning and the parser keeps collecting.
*   **Out-of-range date offsets saturate instead of panicking (MEDIUM).**
    `added:4294967295daysago` parsed cleanly and aborted at resolve. The
    week/offset arms of `resolve_range` now walk `checked_add_days`/
    `checked_sub_days` with saturating fallbacks, so an absurd offset
    clamps to the representable date edges.
*   **Docs/API:** crate-level rustdoc plus a `missing_docs` lint (CI's
    clippy `-D warnings` gate enforces it), with the whole public surface
    documented; the README's grammar list becomes a normative table; the
    README's false "resolved during parsing using chrono" claim is
    corrected (dates stay symbolic `DateSpec` until the consumer calls
    `resolve_range`); the spec's degradation policy records the
    bounded-recursion and stray-closer behavior; `Cargo.toml` gains
    keywords and categories; the crate guidance names the clippy gate CI
    actually runs.

Suite: 64 green (20 unit + 33 parse + 3 rank + 8 cache), clippy
`-D warnings` clean, `cargo fmt --check` clean.

## v1.4.1 (2026-09-12)

**Bugfix + the decided hashable-AST design.** The month-keyword parse gap
closes, and the AST learns to cross cache boundaries safely.

*   **`added:lastmonth` and `added:nextmonth` parse.** 1.3.0 shipped the
    `DateSpec::LastMonth`/`NextMonth` variants with resolution and Display
    forms but never taught `parse_date_spec` the keyword arms, so both
    degraded to text nodes while Display emitted exactly the strings that
    failed to re-parse. Semantic tests pin both keywords with warn-free
    parses (the shape-stable round-trip harness could not see this one).
*   **Hashable AST, with `Value::Real` dropped from hashability** (decided
    2026-09-11). `Expr`, `MatchKind`, `Comparator`, `DateSpec`, `SortSpec`,
    and `Value` implement `Eq`/`Hash`; `Value::Real`'s payload never enters
    a hash (the variant hashes as a bare discriminant), so there are no f64
    bit-pattern, `-0.0`, or NaN hazards. `Expr::contains_real()` recognizes
    real-carrying queries; caches keyed on `Expr` skip them instead of
    hashing them.
*   **`QueryCache<F, S, K>`** (new `cache` module): a dependency-free LRU
    memoizing `parse` keyed by the raw query string, for search-as-you-type.
    Real-carrying queries bypass it in both directions (never written,
    hence never served). SQL-translation memoization stays consumer-side,
    keyed on the now-hashable tree. `ParseResult` gained `Clone` (additive)
    to be cacheable.
*   **Recorded dispositions** (both decided 2026-09-11): the Viaduct
    integration blueprint is waived for now (revisit on an expressed
    Viaduct need), and `collect_text_terms` keeping repeated terms is
    intentional (classic term-frequency weighting).
*   **Docs sync:** the crate guidance header states the real stack (edition
    2024, rust-version 1.85, regex + chrono + unicode-normalization); the
    stale "for the binary" comment in `dates.rs` is rewritten; the spec
    gains the `vl:`/`PerspectiveResolver` semantics and the hashing/cache
    contract; the README documents `QueryCache`.

Suite: 58 green (19 unit + 28 parse + 3 rank + 8 cache), clippy
`-D warnings` clean, `cargo fmt --check` clean.

## v1.4.0 (2026-09-04)

**Phase 2: AST inspector traits.** `Visitor` (read-only, parents before
children, `enter` returning `false` skips a subtree) and `Folder`
(bottom-up, `fold_node` replaces each node after its children), both as
`Expr::visit` / `Expr::fold_nodes`. This is what hosts use to extract active
states and fields, check SQL-eligibility, or apply structural rewrites
without hand-rolling traversal. The method names deliberately avoid `fold`
to not collide with the accent-folding `fold` module. Suite: 49 green
(19 unit + 27 parse + 3 rank).

## v1.3.0 (2026-09-04)

**Phase 2: grammar expansion.** Three new query surfaces, all round-trip
safe and spanned by the 1.2.0 diagnostics:

*   **Relative dates**: `added:in3days` (and the general `in<N>days`),
    `added:lastmonth`, `added:nextmonth`, plus compact offsets `added:+7d`
    and `added:-14d`. New `DateSpec::InDays/LastMonth/NextMonth` with
    resolution arms (month neighbors handle year boundaries).
*   **Wildcard and list matchers** on text fields: `genre:ambient*` is
    `Prefix`, `genre:*metal` is `Suffix`, and `genre:(rock,jazz)` is
    `In` (no spaces inside the list; a quoted value is still literal).
    One star per term; `*both*` stays a literal substring.
*   **Human durations** on Real fields: `duration:>=1h30m` and
    `duration:>90m` parse to seconds (h/m/s/d chains sum), and magnitude
    suffixes scale (`320k` -> 320000, `50mb` -> 5e7). A bare `m` means
    minutes; megabytes are written `mb`.

Suite: 47 green (19 unit + 25 parse + 3 rank).

## v1.2.0 (2026-09-04)

**Phase 2: diagnostic spans.** The lexer keeps each token's byte span
(`lex_with_spans`), and `ParseResult` gains `diagnostics: Vec<Diagnostic>`
(`message`, `start`, `end` byte offsets into the input, end exclusive). Every
degradation that already produced a warning now also records the span a search
bar should underline: the unknown field word, the field missing its value, the
unclosed parenthesis (through end of input), the offending value on a bad
numeric/date or range bound, the perspective name on cycles. `warnings` is
unchanged and still carries every message; every diagnostic also appears there.
`lex()` keeps its exact old signature. Consumers only read `ParseResult`, so
the new field is additive. Suite: 41 green (18 unit + 20 parse + 3 rank).

## v1.1.0 (2026-09-04)

**Phase 2: centralized fuzzy matching.** New `vir_search::fuzzy` module:

*   `damerau_levenshtein(a, b)` — optimal string alignment, the exact
    distance (transpositions count as one edit: "wrok" ↔ "work").
*   `within(a, b, max)` — the bounded predicate with a length-difference
    short-circuit and an early exit once a row's running minimum exceeds
    `max`; same answer as the exact comparison, cheaper on non-matches.
    Property-checked against the exact distance over a sample grid in both
    directions.
*   `threshold(needle_len)` — the length-aware bands both consumers
    shipped (1-4 chars → 1 edit, 5-7 → 2, longer → 3).
*   `hit(candidate, needle)` — accent-folded fuzzy hit: the needle within
    threshold of the whole candidate or any of its whitespace-separated
    words.

Behavior notes for consumers adopting it: Atrium's copy gains accent
folding (its `levenshtein_within` expected pre-lowercased input but did
not fold — `?bjork` now matches `Björk`, the documented forgiving-kind
rule); Conservatory's copy is a straight replacement (same threshold
bands, same fold, same word split). Consumer adoption lands with the
consumer wave, not this release.

Suite: 34 green (15 unit + 16 parse + 3 rank).

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
