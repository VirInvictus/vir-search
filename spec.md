# Specification: vir-search

## 1. Scope
`vir-search` provides a generic, domain-agnostic parser for Calibre-style search expressions, yielding an `Expr<F, S>` AST. It is designed specifically for integration with `vir-gtk` applications like Atrium and Conservatory.

## 2. API Surface
- `parse<F, S, K>(input: &str) -> ParseResult<F, S, K>`
- AST Nodes: `Empty`, `Text(String)`, `Field(F, MatchKind)`, `Compare(F, Comparator, Value)`, `Range(F, Value, Value)`, `State(S)`, `Not`, `And`, `Or`.
- MatchKinds: Substring, Exact, Regex, Fuzzy, HasAny, HasNone.
- Comparators: `=`, `!=`, `<`, `<=`, `>`, `>=`.
- Traits: `ParseField`, `ParseState`, `ParseSort`. `ParseField` must expose `field_type()` to distinguish between String, Int, Real, and Date parsing paths.

## 3. Fallback and Degradation
- If a token looks like a field (e.g. `unknown:value`) but the domain's `ParseField` returns `None`, the parser emits a warning and returns `Expr::Text("unknown:value")`.
- Unbalanced parentheses degrade safely into text matches.
- The lexer never panics.

## 4. Relevance Ranking
- `blend_relevance` heuristic combines an input `bm25` FTS score with an exponential recency decay.
- `collect_text_terms` traverses the generic AST and harvests bare-text components, skipping fielded searches, to supply the SQLite FTS query.
