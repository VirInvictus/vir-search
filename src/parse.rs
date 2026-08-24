use crate::ast::{
    Comparator, DateSpec, Expr, MatchKind, ParseField, ParseSort, ParseState, SortSpec, Value,
};
use crate::lex::{Token, lex};

pub struct ParseResult<F, S, K> {
    pub expr: Expr<F, S>,
    pub sorts: Vec<SortSpec<K>>,
    pub warnings: Vec<String>,
}

type PResult<F, S> = Result<Expr<F, S>, ()>;

pub trait PerspectiveResolver<F, S> {
    fn expression(&self, name: &str) -> Option<String>;
}

impl<F, S> PerspectiveResolver<F, S> for () {
    fn expression(&self, _name: &str) -> Option<String> {
        None
    }
}

pub fn parse<F: ParseField, S: ParseState, K: ParseSort>(input: &str) -> ParseResult<F, S, K> {
    parse_inner::<F, S, K, ()>(input, None, &[])
}

pub fn parse_with_resolver<
    F: ParseField,
    S: ParseState,
    K: ParseSort,
    R: PerspectiveResolver<F, S>,
>(
    input: &str,
    resolver: &R,
) -> ParseResult<F, S, K> {
    parse_inner(input, Some(resolver), &[])
}

fn parse_inner<F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>>(
    input: &str,
    resolver: Option<&R>,
    seen: &[String],
) -> ParseResult<F, S, K> {
    let mut p = Parser {
        tokens: lex(input),
        pos: 0,
        sorts: Vec::new(),
        warnings: Vec::new(),
        resolver,
        seen: seen.to_vec(),
        _marker: std::marker::PhantomData,
    };
    let expr = p.boolean_expr().unwrap_or_else(|_| Expr::Empty);
    ParseResult {
        expr,
        sorts: p.sorts,
        warnings: p.warnings,
    }
}

struct Parser<'a, F, S, K, R> {
    tokens: Vec<Token>,
    pos: usize,
    sorts: Vec<SortSpec<K>>,
    warnings: Vec<String>,
    resolver: Option<&'a R>,
    seen: Vec<String>,
    _marker: std::marker::PhantomData<(F, S)>,
}

impl<'a, F: ParseField, S: ParseState, K: ParseSort, R: PerspectiveResolver<F, S>>
    Parser<'a, F, S, K, R>
{
    fn peek(&mut self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned()?;
        self.pos += 1;
        Some(t)
    }

    fn boolean_expr(&mut self) -> PResult<F, S> {
        let mut items = vec![self.boolean_term()?];
        while let Some(Token::Word(w)) = self.peek() {
            if w.eq_ignore_ascii_case("or") {
                self.pos += 1;
                items.push(self.boolean_term()?);
            } else {
                break;
            }
        }
        if items.len() == 1 {
            Ok(items.pop().unwrap())
        } else {
            items.retain(|e| !matches!(e, Expr::Empty));
            if items.is_empty() {
                Ok(Expr::Empty)
            } else if items.len() == 1 {
                Ok(items.pop().unwrap())
            } else {
                Ok(Expr::Or(items))
            }
        }
    }

    fn boolean_term(&mut self) -> PResult<F, S> {
        let mut items = vec![self.boolean_factor()?];
        while let Some(t) = self.peek() {
            if t == &Token::RParen {
                break;
            }
            if let Token::Word(w) = t {
                if w.eq_ignore_ascii_case("or") {
                    break;
                }
                if w.eq_ignore_ascii_case("and") {
                    self.pos += 1;
                    items.push(self.boolean_factor()?);
                    continue;
                }
            }
            items.push(self.boolean_factor()?);
        }
        if items.len() == 1 {
            Ok(items.pop().unwrap())
        } else {
            items.retain(|e| !matches!(e, Expr::Empty));
            if items.is_empty() {
                Ok(Expr::Empty)
            } else if items.len() == 1 {
                Ok(items.pop().unwrap())
            } else {
                Ok(Expr::And(items))
            }
        }
    }

    fn boolean_factor(&mut self) -> PResult<F, S> {
        if let Some(Token::Bang) = self.peek() {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.predicate()?)));
        }
        if let Some(Token::Word(w)) = self.peek() {
            if w.eq_ignore_ascii_case("not") {
                self.pos += 1;
                return Ok(Expr::Not(Box::new(self.predicate()?)));
            }
        }
        self.predicate()
    }

    fn predicate(&mut self) -> PResult<F, S> {
        let t = self.advance().ok_or(())?;
        match t {
            Token::LParen => {
                let inner = self.boolean_expr()?;
                if let Some(Token::RParen) = self.peek() {
                    self.pos += 1;
                    Ok(inner)
                } else {
                    self.warnings.push("unclosed parenthesis".into());
                    Ok(text_or_empty(format!("({inner}")))
                }
            }
            Token::Word(w) => {
                let w_clone = w.clone();
                if let Some(Token::Colon) = self.peek() {
                    self.pos += 1;
                    let lname = w_clone.to_ascii_lowercase();
                    match lname.as_str() {
                        "is" => {
                            let val = self.value_string().ok_or(())?;
                            match S::parse(&val.to_ascii_lowercase()) {
                                Some(s) => Ok(Expr::State(s)),
                                None => {
                                    self.warnings
                                        .push(format!("unknown state {val:?}; matching as text"));
                                    Ok(Expr::Text(format!("is:{val}")))
                                }
                            }
                        }
                        "sort" => {
                            let val = self.value_string().ok_or(())?;
                            let (key_str, desc) = if let Some(stripped) = val.strip_prefix('-') {
                                (stripped.to_string(), true)
                            } else if let Some(stripped) = val.strip_prefix('+') {
                                (stripped.to_string(), false)
                            } else {
                                (val.clone(), false)
                            };
                            self.sort_spec(key_str, desc)
                        }
                        "vl" => {
                            let val = self.value_string().ok_or(())?;
                            self.resolve_vl(&val)
                        }
                        _ => match F::parse(&lname) {
                            Some(field) => {
                                let ft = field.field_type();
                                if ft == crate::ast::FieldType::Int
                                    || ft == crate::ast::FieldType::Real
                                    || ft == crate::ast::FieldType::Date
                                {
                                    self.relational(field)
                                } else {
                                    self.text_match(field)
                                }
                            }
                            None => {
                                let remainder = self.value_string().unwrap_or_default();
                                self.warnings
                                    .push(format!("unknown field {w_clone:?}; matching as text"));
                                Ok(text_or_empty(format!("{w_clone}:{remainder}")))
                            }
                        },
                    }
                } else {
                    Ok(text_or_empty(w_clone))
                }
            }
            Token::Quoted(q) => Ok(text_or_empty(q.clone())),
            Token::Colon
            | Token::RParen
            | Token::Eq
            | Token::Ne
            | Token::Lt
            | Token::Le
            | Token::Gt
            | Token::Ge
            | Token::Tilde
            | Token::Quest
            | Token::DotDot
            | Token::Bang => {
                let mut fallback = String::new();
                if let Token::Colon = &t {
                    fallback.push(':');
                }
                if let Token::Eq = &t {
                    fallback.push('=');
                }
                if let Token::Tilde = &t {
                    fallback.push('~');
                }
                Ok(text_or_empty(fallback))
            }
        }
    }

    fn sort_spec(&mut self, key_str: String, descending: bool) -> PResult<F, S> {
        match K::parse(&key_str.to_ascii_lowercase()) {
            Some(k) => {
                self.sorts.push(SortSpec { key: k, descending });
                Ok(Expr::Empty)
            }
            None => {
                self.warnings.push(format!("unknown sort key {key_str:?}"));
                let pfx = if descending { "-" } else { "" };
                Ok(Expr::Text(format!("sort:{pfx}{key_str}")))
            }
        }
    }

    fn text_match(&mut self, field: F) -> PResult<F, S> {
        let pk = self.peek();
        match pk {
            Some(Token::Eq) => {
                self.pos += 1;
                let val = self.value_string().ok_or(())?;
                Ok(Expr::Field {
                    field,
                    kind: MatchKind::Exact(val),
                })
            }
            Some(Token::Tilde) => {
                self.pos += 1;
                let val = self.value_string().ok_or(())?;
                Ok(Expr::Field {
                    field,
                    kind: MatchKind::Regex(val),
                })
            }
            Some(Token::Quest) => {
                self.pos += 1;
                let val = self.value_string().ok_or(())?;
                Ok(Expr::Field {
                    field,
                    kind: MatchKind::Fuzzy(val),
                })
            }
            _ => {
                let val = self.value_string().ok_or(())?;
                if let Some(b) = bool_word(&val) {
                    Ok(Expr::Field {
                        field,
                        kind: if b {
                            MatchKind::HasAny
                        } else {
                            MatchKind::HasNone
                        },
                    })
                } else {
                    Ok(Expr::Field {
                        field,
                        kind: MatchKind::Substring(val),
                    })
                }
            }
        }
    }

    fn relational(&mut self, field: F) -> PResult<F, S> {
        let comp = self.eat_comparator();
        let raw = self.value_string().ok_or(())?;
        // Handle :true/:false presence checks even on numeric/date fields
        if comp.is_none() {
            if let Some(b) = bool_word(&raw) {
                return Ok(Expr::Field {
                    field,
                    kind: if b {
                        MatchKind::HasAny
                    } else {
                        MatchKind::HasNone
                    },
                });
            }
        }
        let Some(low) = parse_typed_value(field.clone(), &raw) else {
            self.warnings
                .push(format!("bad numeric/date value {raw:?}; matching as text"));
            let prefix = comp.map(|c| c.as_str()).unwrap_or("");
            return Ok(Expr::Text(format!("{field}:{prefix}{raw}")));
        };
        if comp.is_none() && matches!(self.peek(), Some(Token::DotDot)) {
            self.pos += 1;
            let raw_hi = self.value_string().ok_or(())?;
            if let Some(high) = parse_typed_value(field.clone(), &raw_hi) {
                return Ok(Expr::Range { field, low, high });
            }
            self.warnings
                .push(format!("bad range bound {raw_hi:?}; matching as text"));
            return Ok(Expr::Text(format!("{field}:{raw}..{raw_hi}")));
        }
        Ok(Expr::Compare {
            field,
            comp: comp.unwrap_or(Comparator::Eq),
            value: low,
        })
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

    fn resolve_vl(&mut self, name: &str) -> PResult<F, S> {
        let key = name.to_ascii_lowercase();
        let Some(resolver) = self.resolver else {
            return Ok(Expr::Text(format!("vl:{name}")));
        };
        if self.seen.iter().any(|s| s == &key) {
            self.warnings
                .push(format!("perspective cycle at {name:?}; ignored"));
            return Ok(Expr::Empty);
        }
        let Some(text) = resolver.expression(name) else {
            self.warnings
                .push(format!("unknown perspective {name:?}; ignored"));
            return Ok(Expr::Empty);
        };
        let mut seen = self.seen.clone();
        seen.push(key);
        let sub = parse_inner(&text, self.resolver, &seen);
        self.warnings.extend(sub.warnings);
        self.sorts.extend(sub.sorts);
        Ok(sub.expr)
    }
}

fn bool_word(w: &str) -> Option<bool> {
    match w.to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
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
        raw.parse::<f64>().ok().map(Value::Real)
    } else {
        raw.parse::<i64>().ok().map(Value::Int)
    }
}

fn parse_date_spec(raw: &str) -> Option<DateSpec> {
    let s = raw.to_ascii_lowercase();
    match s.as_str() {
        "today" => return Some(DateSpec::Today),
        "yesterday" => return Some(DateSpec::Yesterday),
        "thisweek" => return Some(DateSpec::ThisWeek),
        "thismonth" => return Some(DateSpec::ThisMonth),
        "thisyear" => return Some(DateSpec::ThisYear),
        _ => {}
    }
    if let Some(n) = s.strip_suffix("daysago") {
        return n.parse::<u32>().ok().map(DateSpec::DaysAgo);
    }
    let mut parts = s.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
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
