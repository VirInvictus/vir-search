//! Tokenizer. Best-effort: it never fails, so the parser can decide
//! how to degrade malformed input.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A bareword (may contain `.`, `-`, digits, unicode).
    Word(String),
    /// A `"quoted string"`.
    Quoted(String),
    Colon,
    LParen,
    RParen,
    Eq,     // =
    Ne,     // !=
    Lt,     // <
    Le,     // <=
    Gt,     // >
    Ge,     // >=
    Tilde,  // ~ (regex prefix)
    Quest,  // ? (fuzzy prefix)
    Bang,   // ! (NOT)
    DotDot, // .. (range)
}

/// A token with its byte span in the source (`start..end`, end exclusive), so
/// consumers can underline the exact broken fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned {
    pub token: Token,
    pub start: usize,
    pub end: usize,
}

/// Characters that always terminate a bareword. Kept in sync with the quoting
/// check in `ast::quote_if_needed` or round-tripping breaks.
fn is_boundary(c: char) -> bool {
    c.is_whitespace() || matches!(c, '(' | ')' | ':' | '"' | '~' | '?' | '!' | '<' | '>' | '=')
}

pub fn lex(input: &str) -> Vec<Token> {
    lex_with_spans(input).into_iter().map(|s| s.token).collect()
}

/// Tokenize, keeping each token's byte span.
pub fn lex_with_spans(input: &str) -> Vec<Spanned> {
    let indexed: Vec<(usize, char)> = input.char_indices().collect();
    let len = input.len();
    // Byte offset of the token after index `to`: the next token's start, or
    // the input end.
    let end_of = |to: usize| indexed.get(to).map_or(len, |(b, _)| *b);

    let mut tokens = Vec::new();
    let mut i = 0;
    while i < indexed.len() {
        let (start, c) = indexed[i];
        let push = |token: Token, next: usize, tokens: &mut Vec<Spanned>| {
            tokens.push(Spanned {
                token,
                start,
                end: end_of(next),
            });
        };
        match c {
            c if c.is_whitespace() => {
                i += 1;
                continue;
            }
            '(' => {
                push(Token::LParen, i + 1, &mut tokens);
                i += 1;
            }
            ')' => {
                push(Token::RParen, i + 1, &mut tokens);
                i += 1;
            }
            ':' => {
                push(Token::Colon, i + 1, &mut tokens);
                i += 1;
            }
            '~' => {
                push(Token::Tilde, i + 1, &mut tokens);
                i += 1;
            }
            '?' => {
                push(Token::Quest, i + 1, &mut tokens);
                i += 1;
            }
            '=' => {
                push(Token::Eq, i + 1, &mut tokens);
                i += 1;
            }
            '>' => {
                let (t, next) = two(&indexed, i, '=', Token::Ge, Token::Gt);
                push(t, next, &mut tokens);
                i = next;
            }
            '<' => {
                let (t, next) = two(&indexed, i, '=', Token::Le, Token::Lt);
                push(t, next, &mut tokens);
                i = next;
            }
            '!' => {
                let (t, next) = two(&indexed, i, '=', Token::Ne, Token::Bang);
                push(t, next, &mut tokens);
                i = next;
            }
            '"' => {
                let (s, next) = scan_quoted(&indexed, &mut i);
                push(Token::Quoted(s), next, &mut tokens);
                i = next;
            }
            '.' if indexed.get(i + 1).map(|(_, c2)| *c2) == Some('.') => {
                push(Token::DotDot, i + 2, &mut tokens);
                i += 2;
            }
            _ => {
                let (w, next) = scan_word(&indexed, &mut i);
                push(Token::Word(w), next, &mut tokens);
                i = next;
            }
        }
    }
    tokens
}

/// Consume the char at `i`; if the next char is `second`, consume it too and
/// emit `both`, else emit `single`. Returns the token and the next index.
fn two(
    indexed: &[(usize, char)],
    i: usize,
    second: char,
    both: Token,
    single: Token,
) -> (Token, usize) {
    if indexed.get(i + 1).map(|(_, c)| *c) == Some(second) {
        (both, i + 2)
    } else {
        (single, i + 1)
    }
}

/// Scan a `"..."` string (supports `\"` and `\\`). An unterminated quote runs
/// to EOF. Returns the string and the next index.
fn scan_quoted(indexed: &[(usize, char)], i: &mut usize) -> (String, usize) {
    *i += 1; // opening quote
    let mut out = String::new();
    while *i < indexed.len() {
        let c = indexed[*i].1;
        if c == '\\' {
            match indexed.get(*i + 1).map(|(_, c2)| *c2) {
                Some('"') => {
                    out.push('"');
                    *i += 2;
                }
                Some('\\') => {
                    out.push('\\');
                    *i += 2;
                }
                _ => {
                    out.push(c);
                    *i += 1;
                }
            }
        } else if c == '"' {
            *i += 1;
            break;
        } else {
            out.push(c);
            *i += 1;
        }
    }
    (out, *i)
}

/// Scan a bareword. Stops at a boundary char or a `..` (range), keeping a lone
/// `.` inside the word (so `1998..2004` splits but `Mr. X` keeps the dot).
/// Returns the word and the next index.
fn scan_word(indexed: &[(usize, char)], i: &mut usize) -> (String, usize) {
    let mut out = String::new();
    while *i < indexed.len() {
        let c = indexed[*i].1;
        if is_boundary(c) {
            break;
        }
        if c == '.' && indexed.get(*i + 1).map(|(_, c2)| *c2) == Some('.') {
            break;
        }
        out.push(c);
        *i += 1;
    }
    (out, *i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_field_value() {
        assert_eq!(
            lex("artist:Aphex"),
            vec![
                Token::Word("artist".into()),
                Token::Colon,
                Token::Word("Aphex".into())
            ]
        );
    }

    #[test]
    fn lexes_operators_and_quotes() {
        assert_eq!(
            lex("year:>=1990 album:\"Selected Works\""),
            vec![
                Token::Word("year".into()),
                Token::Colon,
                Token::Ge,
                Token::Word("1990".into()),
                Token::Word("album".into()),
                Token::Colon,
                Token::Quoted("Selected Works".into()),
            ]
        );
    }

    #[test]
    fn match_kind_prefixes() {
        assert_eq!(
            lex("title:~rx genre:?amb t:=x"),
            vec![
                Token::Word("title".into()),
                Token::Colon,
                Token::Tilde,
                Token::Word("rx".into()),
                Token::Word("genre".into()),
                Token::Colon,
                Token::Quest,
                Token::Word("amb".into()),
                Token::Word("t".into()),
                Token::Colon,
                Token::Eq,
                Token::Word("x".into()),
            ]
        );
    }

    #[test]
    fn unterminated_quote_runs_to_eof() {
        assert_eq!(lex("\"no end"), vec![Token::Quoted("no end".into())]);
    }

    #[test]
    fn spans_are_byte_offsets() {
        let spans = lex_with_spans("artist:Aphex");
        assert_eq!((spans[0].start, spans[0].end), (0, 6));
        assert_eq!((spans[1].start, spans[1].end), (6, 7));
        assert_eq!((spans[2].start, spans[2].end), (7, 12));
    }

    #[test]
    fn spans_cover_unicode_barewords() {
        // Björk's ö is two bytes: byte spans, not char counts.
        let spans = lex_with_spans("Björk x");
        assert_eq!((spans[0].start, spans[0].end), (0, 6));
        assert_eq!((spans[1].start, spans[1].end), (7, 8));
    }

    #[test]
    fn quoted_span_includes_the_quotes() {
        let spans = lex_with_spans("t:\"a b\"");
        assert_eq!((spans[2].start, spans[2].end), (2, 7));
    }
}
