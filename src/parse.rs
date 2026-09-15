//! The recursive-descent parser: tokens in, a typed [`Expr`] out, never an
//! error. Every degradation is local and reported twice (flat warning plus
//! byte-spanned diagnostic).

use std::collections::HashMap;

use crate::ast::{
    Comparator, DateSpec, Expr, MatchKind, ParseField, ParseSort, ParseState, SortSpec, Value,
    bool_word,
};
use crate::lex::{Spanned, Token, lex_with_spans};

/// The parse output: the tree plus everything the parse recorded along the
/// way (extracted sorts, warnings, spanned diagnostics).
#[derive(Clone, Debug)]
pub struct ParseResult<F, S, K> {
    /// The parsed tree. Never a failure: degraded fragments live in here as
    /// text nodes, not in an error channel.
    pub expr: Expr<F, S>,
    /// Sort directives extracted from `sort:` prefixes, in query order.
    pub sorts: Vec<SortSpec<K>>,
    /// Degradations, in order, as plain strings (the flat log). Bounded by
    /// [`MAX_RECORDED`]: repeated and pathological input cannot flood the
    /// log, and a tail entry counts what was suppressed.
    pub warnings: Vec<String>,
    /// The warnings that carry a byte span in the input (`start..end`, end
    /// exclusive), for search-bar underlines. Every diagnostic also appears
    /// in `warnings`, list for list. Bounded like `warnings`; the suppressed
    /// summary's span is `0..0` (it underlines nothing).
    pub diagnostics: Vec<Diagnostic>,
}

/// A degradation worth underlining: what went wrong and which bytes of the
/// input are responsible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// What went wrong, phrased like its sibling warning in `warnings`.
    pub message: String,
    /// First byte of the responsible fragment.
    pub start: usize,
    /// One past the last byte of the responsible fragment.
    pub end: usize,
}

/// Constructor for `Expr::And` / `Expr::Or`, kept in a type alias because the
/// bare fn-pointer type trips `clippy::type_complexity`.
type Combiner<F, S> = fn(Vec<Expr<F, S>>) -> Expr<F, S>;

/// Deepest parenthesized group (or chained `vl:` expansion) the parser will
/// descend into. The never-fail contract is structural, and a stack overflow
/// aborts rather than panics: past this budget the parser degrades the
/// fragment to text with a warning instead of recursing.
const MAX_DEPTH: usize = 128;

/// Consecutive negation marks one factor may wrap. The wrapping loop is
/// iterative, but the resulting `Not` chain is walked recursively by `Drop`,
/// `Display`, and `visit`, so a pathological run (`"!"*100000`) must not
/// become a chain that deep. Extras are dropped with parity preserved: `Not`
/// is involutive, so the capped chain negates exactly when the input did.
const MAX_NEGATIONS: usize = 64;

/// Degradations one parse records in full. Pathological input (`"("*50000`)
/// used to emit a near-copy of the same warning per token; past this many
/// recorded entries further degradations are counted and summarized once
/// ("and N more suppressed") in both lists.
const MAX_RECORDED: usize = 100;

/// Copies of one identical message a parse keeps before suppressing the
/// rest: enough to show a degradation recurs, not enough to flood the log
/// with a single repeated line.
const MAX_DUPLICATES: usize = 3;

/// Expands `vl:name` perspectives for [`parse_with_resolver`]. Returning
/// `None` degrades the node to `Expr::Empty` with a warning; `()` is a
/// built-in resolver that always answers `None` (used by plain [`parse`]).
pub trait PerspectiveResolver<F, S> {
    /// The named perspective's stored query text, if it exists.
    fn expression(&self, name: &str) -> Option<String>;
}

impl<F, S> PerspectiveResolver<F, S> for () {
    fn expression(&self, _name: &str) -> Option<String> {
        None
    }
}

/// Parse a search expression. Never fails: see the crate docs for the
/// degradation contract, the README for the grammar.
pub fn parse<F: ParseField, S: ParseState, K: ParseSort>(input: &str) -> ParseResult<F, S, K> {
    parse_inner::<F, S, K, ()>(input, None, &[], 0)
}

/// Like [`parse`], with a resolver that expands `vl:name` perspective
/// references (cycle-guarded, depth-bounded).
pub fn parse_with_resolver<
    F: ParseField,
    S: ParseState,
    K: ParseSort,
    R: PerspectiveResolver<F, S>,
>(
    input: &str,
    resolver: &R,
) -> ParseResult<F, S, K> {
    parse_inner(input, Some(resolver), &[], 0)
}

fn parse_inner<F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>>(
    input: &str,
    resolver: Option<&R>,
    seen: &[String],
    depth: usize,
) -> ParseResult<F, S, K> {
    let mut p = Parser {
        tokens: lex_with_spans(input),
        pos: 0,
        input_len: input.len(),
        sorts: Vec::new(),
        warnings: Vec::new(),
        diagnostics: Vec::new(),
        dup_counts: HashMap::new(),
        suppressed: 0,
        resolver,
        seen: seen.to_vec(),
        depth,
        paren_depth: 0,
        _marker: std::marker::PhantomData,
    };
    let expr = p.boolean_expr();
    p.finish_summary();
    ParseResult {
        expr,
        sorts: p.sorts,
        warnings: p.warnings,
        diagnostics: p.diagnostics,
    }
}

struct Parser<'a, F, S, K, R> {
    tokens: Vec<Spanned>,
    pos: usize,
    input_len: usize,
    sorts: Vec<SortSpec<K>>,
    warnings: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    /// Copies kept per distinct message, against [`MAX_DUPLICATES`].
    dup_counts: HashMap<String, usize>,
    /// Degradations dropped by the caps, summarized once at the end.
    suppressed: usize,
    resolver: Option<&'a R>,
    seen: Vec<String>,
    /// Recursion budget spent by enclosing groups and `vl:` expansions,
    /// against [`MAX_DEPTH`]. Shared across a perspective chain: the
    /// sub-parse inherits the parent's depth plus one.
    depth: usize,
    /// Paren groups currently open in this parse. A `)` with this at zero
    /// closes nothing: it is a stray, not a closer.
    paren_depth: usize,
    _marker: std::marker::PhantomData<(F, S)>,
}

impl<'a, F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>>
    Parser<'a, F, S, K, R>
{
    fn peek(&mut self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    fn advance_spanned(&mut self) -> Option<Spanned> {
        let s = self.tokens.get(self.pos).cloned()?;
        self.pos += 1;
        Some(s)
    }

    fn advance(&mut self) -> Option<Token> {
        self.advance_spanned().map(|s| s.token)
    }

    /// Byte span of the token just consumed (the value a warning is about),
    /// or the whole input when nothing was consumed yet.
    fn prev_span(&self) -> (usize, usize) {
        self.tokens
            .get(self.pos.wrapping_sub(1))
            .map(|s| (s.start, s.end))
            .unwrap_or((0, self.input_len))
    }

    /// Record a degradation in both lists: `warnings` stays the flat string
    /// log, `diagnostics` carries the span to underline. The caps in
    /// [`MAX_RECORDED`]/[`MAX_DUPLICATES`] drop anything past them;
    /// [`Parser::finish_summary`] closes the log with one count.
    fn warn_spanned(&mut self, message: String, span: (usize, usize)) {
        let copies = self.dup_counts.entry(message.clone()).or_insert(0);
        if self.diagnostics.len() >= MAX_RECORDED || *copies >= MAX_DUPLICATES {
            self.suppressed += 1;
            return;
        }
        *copies += 1;
        self.diagnostics.push(Diagnostic {
            message: message.clone(),
            start: span.0,
            end: span.1,
        });
        self.warnings.push(message);
    }

    /// Append the suppressed-degradations count to both lists when the caps
    /// dropped anything, so a reader always sees that the log is partial.
    fn finish_summary(&mut self) {
        if self.suppressed == 0 {
            return;
        }
        let message = format!(
            "and {} more degradation{} suppressed",
            self.suppressed,
            if self.suppressed == 1 { "" } else { "s" }
        );
        self.warnings.push(message.clone());
        self.diagnostics.push(Diagnostic {
            message,
            start: 0,
            end: 0,
        });
    }

    fn boolean_expr(&mut self) -> Expr<F, S> {
        let mut items = vec![self.boolean_term()];
        while let Some(Token::Word(w)) = self.peek() {
            if w.eq_ignore_ascii_case("or") {
                self.pos += 1;
                items.push(self.boolean_term());
            } else {
                break;
            }
        }
        self.combine(items, Expr::Or)
    }

    fn boolean_term(&mut self) -> Expr<F, S> {
        let mut items = vec![self.boolean_factor()];
        while let Some(t) = self.peek() {
            if t == &Token::RParen {
                if self.paren_depth > 0 {
                    break;
                }
                // No group is open, so this closes nothing: consume it, warn,
                // and keep collecting. Breaking here would silently discard
                // the rest of the query.
                if let Some(sp) = self.advance_spanned() {
                    self.warn_spanned("unmatched ')'; ignored".into(), (sp.start, sp.end));
                }
                continue;
            }
            if let Token::Word(w) = t {
                if w.eq_ignore_ascii_case("or") {
                    break;
                }
                if w.eq_ignore_ascii_case("and") {
                    self.pos += 1;
                    items.push(self.boolean_factor());
                    continue;
                }
            }
            items.push(self.boolean_factor());
        }
        self.combine(items, Expr::And)
    }

    /// Fold `Empty` out of a combinator's operands so a degraded fragment never
    /// dangles: one survivor is the whole node, none is `Empty`.
    fn combine(&self, mut items: Vec<Expr<F, S>>, ctor: Combiner<F, S>) -> Expr<F, S> {
        if items.len() == 1 {
            return items.pop().unwrap();
        }
        items.retain(|e| !matches!(e, Expr::Empty));
        match items.len() {
            0 => Expr::Empty,
            1 => items.pop().unwrap(),
            _ => ctor(items),
        }
    }

    fn boolean_factor(&mut self) -> Expr<F, S> {
        let mut negations = 0usize;
        let marks_before = self.pos;
        loop {
            match self.peek() {
                Some(Token::Bang) => {
                    self.pos += 1;
                    negations += 1;
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("not") => {
                    self.pos += 1;
                    negations += 1;
                }
                _ => break,
            }
        }
        // The marks' byte span, taken before the operand advances prev_span.
        let marks_span = (negations > 0).then(|| {
            (
                self.tokens[marks_before].start,
                self.tokens[self.pos - 1].end,
            )
        });
        let mut expr = self.predicate();
        let wraps = if negations <= MAX_NEGATIONS {
            negations
        } else {
            let mut w = MAX_NEGATIONS;
            if w % 2 != negations % 2 {
                w += 1;
            }
            w
        };
        if negations > wraps {
            let span = marks_span.unwrap_or((0, self.input_len));
            self.warn_spanned(
                format!("{negations} consecutive negations exceed the cap; keeping {wraps}"),
                span,
            );
        }
        for _ in 0..wraps {
            expr = Expr::Not(Box::new(expr));
        }
        expr
    }

    fn predicate(&mut self) -> Expr<F, S> {
        // EOF is a degraded fragment, not an error: the combinators above fold
        // `Empty` out, so a trailing operator keeps what already parsed.
        let Some(sp) = self.advance_spanned() else {
            return Expr::Empty;
        };
        let t_span = (sp.start, sp.end);
        match sp.token {
            Token::LParen => {
                if self.depth >= MAX_DEPTH {
                    // The recursion budget is spent. Degrade this group to
                    // its literal text (the punctuation-degradation pattern)
                    // instead of recursing into a stack overflow; the
                    // remaining tokens keep parsing as siblings.
                    self.warn_spanned("nested too deeply; matching as text".into(), t_span);
                    return text_or_empty("(".into());
                }
                self.depth += 1;
                self.paren_depth += 1;
                let inner = self.boolean_expr();
                self.depth -= 1;
                self.paren_depth -= 1;
                if self.peek() == Some(&Token::RParen) {
                    self.pos += 1;
                } else {
                    self.warn_spanned("unclosed parenthesis".into(), (t_span.0, self.input_len));
                }
                inner
            }
            Token::Word(w) => {
                let w_clone = w.clone();
                if let Some(Token::Colon) = self.peek() {
                    self.pos += 1;
                    let lname = w_clone.to_ascii_lowercase();
                    match lname.as_str() {
                        "is" => {
                            let Some(val) = self.value_string() else {
                                self.warn_spanned("missing value for is:".into(), t_span);
                                return text_or_empty("is:".into());
                            };
                            match S::parse(&val.to_ascii_lowercase()) {
                                Some(s) => Expr::State(s),
                                None => {
                                    self.warn_spanned(
                                        format!("unknown state {val:?}; matching as text"),
                                        self.prev_span(),
                                    );
                                    text_or_empty(format!("is:{val}"))
                                }
                            }
                        }
                        "sort" => {
                            let Some(val) = self.value_string() else {
                                self.warn_spanned("missing value for sort:".into(), t_span);
                                return text_or_empty("sort:".into());
                            };
                            let (key_str, desc) = if let Some(stripped) = val.strip_prefix('-') {
                                (stripped.to_string(), true)
                            } else if let Some(stripped) = val.strip_prefix('+') {
                                (stripped.to_string(), false)
                            } else {
                                (val.clone(), false)
                            };
                            self.sort_spec(key_str, desc, self.prev_span())
                        }
                        "vl" => {
                            let Some(val) = self.value_string() else {
                                self.warn_spanned("missing value for vl:".into(), t_span);
                                return text_or_empty("vl:".into());
                            };
                            self.resolve_vl(&val, self.prev_span())
                        }
                        _ => match F::parse(&lname) {
                            Some(field) => {
                                let ft = field.field_type();
                                if ft == crate::ast::FieldType::Int
                                    || ft == crate::ast::FieldType::Real
                                    || ft == crate::ast::FieldType::Date
                                {
                                    self.relational(field, t_span)
                                } else {
                                    self.text_match(field, t_span)
                                }
                            }
                            None => {
                                let remainder = self.value_string().unwrap_or_default();
                                self.warn_spanned(
                                    format!("unknown field {w_clone:?}; matching as text"),
                                    t_span,
                                );
                                text_or_empty(format!("{w_clone}:{remainder}"))
                            }
                        },
                    }
                } else {
                    text_or_empty(w_clone)
                }
            }
            Token::Quoted(q) => text_or_empty(q),
            Token::RParen => {
                if self.paren_depth == 0 {
                    // Nothing is open, so this closes nothing: a leading or
                    // post-operator stray. Warn instead of silently reading
                    // it as nothing (the boolean_term loop covers the
                    // mid-factor position the same way).
                    self.warn_spanned("unmatched ')'; ignored".into(), t_span);
                }
                Expr::Empty
            }
            Token::Colon => text_or_empty(":".into()),
            Token::Eq => text_or_empty("=".into()),
            Token::Ne => text_or_empty("!=".into()),
            Token::Lt => text_or_empty("<".into()),
            Token::Le => text_or_empty("<=".into()),
            Token::Gt => text_or_empty(">".into()),
            Token::Ge => text_or_empty(">=".into()),
            Token::Tilde => text_or_empty("~".into()),
            Token::Quest => text_or_empty("?".into()),
            Token::DotDot => text_or_empty("..".into()),
            Token::Bang => text_or_empty("!".into()),
        }
    }

    fn sort_spec(&mut self, key_str: String, descending: bool, span: (usize, usize)) -> Expr<F, S> {
        match K::parse(&key_str.to_ascii_lowercase()) {
            Some(k) => {
                self.sorts.push(SortSpec { key: k, descending });
                Expr::Empty
            }
            None => {
                self.warn_spanned(format!("unknown sort key {key_str:?}"), span);
                let pfx = if descending { "-" } else { "" };
                text_or_empty(format!("sort:{pfx}{key_str}"))
            }
        }
    }

    fn text_match(&mut self, field: F, field_span: (usize, usize)) -> Expr<F, S> {
        match self.peek() {
            Some(Token::Eq) => {
                self.pos += 1;
                match self.value_string() {
                    Some(val) => Expr::Field {
                        field,
                        kind: MatchKind::Exact(val),
                    },
                    None => {
                        self.warn_spanned(format!("missing value for {field}:="), field_span);
                        text_or_empty(format!("{field}:="))
                    }
                }
            }
            Some(Token::Tilde) => {
                self.pos += 1;
                match self.value_string() {
                    Some(val) => Expr::Field {
                        field,
                        kind: MatchKind::Regex(val),
                    },
                    None => {
                        self.warn_spanned(format!("missing value for {field}:~"), field_span);
                        text_or_empty(format!("{field}:~"))
                    }
                }
            }
            Some(Token::Quest) => {
                self.pos += 1;
                match self.value_string() {
                    Some(val) => Expr::Field {
                        field,
                        kind: MatchKind::Fuzzy(val),
                    },
                    None => {
                        self.warn_spanned(format!("missing value for {field}:?"), field_span);
                        text_or_empty(format!("{field}:?"))
                    }
                }
            }
            // A relational comparator on a text field is meaningless; degrade
            // to the visible text form (the bad-numeric-value pattern below)
            // instead of discarding the query.
            Some(Token::Ne | Token::Lt | Token::Le | Token::Gt | Token::Ge) => {
                let prefix = self.eat_comparator().map(|c| c.as_str()).unwrap_or("");
                match self.value_string() {
                    Some(val) => {
                        self.warn_spanned(
                            format!(
                                "relational comparator on text field {field}; matching as text"
                            ),
                            field_span,
                        );
                        text_or_empty(format!("{field}:{prefix}{val}"))
                    }
                    None => {
                        self.warn_spanned(
                            format!("missing value for {field}:{prefix}"),
                            field_span,
                        );
                        text_or_empty(format!("{field}:{prefix}"))
                    }
                }
            }
            // `genre:(rock,jazz)`: a parenthesized comma list becomes an
            // In matcher. No spaces inside the list (a space ends the
            // bareword); a quoted value is literal, never a list.
            Some(Token::LParen) => {
                self.pos += 1;
                if matches!(self.peek(), Some(Token::Quoted(_))) {
                    // A quoted list body stays literal (decided 2026-09-15):
                    // the quotes never become list syntax, so the whole
                    // visible fragment degrades to text with a warning.
                    let body = self.value_string().unwrap_or_default();
                    if self.peek() == Some(&Token::RParen) {
                        self.pos += 1;
                    }
                    self.warn_spanned(
                        format!("quoted list for {field}:(...); matching as text"),
                        field_span,
                    );
                    return text_or_empty(format!("{field}:({body})"));
                }
                match self.value_string() {
                    Some(word) if self.peek() == Some(&Token::RParen) => {
                        self.pos += 1;
                        let items: Vec<String> = word
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        if items.is_empty() {
                            self.warn_spanned(format!("empty list for {field}:( )"), field_span);
                            return text_or_empty(format!("{field}:()"));
                        }
                        Expr::Field {
                            field,
                            kind: MatchKind::In(items),
                        }
                    }
                    got => {
                        let got = got.unwrap_or_default();
                        self.warn_spanned(
                            format!("malformed list for {field}:(...); matching as text"),
                            field_span,
                        );
                        text_or_empty(format!("{field}:({got}"))
                    }
                }
            }
            _ => {
                let quoted = matches!(self.peek(), Some(Token::Quoted(_)));
                let Some(val) = self.value_string() else {
                    self.warn_spanned(format!("missing value for {field}:"), field_span);
                    return text_or_empty(format!("{field}:"));
                };
                // A quoted value is literal text: `genre:"true"` is a substring
                // match, never the boolean presence check `genre:true` means.
                if !quoted {
                    if let Some(b) = bool_word(&val) {
                        return Expr::Field {
                            field,
                            kind: if b {
                                MatchKind::HasAny
                            } else {
                                MatchKind::HasNone
                            },
                        };
                    }
                    // Wildcards: one unquoted trailing or leading star selects
                    // prefix/suffix matching on the bareword (barewords never
                    // contain spaces, so Display round-trips).
                    if let Some(base) = val
                        .strip_suffix('*')
                        .filter(|b| !b.is_empty() && !b.contains('*'))
                    {
                        return Expr::Field {
                            field,
                            kind: MatchKind::Prefix(base.to_string()),
                        };
                    }
                    if let Some(base) = val
                        .strip_prefix('*')
                        .filter(|b| !b.is_empty() && !b.contains('*'))
                    {
                        return Expr::Field {
                            field,
                            kind: MatchKind::Suffix(base.to_string()),
                        };
                    }
                }
                Expr::Field {
                    field,
                    kind: MatchKind::Substring(val),
                }
            }
        }
    }

    fn relational(&mut self, field: F, field_span: (usize, usize)) -> Expr<F, S> {
        let comp = self.eat_comparator();
        let prefix = comp.map(|c| c.as_str()).unwrap_or("").to_string();
        let quoted = matches!(self.peek(), Some(Token::Quoted(_)));
        let Some(raw) = self.value_string() else {
            self.warn_spanned(format!("missing value for {field}:{prefix}"), field_span);
            return text_or_empty(format!("{field}:{prefix}"));
        };
        // Handle :true/:false presence checks even on numeric/date fields (a
        // quoted value is literal text, never the presence check).
        if comp.is_none() && !quoted {
            if let Some(b) = bool_word(&raw) {
                return Expr::Field {
                    field,
                    kind: if b {
                        MatchKind::HasAny
                    } else {
                        MatchKind::HasNone
                    },
                };
            }
        }
        let Some(low) = parse_typed_value(field.clone(), &raw) else {
            self.warn_spanned(
                format!("bad numeric/date value {raw:?}; matching as text"),
                self.prev_span(),
            );
            return text_or_empty(format!("{field}:{prefix}{raw}"));
        };
        if comp.is_none() && matches!(self.peek(), Some(Token::DotDot)) {
            self.pos += 1;
            let Some(raw_hi) = self.value_string() else {
                self.warn_spanned(
                    format!("missing value for {field}:{raw}.."),
                    self.prev_span(),
                );
                return text_or_empty(format!("{field}:{raw}.."));
            };
            if let Some(high) = parse_typed_value(field.clone(), &raw_hi) {
                return Expr::Range { field, low, high };
            }
            self.warn_spanned(
                format!("bad range bound {raw_hi:?}; matching as text"),
                self.prev_span(),
            );
            return text_or_empty(format!("{field}:{raw}..{raw_hi}"));
        }
        Expr::Compare {
            field,
            comp: comp.unwrap_or(Comparator::Eq),
            value: low,
        }
    }

    fn eat_comparator(&mut self) -> Option<Comparator> {
        let comp = match self.peek()? {
            Token::Eq => Comparator::Eq,
            Token::Ne => Comparator::Ne,
            Token::Lt => Comparator::Lt,
            Token::Le => Comparator::Le,
            Token::Gt => Comparator::Gt,
            Token::Ge => Comparator::Ge,
            _ => return None,
        };
        self.pos += 1;
        Some(comp)
    }

    fn value_string(&mut self) -> Option<String> {
        match self.advance()? {
            Token::Word(w) => Some(w.clone()),
            Token::Quoted(s) => Some(s.clone()),
            _ => {
                self.pos -= 1;
                None
            }
        }
    }

    fn resolve_vl(&mut self, name: &str, span: (usize, usize)) -> Expr<F, S> {
        let key = name.to_ascii_lowercase();
        let Some(resolver) = self.resolver else {
            return text_or_empty(format!("vl:{name}"));
        };
        if self.seen.iter().any(|s| s == &key) {
            self.warn_spanned(format!("perspective cycle at {name:?}; ignored"), span);
            return Expr::Empty;
        }
        if self.depth >= MAX_DEPTH {
            self.warn_spanned(
                format!("perspective nested too deeply at {name:?}; ignored"),
                span,
            );
            return Expr::Empty;
        }
        let Some(text) = resolver.expression(name) else {
            self.warn_spanned(format!("unknown perspective {name:?}; ignored"), span);
            return Expr::Empty;
        };
        let mut seen = self.seen.clone();
        seen.push(key);
        let sub = parse_inner(&text, self.resolver, &seen, self.depth + 1);
        self.warnings.extend(sub.warnings);
        self.diagnostics.extend(sub.diagnostics);
        self.sorts.extend(sub.sorts);
        sub.expr
    }
}

fn text_or_empty<F, S>(s: String) -> Expr<F, S> {
    if s.is_empty() {
        Expr::Empty
    } else {
        Expr::Text(s)
    }
}

fn parse_typed_value<F: ParseField>(field: F, raw: &str) -> Option<Value> {
    if field.field_type() == crate::ast::FieldType::Date {
        return parse_date_spec(raw).map(Value::Date);
    }
    if field.field_type() == crate::ast::FieldType::Real {
        return raw
            .parse::<f64>()
            .ok()
            .or_else(|| parse_duration(raw))
            .map(Value::Real);
    }
    raw.parse::<i64>().ok().map(Value::Int)
}

fn parse_date_spec(raw: &str) -> Option<DateSpec> {
    let s = raw.to_ascii_lowercase();
    match s.as_str() {
        "today" => return Some(DateSpec::Today),
        "yesterday" => return Some(DateSpec::Yesterday),
        "tomorrow" => return Some(DateSpec::Tomorrow),
        "thisweek" => return Some(DateSpec::ThisWeek),
        "lastweek" => return Some(DateSpec::LastWeek),
        "nextweek" => return Some(DateSpec::NextWeek),
        "thismonth" => return Some(DateSpec::ThisMonth),
        "lastmonth" => return Some(DateSpec::LastMonth),
        "nextmonth" => return Some(DateSpec::NextMonth),
        "thisyear" => return Some(DateSpec::ThisYear),
        _ => {}
    }
    if let Some(n) = s.strip_suffix("daysago") {
        return n.parse::<u32>().ok().map(DateSpec::DaysAgo);
    }
    if let Some(n) = s.strip_prefix("in").and_then(|r| r.strip_suffix("days")) {
        return n.parse::<u32>().ok().map(DateSpec::InDays);
    }
    // Compact offsets: `+7d` forward, `-7d` back.
    if let Some(n) = s.strip_suffix('d').and_then(|r| r.strip_prefix('+')) {
        return n.parse::<u32>().ok().map(DateSpec::InDays);
    }
    if let Some(n) = s.strip_suffix('d').and_then(|r| r.strip_prefix('-')) {
        return n.parse::<u32>().ok().map(DateSpec::DaysAgo);
    }
    let mut parts = s.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    // Years outside 0..=9999 degrade (decided 2026-09-15): resolve_range
    // clamps unrepresentable dates to a 1970 fallback, which silently
    // inverts a range (`added:<262143` matching everything), and Display
    // formats a four-digit year. Rejecting here turns the silent lie into
    // the visible bad-value degradation.
    if !(0..=9999).contains(&year) {
        return None;
    }
    let month = match parts.next() {
        Some(m) => Some(m.parse::<u32>().ok().filter(|m| (1..=12).contains(m))?),
        None => None,
    };
    let day = match parts.next() {
        Some(d) => Some(d.parse::<u32>().ok().filter(|d| (1..=31).contains(d))?),
        None => None,
    };
    if parts.next().is_some() {
        return None;
    }
    Some(DateSpec::Ymd(year, month, day))
}

/// Human duration parsing for Real fields: a chain of (number, time-unit)
/// pairs is summed in seconds (`1h30m` -> 5400, `90m` -> 5400, `2d12h`), and
/// a plain number with a magnitude suffix scales (`320k` -> 320000, `50mb` ->
/// 5e7). A bare `m` means minutes; megabytes are written `mb`.
fn parse_duration(raw: &str) -> Option<f64> {
    let s: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if s.is_empty() {
        return None;
    }
    if let Some(total) = duration_chain(&s) {
        return Some(total);
    }
    // Magnitude suffixes: only reached when no duration chain consumed the
    // string (`50mb`'s `b` is not a digit, so the chain rejects it).
    const MAGNITUDES: &[(&str, f64)] = &[("kb", 1e3), ("k", 1e3), ("mb", 1e6), ("gb", 1e9)];
    for (suffix, factor) in MAGNITUDES {
        if let Some(num) = s.strip_suffix(suffix) {
            return num.parse::<f64>().ok().map(|n| n * factor);
        }
    }
    None
}

/// Sum of one or more (number, time-unit) pairs consuming the whole string.
fn duration_chain(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    // Longest-first so `minutes` wins over `min` over `m`.
    const UNITS: &[(&str, f64)] = &[
        ("seconds", 1.0),
        ("second", 1.0),
        ("secs", 1.0),
        ("sec", 1.0),
        ("minutes", 60.0),
        ("minute", 60.0),
        ("mins", 60.0),
        ("min", 60.0),
        ("hours", 3600.0),
        ("hour", 3600.0),
        ("hrs", 3600.0),
        ("hr", 3600.0),
        ("days", 86400.0),
        ("day", 86400.0),
        ("s", 1.0),
        ("m", 60.0),
        ("h", 3600.0),
        ("d", 86400.0),
    ];
    let len = bytes.len();
    let mut total = 0.0;
    let mut i = 0;
    while i < len {
        let num_start = i;
        while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }
        if i == num_start {
            return None;
        }
        let n: f64 = s[num_start..i].parse().ok()?;
        let (unit, factor) = UNITS.iter().find(|(u, _)| s[i..].starts_with(u))?;
        total += n * factor;
        i += unit.len();
    }
    Some(total)
}
