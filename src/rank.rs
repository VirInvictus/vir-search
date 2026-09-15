//! Ranking heuristics: the BM25-plus-recency blend and the FTS term harvest.

use crate::ast::{Expr, ParseField, ParseState};

/// Blend an FTS (BM25-style) score with an exponential recency decay into one
/// relevance number: `|bm25| / (1 + |bm25|) + 0.25 * 0.5^(days_since /
/// half_life_days)`. A `half_life_days` of zero disables the recency term.
pub fn blend_relevance(bm25: f64, days_since: i64, half_life_days: f64) -> f64 {
    let mag = bm25.abs();
    let relevance = mag / (1.0 + mag);
    let recency = if half_life_days > 0.0 {
        0.5f64.powf(days_since.max(0) as f64 / half_life_days)
    } else {
        0.0
    };
    relevance + 0.25 * recency
}

/// Harvest the bare-text terms of a query for positive FTS lookups, in query
/// order. Duplicates are kept on purpose (classic term-frequency weighting,
/// recorded 2026-09-11); negated subtrees stay out (`NOT ambient` is not a
/// positive term). Fielded constraints are skipped: consumers supply those
/// from the tree itself.
pub fn collect_text_terms<F: ParseField, S: ParseState>(expr: &Expr<F, S>) -> Vec<String> {
    let mut out = Vec::new();
    walk(expr, &mut out);
    out
}

fn walk<F: ParseField, S: ParseState>(expr: &Expr<F, S>, out: &mut Vec<String>) {
    match expr {
        Expr::Text(s) if !s.is_empty() => out.push(s.clone()),
        Expr::And(items) | Expr::Or(items) => items.iter().for_each(|e| walk(e, out)),
        // Everything else stays out, negated subtrees included (`NOT ambient`
        // is not a positive FTS term); fielded constraints are the consumer's
        // to harvest from the tree itself.
        _ => {}
    }
}
