# vir-search Specification

**Domain**: Expression parsing and AST generation.
**Language**: Rust (edition 2024).
**Dependencies**: `chrono`, `regex`, `unicode-normalization`.

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

## 3. Fallback and Degradation Policies

The parser enforces a strict "never fail" policy, structurally: there is no
error channel in the parse path, so a partial query can never take the whole
query down with it.
- If a token resembles a field syntax (`unknown:value`) but the domain's `ParseField` implementation returns `None`, the parser emits a warning in the `ParseResult` and degrades the node to `Expr::Text("unknown:value")`.
- A trailing logical operator (`foo AND`), a missing value (`genre:`, `title:=`), or EOF mid-expression degrades locally: what already parsed stands, the broken fragment becomes a visible text node carrying the partial expression, and a warning is recorded.
- Unbalanced parentheses keep the successfully parsed content and record a warning, rather than flattening the input into one text node.
- Standalone punctuation degrades to a text node of its literal form (`?`, `!=`, `..`); a stray `)` reads as nothing.
- A quoted value is literal text: `genre:"true"` is a substring match, never the boolean presence check that the unquoted `genre:true` means. A relational comparator on a text field (`author:>=Sanderson`) degrades to its visible text form rather than dropping the query.
- The lexer handles quotes, backslash escapes (`\"`, `\\`), and unicode safely without panicking; `Display` escapes backslashes before quotes so rendering round-trips.

## 4. Relevance Ranking and Fuzzy Matching

The library provides `blend_relevance`, a mathematical heuristic combining a `bm25` Full-Text Search score with an exponential recency decay. It also provides `collect_text_terms`, which traverses the generic AST to harvest bare-text components, skipping negated subtrees (`NOT x` is not a positive term). Consumers use these extracted strings to supply their underlying SQLite FTS queries while bypassing the strictly fielded constraints.

The `fuzzy` module centralizes the shared fuzzy matcher: `damerau_levenshtein` (optimal string alignment), `within` (the bounded, early-exiting predicate), `threshold` (length-aware bands: 1-4 characters tolerate one edit, 5-7 two, longer three), and `hit` (accent-folded, whole-candidate-or-any-word). It ships no evaluation; consumers call it from their own field matching.
