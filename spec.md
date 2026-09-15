# vir-search Specification

**Domain**: Expression parsing and AST generation.
**Language**: Rust (edition 2024).
**Dependencies**: `chrono`, `unicode-normalization`.

## 1. Scope and Architecture

`vir-search` provides a generic, domain-agnostic parser for Calibre-style search expressions, yielding an `Expr<F, S>` abstract syntax tree. It is designed specifically for integration with `vir-gtk` applications like Atrium and Conservatory, centralizing the complexity of recursive-descent parsing without tightly coupling to their local database schemas.

## 2. API Contract and Generics

The library exposes the following core entrypoint:
`pub fn parse<F: ParseField, S: ParseState, K: ParseSort>(input: &str) -> ParseResult<F, S, K>`

### 2.1 AST Nodes
- `Empty`: Identity node for empty input or degraded cycles.
- `Text(String)`: Bare free text, matched via FTS or substring.
- `Field(F, MatchKind)`: A direct metadata field constraint.
- `Compare(F, Comparator, Value)`: A relational constraint (e.g., `rating:>=4`).
- `Range(F, Value, Value)`: A bounded range (e.g., `year:2020..2023`).
- `State(S)`: A boolean binary state (e.g., `is:read`).
- Logic: `Not(Box<Expr>)`, `And(Vec<Expr>)`, `Or(Vec<Expr>)`.

### 2.2 Traits
Consumers must implement:
- `ParseField`: Provides name resolution and crucially `field_type()` to distinguish parsing paths (String, Int, Real, Date).
- `ParseState`: Resolves `is:*` boolean states.
- `ParseSort`: Resolves `sort:*` directives extracted during parsing.

### 2.3 Perspectives (`vl:`)
`parse_with_resolver(input, resolver)` expands `vl:name` through the
consumer-provided `PerspectiveResolver`, whose `expression(name)` returns the
named perspective's stored query text (or `None`). Expansion semantics:
- No resolver attached (plain `parse`): `vl:name` degrades to the literal text node `vl:name`.
- Unknown name: the node degrades to `Expr::Empty` with a warning and a spanned diagnostic on the name.
- A cycle (`vl:a` -> `vl:b` -> `vl:a`) is cut at the first repeat: the node degrades to `Expr::Empty` with a "perspective cycle" warning. Each expansion branch tracks its own ancestor chain, so a diamond (`a` referencing `b` and `c`, both referencing `d`) is fine.
- The sub-parse is full-fidelity: its warnings, diagnostics, and extracted `sort:` directives merge into the outer `ParseResult`.

## 3. Fallback and Degradation Policies

The parser enforces a strict "never fail" policy, structurally: there is no
error channel in the parse path, so a partial query can never take the whole
query down with it. Every degradation is also reported twice: as a string in
`ParseResult.warnings` and as a `Diagnostic { message, start, end }` in
`ParseResult.diagnostics`, carrying the byte span a UI should underline.
- If a token resembles a field syntax (`unknown:value`) but the domain's `ParseField` implementation returns `None`, the parser emits a warning in the `ParseResult` and degrades the node to `Expr::Text("unknown:value")`.
- A trailing logical operator (`foo AND`), a missing value (`genre:`, `title:=`), or EOF mid-expression degrades locally: what already parsed stands, the broken fragment becomes a visible text node carrying the partial expression, and a warning is recorded.
- Unbalanced parentheses keep the successfully parsed content and record a warning, rather than flattening the input into one text node.
- Standalone punctuation degrades to a text node of its literal form (`?`, `!=`, `..`). A stray `)` reads as nothing, but it is consumed with a warning and the parser keeps collecting: a top-level closer must not silently discard the rest of the query.
- Recursion is bounded (1.4.2): parentheses and chained `vl:` expansions share a fixed depth budget, and a `(` past the budget degrades to its literal text with a spanned warning; a run of more than 64 consecutive negation marks wraps at the cap with parity preserved, dropping the redundant marks with a warning. Pathological input therefore parses to a shallow, warning-carrying tree instead of overflowing the stack in the parse or in the later `Drop`/`Display`/`visit` walks.
- A quoted value is literal text: `genre:"true"` is a substring match, never the boolean presence check that the unquoted `genre:true` means. A relational comparator on a text field (`author:>=Sanderson`) degrades to its visible text form rather than dropping the query.
- Wildcard and list matchers on text fields: `field:base*` (Prefix), `field:*base` (Suffix), `field:(a,b)` (In). Unquoted single-word values only; a quoted value is literal text and never intercepted by wildcards or boolean presence checks.
- The lexer handles quotes, backslash escapes (`\"`, `\\`), and unicode safely without panicking; `Display` escapes backslashes before quotes so rendering round-trips.

## 4. Relevance Ranking and Fuzzy Matching

The library provides `blend_relevance`, a mathematical heuristic combining a `bm25` Full-Text Search score with an exponential recency decay. It also provides `collect_text_terms`, which traverses the generic AST to harvest bare-text components, skipping negated subtrees (`NOT x` is not a positive term). Consumers use these extracted strings to supply their underlying SQLite FTS queries while bypassing the strictly fielded constraints.

The `fuzzy` module centralizes the shared fuzzy matcher: `damerau_levenshtein` (optimal string alignment), `within` (the bounded, early-exiting predicate), `threshold` (length-aware bands: 1-4 characters tolerate one edit, 5-7 two, longer three), and `hit` (accent-folded, whole-candidate-or-any-word). It ships no evaluation; consumers call it from their own field matching.

## 5. Hashing and the Query Cache

`Expr<F, S>` implements `Eq` and `Hash` (as do `MatchKind`, `Comparator`, `DateSpec`, `SortSpec`, and `Value`), with one deliberate exception (decided 2026-09-11): **`Value::Real` is dropped from hashability.** Its payload never enters a `Hash` impl (the variant hashes as a bare discriminant), so there are no f64 bit-pattern, `-0.0`, or NaN hazards anywhere in the crate. The contract that keeps this sound: a cache keyed on `Expr` must skip any query where `Expr::contains_real()` is true, recognizing real-carrying queries instead of hashing them.

`QueryCache<F, S, K>` (module `cache`) is a dependency-free LRU memoizing `parse`, keyed by the raw query string. A real-carrying query bypasses it in both directions: never written, hence never served. Memoizing SQL translation stays consumer-side: translate the parsed `Expr` once and key that cache on the hashable tree, applying the same `contains_real` skip.
