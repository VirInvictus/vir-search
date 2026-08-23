use crate::ast::{Expr, ParseField, ParseState};

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

pub fn collect_text_terms<F: ParseField, S: ParseState>(expr: &Expr<F, S>) -> Vec<String> {
    let mut out = Vec::new();
    walk(expr, &mut out);
    out
}

fn walk<F: ParseField, S: ParseState>(expr: &Expr<F, S>, out: &mut Vec<String>) {
    match expr {
        Expr::Text(s) if !s.is_empty() => out.push(s.clone()),
        Expr::Not(inner) => walk(inner, out),
        Expr::And(items) | Expr::Or(items) => items.iter().for_each(|e| walk(e, out)),
        _ => {}
    }
}
