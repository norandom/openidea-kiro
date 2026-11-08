//! Guard and effect language lexer and parser (design component `GuardLanguage`,
//! requirements 4.1, 4.4, 4.8).
//!
//! Provides parsing of `_Guard:_` and `_Effect:_` expressions with operator
//! precedence, nesting depth cap, and byte positions for errors.

use std::collections::BTreeSet;

/// Explicit recursion cap on parenthesised / negated sub-expressions for the wasm stack.
pub const MAX_GUARD_DEPTH: usize = 64;

/// Comparison operators allowed in guards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    /// `=` or `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

impl Op {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Op::Eq => "=",
            Op::Ne => "!=",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
        }
    }
}

/// A literal value on the right-hand side of a comparison or effect.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Literal {
    Int(i64),
    Bool(bool),
    /// Quoted or bare word.
    Word(String),
}

/// A parsed guard expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    True,
    Term(String),
    Compare {
        term: String,
        op: Op,
        value: Literal,
    },
    Not(Box<Expr>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
}

impl Expr {
    /// Returns the set of all term names referenced by this expression.
    #[must_use]
    pub fn terms(&self) -> BTreeSet<String> {
        let mut set = BTreeSet::new();
        self.collect_terms(&mut set);
        set
    }

    fn collect_terms(&self, set: &mut BTreeSet<String>) {
        match self {
            Expr::True => {}
            Expr::Term(t) => {
                set.insert(t.clone());
            }
            Expr::Compare { term, .. } => {
                set.insert(term.clone());
            }
            Expr::Not(inner) => {
                inner.collect_terms(set);
            }
            Expr::And(items) | Expr::Or(items) => {
                for item in items {
                    item.collect_terms(set);
                }
            }
        }
    }

    /// Returns all comparisons in document / evaluation order.
    #[must_use]
    pub fn comparisons(&self) -> Vec<(&str, Op, &Literal)> {
        let mut list = Vec::new();
        self.collect_comparisons(&mut list);
        list
    }

    fn collect_comparisons<'a>(&'a self, list: &mut Vec<(&'a str, Op, &'a Literal)>) {
        match self {
            Expr::True | Expr::Term(_) => {}
            Expr::Compare { term, op, value } => {
                list.push((term.as_str(), *op, value));
            }
            Expr::Not(inner) => {
                inner.collect_comparisons(list);
            }
            Expr::And(items) | Expr::Or(items) => {
                for item in items {
                    item.collect_comparisons(list);
                }
            }
        }
    }
}

/// A parsing error with message and byte position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

/// An assignment in an effect clause (`term = literal`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub term: String,
    pub value: Literal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Or,
    And,
    Not,
    True,
    False,
    OpenParen,
    CloseParen,
    Comma,
    Op(Op, &'static str), // Op and the raw spelling ("=" or "==" etc.)
    Int(i64),
    Word(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

fn tokenize(text: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut idx = 0;

    while idx < text.len() {
        let Some(rem) = text.get(idx..) else {
            break;
        };
        let Some(c) = rem.chars().next() else {
            break;
        };

        if c.is_ascii_whitespace() {
            idx += c.len_utf8();
            continue;
        }

        if c == '(' {
            tokens.push(Token {
                kind: TokenKind::OpenParen,
                start: idx,
                end: idx + 1,
            });
            idx += 1;
            continue;
        }

        if c == ')' {
            tokens.push(Token {
                kind: TokenKind::CloseParen,
                start: idx,
                end: idx + 1,
            });
            idx += 1;
            continue;
        }

        if c == ',' {
            tokens.push(Token {
                kind: TokenKind::Comma,
                start: idx,
                end: idx + 1,
            });
            idx += 1;
            continue;
        }

        // Two-character operators
        if rem.starts_with("==") {
            tokens.push(Token {
                kind: TokenKind::Op(Op::Eq, "=="),
                start: idx,
                end: idx + 2,
            });
            idx += 2;
            continue;
        }
        if rem.starts_with("!=") {
            tokens.push(Token {
                kind: TokenKind::Op(Op::Ne, "!="),
                start: idx,
                end: idx + 2,
            });
            idx += 2;
            continue;
        }
        if rem.starts_with("<=") {
            tokens.push(Token {
                kind: TokenKind::Op(Op::Le, "<="),
                start: idx,
                end: idx + 2,
            });
            idx += 2;
            continue;
        }
        if rem.starts_with(">=") {
            tokens.push(Token {
                kind: TokenKind::Op(Op::Ge, ">="),
                start: idx,
                end: idx + 2,
            });
            idx += 2;
            continue;
        }

        // Single-character operators
        if rem.starts_with('=') {
            tokens.push(Token {
                kind: TokenKind::Op(Op::Eq, "="),
                start: idx,
                end: idx + 1,
            });
            idx += 1;
            continue;
        }
        if rem.starts_with('<') {
            tokens.push(Token {
                kind: TokenKind::Op(Op::Lt, "<"),
                start: idx,
                end: idx + 1,
            });
            idx += 1;
            continue;
        }
        if rem.starts_with('>') {
            tokens.push(Token {
                kind: TokenKind::Op(Op::Gt, ">"),
                start: idx,
                end: idx + 1,
            });
            idx += 1;
            continue;
        }

        // Quoted strings: '...' or "..."
        if c == '"' || c == '\'' {
            let quote = c;
            let start = idx;
            let mut search_idx = idx + 1;
            let mut found_end = None;
            while search_idx < text.len() {
                let Some(ch) = text.get(search_idx..).and_then(|s| s.chars().next()) else {
                    break;
                };
                if ch == quote {
                    found_end = Some(search_idx);
                    break;
                }
                search_idx += ch.len_utf8();
            }
            let Some(end_quote) = found_end else {
                return Err(ParseError {
                    message: "unterminated string literal".to_string(),
                    position: start,
                });
            };
            let content = text.get(start + 1..end_quote).unwrap_or("").to_string();
            tokens.push(Token {
                kind: TokenKind::Word(content),
                start,
                end: end_quote + 1,
            });
            idx = end_quote + 1;
            continue;
        }

        // Numbers: negative numbers `-<digit>` or positive `<digit>`
        let is_negative_num = c == '-'
            && rem
                .get(1..)
                .and_then(|s| s.chars().next())
                .is_some_and(|ch| ch.is_ascii_digit());
        if c.is_ascii_digit() || is_negative_num {
            let start = idx;
            let mut end = idx + c.len_utf8();
            while end < text.len() {
                let Some(ch) = text.get(end..).and_then(|s| s.chars().next()) else {
                    break;
                };
                if ch.is_ascii_digit() {
                    end += ch.len_utf8();
                } else {
                    break;
                }
            }
            let slice = text.get(start..end).unwrap_or("0");
            let num = slice.parse::<i64>().map_err(|_| ParseError {
                message: "integer literal out of range".to_string(),
                position: start,
            })?;
            tokens.push(Token {
                kind: TokenKind::Int(num),
                start,
                end,
            });
            idx = end;
            continue;
        }

        // Words: [A-Za-z_][A-Za-z0-9_.-]*
        if c.is_ascii_alphabetic() || c == '_' {
            let start = idx;
            let mut end = idx + c.len_utf8();
            while end < text.len() {
                let Some(ch) = text.get(end..).and_then(|s| s.chars().next()) else {
                    break;
                };
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
                    end += ch.len_utf8();
                } else {
                    break;
                }
            }
            let word = text.get(start..end).unwrap_or("");
            let kind = if word.eq_ignore_ascii_case("or") {
                TokenKind::Or
            } else if word.eq_ignore_ascii_case("and") {
                TokenKind::And
            } else if word.eq_ignore_ascii_case("not") {
                TokenKind::Not
            } else if word.eq_ignore_ascii_case("true") {
                TokenKind::True
            } else if word.eq_ignore_ascii_case("false") {
                TokenKind::False
            } else {
                TokenKind::Word(word.to_string())
            };
            tokens.push(Token { kind, start, end });
            idx = end;
            continue;
        }

        return Err(ParseError {
            message: format!("unexpected character `{c}`"),
            position: idx,
        });
    }

    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    text_len: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], text_len: usize) -> Self {
        Parser {
            tokens,
            pos: 0,
            text_len,
            depth: 0,
        }
    }

    fn current_pos(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map(|t| t.start)
            .unwrap_or(self.text_len)
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&'a Token> {
        let t = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(t)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut items = vec![self.parse_and()?];
        while let Some(t) = self.peek() {
            if t.kind == TokenKind::Or {
                self.next();
                items.push(self.parse_and()?);
            } else {
                break;
            }
        }
        if items.len() == 1 {
            items.pop().ok_or_else(|| ParseError {
                message: "guard ended unexpectedly".to_string(),
                position: self.current_pos(),
            })
        } else {
            Ok(Expr::Or(items))
        }
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut items = vec![self.parse_not()?];
        while let Some(t) = self.peek() {
            if t.kind == TokenKind::And {
                self.next();
                items.push(self.parse_not()?);
            } else {
                break;
            }
        }
        if items.len() == 1 {
            items.pop().ok_or_else(|| ParseError {
                message: "guard ended unexpectedly".to_string(),
                position: self.current_pos(),
            })
        } else {
            Ok(Expr::And(items))
        }
    }

    fn parse_not(&mut self) -> Result<Expr, ParseError> {
        if let Some(t) = self.peek()
            && t.kind == TokenKind::Not
        {
            let pos = t.start;
            self.next();
            self.depth += 1;
            if self.depth > MAX_GUARD_DEPTH {
                return Err(ParseError {
                    message: format!("guard nesting depth exceeds maximum of {MAX_GUARD_DEPTH}"),
                    position: pos,
                });
            }
            let inner = self.parse_not()?;
            self.depth = self.depth.saturating_sub(1);
            return Ok(Expr::Not(Box::new(inner)));
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        let Some(token) = self.peek().cloned() else {
            return Err(ParseError {
                message: "expected a term, a literal, or `(`".to_string(),
                position: self.current_pos(),
            });
        };

        match token.kind {
            TokenKind::OpenParen => {
                let pos = token.start;
                self.next(); // consume '('
                self.depth += 1;
                if self.depth > MAX_GUARD_DEPTH {
                    return Err(ParseError {
                        message: format!(
                            "guard nesting depth exceeds maximum of {MAX_GUARD_DEPTH}"
                        ),
                        position: pos,
                    });
                }
                if let Some(next_t) = self.peek()
                    && next_t.kind == TokenKind::CloseParen
                {
                    return Err(ParseError {
                        message: "guard ended unexpectedly".to_string(),
                        position: next_t.start,
                    });
                }
                let inner = self.parse_or()?;
                self.depth = self.depth.saturating_sub(1);
                let Some(close_t) = self.next() else {
                    return Err(ParseError {
                        message: "expected `)`".to_string(),
                        position: self.text_len,
                    });
                };
                if close_t.kind != TokenKind::CloseParen {
                    return Err(ParseError {
                        message: "expected `)`".to_string(),
                        position: close_t.start,
                    });
                }
                Ok(inner)
            }
            TokenKind::Op(_, raw) => Err(ParseError {
                message: format!("`{raw}` needs a term on its left"),
                position: token.start,
            }),
            TokenKind::True => {
                self.next();
                Ok(Expr::True)
            }
            TokenKind::False => {
                self.next();
                Ok(Expr::Not(Box::new(Expr::True)))
            }
            TokenKind::Word(term_name) => {
                self.next(); // consume term name
                if let Some(op_token) = self.peek().cloned()
                    && let TokenKind::Op(op, _) = op_token.kind
                {
                    self.next(); // consume operator
                    let val = self.parse_literal()?;
                    Ok(Expr::Compare {
                        term: term_name,
                        op,
                        value: val,
                    })
                } else {
                    Ok(Expr::Term(term_name))
                }
            }
            TokenKind::Int(_) | TokenKind::Comma | TokenKind::CloseParen => Err(ParseError {
                message: "expected a term, a literal, or `(`".to_string(),
                position: token.start,
            }),
            TokenKind::Or | TokenKind::And | TokenKind::Not => Err(ParseError {
                message: "expected a term, a literal, or `(`".to_string(),
                position: token.start,
            }),
        }
    }

    fn parse_literal(&mut self) -> Result<Literal, ParseError> {
        let Some(token) = self.next() else {
            return Err(ParseError {
                message: "expected a literal after the operator".to_string(),
                position: self.text_len,
            });
        };
        match &token.kind {
            TokenKind::Int(i) => Ok(Literal::Int(*i)),
            TokenKind::True => Ok(Literal::Bool(true)),
            TokenKind::False => Ok(Literal::Bool(false)),
            TokenKind::Word(w) => Ok(Literal::Word(w.clone())),
            _ => Err(ParseError {
                message: "expected a literal after the operator".to_string(),
                position: token.start,
            }),
        }
    }
}

/// Parses a `_Guard:_` annotation value into an expression tree.
///
/// Empty or whitespace-only text parses as `Ok(Expr::True)`.
///
/// # Errors
/// Returns `ParseError` with leading phrase and byte position if syntax is invalid.
pub fn parse_guard(text: &str) -> Result<Expr, ParseError> {
    if text.trim().is_empty() {
        return Ok(Expr::True);
    }
    let tokens = tokenize(text)?;
    let mut parser = Parser::new(&tokens, text.len());
    let expr = parser.parse_or()?;
    if let Some(trailing) = parser.peek() {
        return Err(ParseError {
            message: "unexpected trailing input".to_string(),
            position: trailing.start,
        });
    }
    Ok(expr)
}

/// Parses an `_Effect:_` annotation value into a list of assignments.
///
/// Format: `term = literal (, term = literal)*` where `=` or `==` is allowed.
///
/// # Errors
/// Returns `ParseError` if text is empty or does not conform to the assignment grammar.
pub fn parse_effect(text: &str) -> Result<Vec<Assignment>, ParseError> {
    if text.trim().is_empty() {
        return Err(ParseError {
            message: "effect needs `term = literal`".to_string(),
            position: 0,
        });
    }
    let tokens = tokenize(text)?;
    let mut parser = Parser::new(&tokens, text.len());
    let mut assignments = Vec::new();

    loop {
        let Some(term_token) = parser.next() else {
            return Err(ParseError {
                message: "effect needs `term = literal`".to_string(),
                position: parser.text_len,
            });
        };
        let term_name = match &term_token.kind {
            TokenKind::Word(w) => w.clone(),
            _ => {
                return Err(ParseError {
                    message: "effect needs `term = literal`".to_string(),
                    position: term_token.start,
                });
            }
        };

        let Some(op_token) = parser.next() else {
            return Err(ParseError {
                message: "effect needs `term = literal`".to_string(),
                position: parser.text_len,
            });
        };
        match op_token.kind {
            TokenKind::Op(Op::Eq, _) => {}
            _ => {
                return Err(ParseError {
                    message: "effect needs `term = literal`".to_string(),
                    position: op_token.start,
                });
            }
        }

        let val = parser.parse_literal()?;
        assignments.push(Assignment {
            term: term_name,
            value: val,
        });

        if let Some(t) = parser.peek() {
            if t.kind == TokenKind::Comma {
                parser.next(); // consume ','
                if parser.peek().is_none() {
                    return Err(ParseError {
                        message: "effect needs `term = literal`".to_string(),
                        position: parser.text_len,
                    });
                }
            } else {
                return Err(ParseError {
                    message: "unexpected trailing input".to_string(),
                    position: t.start,
                });
            }
        } else {
            break;
        }
    }

    Ok(assignments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_guard_parses_to_true() {
        assert_eq!(
            parse_guard("").unwrap_or(Expr::Term("fail".into())),
            Expr::True
        );
        assert_eq!(
            parse_guard("   ").unwrap_or(Expr::Term("fail".into())),
            Expr::True
        );
    }

    #[test]
    fn precedence_or_and_not() {
        let res = parse_guard("a or b and not c");
        assert_eq!(
            res,
            Ok(Expr::Or(vec![
                Expr::Term("a".into()),
                Expr::And(vec![
                    Expr::Term("b".into()),
                    Expr::Not(Box::new(Expr::Term("c".into()))),
                ]),
            ]))
        );
    }

    #[test]
    fn parentheses_override_precedence() {
        let res = parse_guard("(a or b) and c");
        assert_eq!(
            res,
            Ok(Expr::And(vec![
                Expr::Or(vec![Expr::Term("a".into()), Expr::Term("b".into())]),
                Expr::Term("c".into()),
            ]))
        );
    }

    #[test]
    fn both_equality_spellings() {
        let r1 = parse_guard("mode = safe");
        let r2 = parse_guard("mode == safe");
        assert_eq!(
            r1,
            Ok(Expr::Compare {
                term: "mode".into(),
                op: Op::Eq,
                value: Literal::Word("safe".into()),
            })
        );
        assert_eq!(r1, r2);
    }

    #[test]
    fn negative_integers_and_operators() {
        let res = parse_guard("depth >= -500 and depth < 1000");
        assert_eq!(
            res,
            Ok(Expr::And(vec![
                Expr::Compare {
                    term: "depth".into(),
                    op: Op::Ge,
                    value: Literal::Int(-500),
                },
                Expr::Compare {
                    term: "depth".into(),
                    op: Op::Lt,
                    value: Literal::Int(1000),
                },
            ]))
        );
    }

    #[test]
    fn quoted_and_bare_words() {
        let r1 = parse_guard("mode = 'safe'");
        let r2 = parse_guard("mode = \"safe\"");
        let r3 = parse_guard("mode = safe");
        assert_eq!(
            r1,
            Ok(Expr::Compare {
                term: "mode".into(),
                op: Op::Eq,
                value: Literal::Word("safe".into()),
            })
        );
        assert_eq!(r1, r2);
        assert_eq!(r1, r3);
    }

    #[test]
    fn effect_lists() {
        let res = parse_effect("armed = true, mode = safe, depth == -10");
        assert_eq!(
            res,
            Ok(vec![
                Assignment {
                    term: "armed".into(),
                    value: Literal::Bool(true),
                },
                Assignment {
                    term: "mode".into(),
                    value: Literal::Word("safe".into()),
                },
                Assignment {
                    term: "depth".into(),
                    value: Literal::Int(-10),
                },
            ])
        );
    }

    #[test]
    fn empty_effect_fails() {
        assert_eq!(
            parse_effect(""),
            Err(ParseError {
                message: "effect needs `term = literal`".into(),
                position: 0,
            })
        );
        assert_eq!(
            parse_effect("   "),
            Err(ParseError {
                message: "effect needs `term = literal`".into(),
                position: 0,
            })
        );
    }

    #[test]
    fn nesting_depth_above_64_fails() {
        let open_parens = "(".repeat(65);
        let close_parens = ")".repeat(65);
        let s = format!("{open_parens}armed{close_parens}");
        let res = parse_guard(&s);
        assert!(res.is_err_and(|e| {
            e.message
                .contains("guard nesting depth exceeds maximum of 64")
        }));
    }

    #[test]
    fn error_messages_and_positions() {
        // unexpected character
        assert_eq!(
            parse_guard("mode = @safe"),
            Err(ParseError {
                message: "unexpected character `@`".into(),
                position: 7,
            })
        );

        // unterminated string literal
        assert_eq!(
            parse_guard("mode = 'safe"),
            Err(ParseError {
                message: "unterminated string literal".into(),
                position: 7,
            })
        );

        // expected `)`
        assert_eq!(
            parse_guard("(mode = safe"),
            Err(ParseError {
                message: "expected `)`".into(),
                position: 12,
            })
        );

        // expected a term, a literal, or `(`
        assert_eq!(
            parse_guard("mode = safe and"),
            Err(ParseError {
                message: "expected a term, a literal, or `(`".into(),
                position: 15,
            })
        );

        // expected a literal after the operator
        assert_eq!(
            parse_guard("mode ="),
            Err(ParseError {
                message: "expected a literal after the operator".into(),
                position: 6,
            })
        );

        // `<op>` needs a term on its left
        assert_eq!(
            parse_guard("= safe"),
            Err(ParseError {
                message: "`=` needs a term on its left".into(),
                position: 0,
            })
        );
        assert_eq!(
            parse_guard("== safe"),
            Err(ParseError {
                message: "`==` needs a term on its left".into(),
                position: 0,
            })
        );

        // unexpected trailing input
        assert_eq!(
            parse_guard("mode = safe extra"),
            Err(ParseError {
                message: "unexpected trailing input".into(),
                position: 12,
            })
        );

        // guard ended unexpectedly
        assert_eq!(
            parse_guard("()"),
            Err(ParseError {
                message: "guard ended unexpectedly".into(),
                position: 1,
            })
        );
    }

    #[test]
    fn expr_terms_and_comparisons() {
        let e = parse_guard("mode = safe and (depth > 100 or not armed)");
        let expr = e.unwrap_or(Expr::True);
        let terms = expr.terms();
        assert!(terms.contains("mode"));
        assert!(terms.contains("depth"));
        assert!(terms.contains("armed"));
        assert_eq!(terms.len(), 3);

        let comps = expr.comparisons();
        assert_eq!(comps.len(), 2);
        assert_eq!(comps.first().map(|c| c.0), Some("mode"));
        assert_eq!(comps.first().map(|c| c.1), Some(Op::Eq));
        assert_eq!(comps.get(1).map(|c| c.0), Some("depth"));
        assert_eq!(comps.get(1).map(|c| c.1), Some(Op::Gt));
    }
}
