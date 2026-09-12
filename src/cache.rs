//! LRU memoization of parses for search-as-you-type.
//!
//! [`QueryCache`] keys on the raw query string, so it needs nothing from the
//! consumer's types beyond the parse traits. Memoizing SQL translation stays
//! consumer-side: translate the parsed `Expr` once and key that cache on the
//! now-hashable tree, skipping queries where [`Expr::contains_real`](crate::Expr::contains_real)
//! is true.

use std::collections::{HashMap, VecDeque};

use crate::ast::{ParseField, ParseSort, ParseState};
use crate::parse::{ParseResult, parse};

/// Memoizes [`parse`] results keyed by the raw query string, LRU-bounded.
///
/// Queries carrying a real-number literal are never cached: [`Value::Real`](crate::ast::Value::Real)
/// is dropped from hashing by design, so such a query bypasses the cache in
/// both directions (it is never stored, hence never served). The cost is one
/// [`Expr::contains_real`](crate::Expr::contains_real) walk per miss.
pub struct QueryCache<F, S, K> {
    capacity: usize,
    entries: HashMap<String, ParseResult<F, S, K>>,
    /// Keys, least-recently-used first.
    order: VecDeque<String>,
}

impl<F: ParseField, S: ParseState, K: ParseSort> QueryCache<F, S, K> {
    /// A cache holding at most `capacity` results (minimum one).
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// The memoized parse: a hit returns the cached result; a miss parses and
    /// stores, unless the tree carries a real literal (then it only parses).
    pub fn parse(&mut self, input: &str) -> ParseResult<F, S, K> {
        if let Some(hit) = self.hit(input) {
            return hit;
        }
        let result = parse(input);
        if !result.expr.contains_real() {
            if self.entries.len() >= self.capacity {
                if let Some(oldest) = self.order.pop_front() {
                    self.entries.remove(&oldest);
                }
            }
            self.order.push_back(input.to_string());
            self.entries.insert(input.to_string(), result.clone());
        }
        result
    }

    /// Is `input` currently cached? (Real-carrying queries always answer
    /// false; they are never stored.)
    pub fn cached(&self, input: &str) -> bool {
        self.entries.contains_key(input)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    fn hit(&mut self, input: &str) -> Option<ParseResult<F, S, K>> {
        let result = self.entries.get(input)?.clone();
        // Refresh recency: move the key to the back of the eviction order.
        if let Some(pos) = self.order.iter().position(|k| k == input) {
            self.order.remove(pos);
        }
        self.order.push_back(input.to_string());
        Some(result)
    }
}
