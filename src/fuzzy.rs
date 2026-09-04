//! Fuzzy matching: the shared Damerau-Levenshtein core and the length-aware
//! threshold, so the consumers stop carrying diverging copies. Accent folding
//! is applied here (the forgiving-kind rule from [`crate::fold`]); exact and
//! regex matches stay literal.
//!
//! The threshold bands are the ones both consumers shipped: short needles
//! tolerate one edit, longer ones proportionally more.

use crate::fold::fold;

/// Length-aware fuzzy threshold: a needle of 1-4 characters tolerates one
/// edit, 5-7 tolerate two, longer tolerate three — so multi-character words
/// like "strawberry" survive a missed letter or two without throwing the user
/// back to substring.
pub fn threshold(needle_len: usize) -> usize {
    match needle_len {
        0..=4 => 1,
        5..=7 => 2,
        _ => 3,
    }
}

/// Optimal string alignment (Damerau-Levenshtein with adjacent
/// transpositions): the distance between `a` and `b`, counting a transposition
/// of two adjacent characters as a single edit — the most common typing slip
/// ("wrok" ↔ "work"). The simple, exact answer; prefer [`within`] in hot paths.
pub fn damerau_levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev2 = vec![0usize; m + 1];
    let mut prev = (0..=m).collect::<Vec<_>>();
    let mut curr = vec![0usize; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            // Damerau's transposition rule needs the row two before.
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                curr[j] = curr[j].min(prev2[j - 2] + 1);
            }
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}

/// Whether the Damerau-Levenshtein distance between `a` and `b` is at most
/// `max`, with a length-difference short-circuit and an early exit once a
/// row's running minimum exceeds `max`. Same answer as
/// `damerau_levenshtein(a, b) <= max`, cheaper on non-matches.
pub fn within(a: &str, b: &str, max: usize) -> bool {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    // Length difference alone exceeds the budget — short-circuit.
    if a_chars.len().abs_diff(b_chars.len()) > max {
        return false;
    }
    if a_chars.is_empty() {
        return b_chars.len() <= max;
    }
    if b_chars.is_empty() {
        return a_chars.len() <= max;
    }
    let n = a_chars.len();
    let m = b_chars.len();
    // Three rows for Damerau: the transposition rule looks back two rows.
    let mut prev_prev: Vec<usize> = vec![0; m + 1];
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr: Vec<usize> = vec![0; m + 1];
    for i in 1..=n {
        curr[0] = i;
        let mut row_min = curr[0];
        for j in 1..=m {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            let mut v = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            if i >= 2
                && j >= 2
                && a_chars[i - 1] == b_chars[j - 2]
                && a_chars[i - 2] == b_chars[j - 1]
            {
                v = v.min(prev_prev[j - 2] + 1);
            }
            curr[j] = v;
            row_min = row_min.min(v);
        }
        if row_min > max {
            return false;
        }
        std::mem::swap(&mut prev_prev, &mut prev);
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m] <= max
}

/// Accent-folded fuzzy hit: the needle is within [`threshold`] edits (by
/// needle length) of the whole candidate or any of its whitespace-separated
/// words, both sides folded so `?bjork` fuzzy-matches `Björk`.
pub fn hit(candidate: &str, needle: &str) -> bool {
    let cand = fold(candidate);
    let need = fold(needle);
    let max = threshold(need.chars().count());
    within(&cand, &need, max) || cand.split_whitespace().any(|word| within(word, &need, max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_bands() {
        assert_eq!(threshold(0), 1);
        assert_eq!(threshold(4), 1);
        assert_eq!(threshold(5), 2);
        assert_eq!(threshold(7), 2);
        assert_eq!(threshold(8), 3);
        assert_eq!(threshold(40), 3);
    }

    #[test]
    fn distance_basics() {
        assert_eq!(damerau_levenshtein("", ""), 0);
        assert_eq!(damerau_levenshtein("abc", ""), 3);
        assert_eq!(damerau_levenshtein("", "abc"), 3);
        assert_eq!(damerau_levenshtein("work", "work"), 0);
        assert_eq!(damerau_levenshtein("wrok", "work"), 1); // transposition
        assert_eq!(damerau_levenshtein("strawbery", "strawberry"), 1);
        assert_eq!(damerau_levenshtein("ca", "ac"), 1); // adjacent swap
        assert_eq!(damerau_levenshtein("abc", "xyz"), 3);
    }

    #[test]
    fn within_matches_the_exact_distance() {
        let samples = [
            ("ambient", "ambiant"),
            ("boards of canada", "boards o canada"),
            ("roygbiv", "roygibv"),
            ("electronic", "elektronic"),
            ("a", "ab"),
        ];
        for (a, b) in samples {
            for max in 0..=4usize {
                assert_eq!(
                    within(a, b, max),
                    damerau_levenshtein(a, b) <= max,
                    "within({a:?}, {b:?}, {max}) diverged"
                );
                assert_eq!(
                    within(b, a, max),
                    damerau_levenshtein(b, a) <= max,
                    "within asymmetric at {a:?}/{b:?}/{max}"
                );
            }
        }
        // The short-circuits: a too-long candidate is false without work.
        assert!(!within("a", "abcdef", 2));
        assert!(within("", "", 0));
        assert!(within("ab", "", 2));
    }

    #[test]
    fn hit_folds_and_checks_words() {
        // Accent-folded on both sides.
        assert!(hit("Björk", "bjork"));
        // Whole candidate within threshold.
        assert!(hit("Ambient", "ambiant"));
        // Any whitespace-separated word within threshold.
        assert!(hit("Boards of Canada", "boards"));
        assert!(hit("Music Has the Right to Children", "childen"));
        // No hit: nothing within three edits.
        assert!(!hit("Row-dotted line", "xylophone"));
    }
}
