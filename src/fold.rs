//! Accent-folding for search matching.
//!
//! `fold` removes diacritics and lowercases, so `Björk` and `bjork` match. It
//! strips the combining-mark range U+0300-036F after NFD decomposition: the
//! Latin and Greek diacritics. That tracks the SQL side of the consumers'
//! dual path, SQLite FTS5's `unicode61 remove_diacritics 2`, for Latin text
//! (verified against FTS5 directly, 2026-09-15): FTS5's own table is
//! narrower in a few corners (a Cyrillic й keeps its breve on the SQL side,
//! this function strips it), and neither side strips Hebrew niqqud or
//! Arabic harakat, which NFD does not decompose.
//!
//! This is deliberately narrower than a dedup normalizer: those also fold
//! quote/dash punctuation and collapse whitespace, while search folding only
//! strips diacritics and lowercases, so it never changes token boundaries.
//! Applied only to the *forgiving* match kinds (substring, quoted substring,
//! fuzzy); `=exact` and `~regex` stay literal.

use unicode_normalization::UnicodeNormalization;

/// The combining-mark range dropped after NFD decomposition (the diacritics).
const COMBINING: std::ops::RangeInclusive<char> = '\u{0300}'..='\u{036F}';

/// Diacritic-fold and lowercase `s` for accent-insensitive matching.
pub fn fold(s: &str) -> String {
    s.nfd()
        .filter(|c| !COMBINING.contains(c))
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_diacritics_and_lowercases() {
        assert_eq!(fold("Björk"), "bjork");
        assert_eq!(fold("Sigur Rós"), "sigur ros");
        assert_eq!(fold("Motörhead"), "motorhead");
        assert_eq!(fold("Beyoncé"), "beyonce");
        assert_eq!(fold("Antonín Dvořák"), "antonin dvorak");
    }

    #[test]
    fn ascii_is_only_lowercased() {
        assert_eq!(fold("Aphex Twin"), "aphex twin");
        assert_eq!(fold("MF DOOM"), "mf doom");
    }

    #[test]
    fn is_idempotent() {
        let once = fold("Mötley Crüe");
        assert_eq!(fold(&once), once);
    }

    #[test]
    fn preserves_token_boundaries() {
        // Unlike a dedup normalizer, whitespace runs are not collapsed here.
        assert_eq!(fold("a  b"), "a  b");
    }
}
