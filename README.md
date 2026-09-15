# vir-search

[![CI](https://github.com/VirInvictus/vir-search/actions/workflows/ci.yml/badge.svg)](https://github.com/VirInvictus/vir-search/actions/workflows/ci.yml)

A domain-agnostic Rust library for parsing Calibre-style search expressions into a typed Abstract Syntax Tree (AST).

Extracted from `atrium-search` and `conservatory-search`, `vir-search` provides the lexer, generic recursive-descent parser, ranking heuristics, and date-range resolvers that underpin the VirInvictus ecosystem. By parameterizing the AST over the consumer's `Field`, `State`, and `SortKey` types, it avoids domain-coupling while keeping one search grammar across the desktop suite.

## Architecture and Scope

The library is explicitly designed to handle the parsing stage of a query pipeline. It does *not* provide SQL translation or in-memory evaluation logic; consumers must translate the resulting `Expr<F, S>` into their own domain logic (e.g., SQLite FTS queries for Conservatory, or `SearchItem` evaluations for Atrium).

### The Parser (`vir_search::parse`)

The core parser uses recursive descent to produce an AST. It is designed around a **"never fail"** philosophy: syntax errors, unrecognized fields, or malformed constraints do not panic or return hard errors. Instead, they degrade locally and visibly into raw text-matching nodes. This ensures that users can always fall back to standard full-text search behavior even if they type something the parser doesn't natively understand.

### The Grammar (normative)

| Form | Example | AST / meaning |
|---|---|---|
| Bareword | `roygbiv` | `Text`: free-text match (FTS or substring) |
| Quoted string | `"boards of canada"` | `Text`: literal; never intercepted by wildcards or boolean words |
| Field + value | `author:sanderson` | `Field` with `MatchKind::Substring` |
| Exact | `author:=Brandon` | `Field` with `MatchKind::Exact` |
| Regex | `title:~^Live` | `Field` with `MatchKind::Regex` (applied consumer-side) |
| Fuzzy | `title:?stromlite` | `Field` with `MatchKind::Fuzzy`: Damerau-Levenshtein with a length-aware threshold, accent-folded (the shared `fuzzy` module) |
| Prefix / suffix | `title:foo*`, `author:*bar` | `Field` with `MatchKind::Prefix` / `Suffix` (unquoted single-word values only) |
| Juxtaposition | `author:x title:y` | implicit `AND`: adjacent terms conjoin, no operator needed |
| In-list | `genre:(rock,jazz)` | `Field` with `MatchKind::In`: an OR over the members; no spaces inside the list, and a quoted body stays literal text (never list syntax) |
| Presence | `genre:true` / `genre:false` | `Field` with `MatchKind::HasAny` / `HasNone` (quoted forms stay literal substring) |
| Relational | `rating:>=4`, `duration:<600` | `Compare` on `Int`/`Real`/`Date` fields; on a text field the fragment degrades to visible text |
| Range | `year:2020..2023` | `Range` (low inclusive, high exclusive) |
| Date keywords | `added:today`, `added:yesterday`, `added:tomorrow`, `added:thisweek`, `added:lastweek`, `added:nextweek`, `added:thismonth`, `added:lastmonth`, `added:nextmonth`, `added:thisyear` | `Compare` with the symbolic `DateSpec` (bare keywords without a field parse as free text) |
| Relative dates | `added:3daysago`, `added:in7days`, `added:+7d`, `added:-14d` | `Compare` with `DateSpec::DaysAgo(n)` / `InDays(n)` |
| Calendar dates | `added:2024`, `added:2024-06`, `added:2024-06-08` | `Compare` with `DateSpec::Ymd` |
| Durations | `duration:1h30m`, `duration:90m`, `duration:2d12h` | `Value::Real` seconds on `Real` fields |
| Magnitudes | `size:320k`, `size:<50mb` | `Value::Real` scaled (`k`/`kb`/`mb`/`gb`) |
| State | `is:finished` | `State` (the consumer's `ParseState`) |
| Sort | `sort:-added`, `sort:+added` | Extracted into `ParseResult.sorts`; the node itself is empty (`-` descending, `+` or bare ascending) |
| Perspective | `vl:reading` | Expanded through the consumer's resolver (`parse_with_resolver`) |
| Logic | `a AND b`, `a OR b`, `NOT a`, `!a`, `( ... )` | `And` / `Or` / `Not`; doubled negation parses |

Date tokens never resolve at parse time: they stay symbolic `DateSpec` values, and `dates::resolve_range(spec, today)` turns one into a concrete `[start, end)` epoch-seconds (UTC) window whenever the consumer asks, so a stored query never bakes in a timestamp. Resolution saturates at the representable date edges, so an absurd offset (`added:4294967295daysago`) clamps instead of panicking.

Degradation is local and visible: a quoted value is always literal text (`genre:"true"` is a substring match, not the `genre:true` presence check; a quoted In-list body stays literal too rather than becoming list syntax), a trailing operator or missing value keeps what already parsed, a stray `)` is consumed with a warning while the rest of the query keeps parsing, and recursion is bounded (a pathologically deep fragment degrades to text instead of overflowing the stack). A `Y..` calendar year outside 0..=9999 degrades with a warning instead of resolving through a fallback that would silently invert a range. Every degraded fragment records a warning in `ParseResult.warnings` plus a byte-spanned entry in `ParseResult.diagnostics`, so a UI can underline the exact broken fragment; the log is bounded, with a tail entry counting anything suppressed.

## Usage

To use `vir-search`, you must implement `ParseField`, `ParseState`, and `ParseSort` on your domain enums. Your `ParseField` implementation must return a `FieldType` so the parser knows whether to attempt numeric/date parsing or fall back to strings. A complete compile-checked minimal consumer lives in `examples/mini_consumer.rs`.

```rust
use vir_search::parse::parse;
use vir_search::ast::{FieldType, ParseField, ParseState, ParseSort};

#[derive(Clone, PartialEq, Eq)]
enum MyField { Author, Rating }

impl std::fmt::Display for MyField { /* ... */ }

impl ParseField for MyField {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "author" => Some(Self::Author),
            "rating" => Some(Self::Rating),
            _ => None,
        }
    }
    
    fn field_type(&self) -> FieldType {
        match self {
            Self::Rating => FieldType::Int,
            Self::Author => FieldType::String,
        }
    }
}

// ... implement MyState and MySort ...

// Parse the user's input into an AST
let result = parse::<MyField, MyState, MySort>("author:sanderson rating:>=4");
println!("Parsed expression: {}", result.expr);
```

### Search-as-you-type: `QueryCache`

For rapid repeated parsing, `QueryCache::new(capacity)` memoizes parses keyed
by the raw query string, LRU-bounded. Queries containing a real-number literal
(`rating:>=4.5`, `duration:>90m`) bypass the cache by design: `Value::Real` is
deliberately excluded from hashing, so such queries are recognized with
`Expr::contains_real()` and skipped rather than hashed. The parsed `Expr`
itself implements `Eq` + `Hash`, so consumer-side caches (for example over SQL
translation) can key on the tree, applying the same skip.

## Support

If vir-search's useful to you and you'd like to chip in:

- liberapay · [liberapay.com/bdkl](https://liberapay.com/bdkl/)
- bitcoin
  ```
  bc1qkge6zr45tzqfwfmvma2ylumt6mg7wlwmhr05yv
  ```
