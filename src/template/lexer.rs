//! Tokenizer for the template language.
//!
//! Lexing is mode-switching, as it must be for a template language: outside
//! `{{ }}` almost every byte is literal text, while inside it we want
//! identifiers, dots, pipes and quoted strings. A single context-free token set
//! cannot express that, so the lexer carries a [`Mode`] and flips it at the
//! placeholder delimiters.
//!
//! Every token carries a byte [`Span`] so errors can point at the offending
//! part of the template rather than describing it in prose.

use std::fmt;

/// A byte range within the template source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
    /// Span covering both, for errors that span several tokens.
    pub fn to(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    /// Literal text with escapes already resolved.
    Text(String),
    /// `[`
    OpenGroup,
    /// `]`
    CloseGroup,
    /// `{{`
    OpenExpr,
    /// `}}`
    CloseExpr,
    /// A bare word inside a placeholder.
    Ident(String),
    /// `.`
    Dot,
    /// `|`
    Pipe,
    /// A quoted literal inside a placeholder, contents unescaped.
    Str(String),
    Eof,
}

impl Tok {
    /// How to name this token in an error message.
    pub fn describe(&self) -> String {
        match self {
            Tok::Text(_) => "text".to_string(),
            Tok::OpenGroup => "`[`".to_string(),
            Tok::CloseGroup => "`]`".to_string(),
            Tok::OpenExpr => "`{{`".to_string(),
            Tok::CloseExpr => "`}}`".to_string(),
            Tok::Ident(s) => format!("`{s}`"),
            Tok::Dot => "`.`".to_string(),
            Tok::Pipe => "`|`".to_string(),
            Tok::Str(_) => "a quoted literal".to_string(),
            Tok::Eof => "end of template".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Outside a placeholder: everything is literal text.
    Text,
    /// Inside `{{ }}`: identifiers, dots, pipes, strings.
    Expr,
}

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn lex(input: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(input).run()
}

struct Lexer<'a> {
    src: &'a str,
    /// (byte offset, char) pairs, so spans stay byte-accurate while scanning
    /// is done per character — templates routinely contain non-ASCII.
    chars: Vec<(usize, char)>,
    pos: usize,
    mode: Mode,
    /// Byte offset of the `{{` currently open, for the unclosed-placeholder error.
    open_expr: usize,
    out: Vec<Token>,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer {
            src,
            chars: src.char_indices().collect(),
            pos: 0,
            mode: Mode::Text,
            open_expr: 0,
            out: Vec::new(),
        }
    }

    fn at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).map(|(_, c)| *c)
    }

    /// Byte offset of the cursor, clamped to the end of input.
    fn byte(&self) -> usize {
        self.chars
            .get(self.pos)
            .map(|(b, _)| *b)
            .unwrap_or(self.src.len())
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.at(0);
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn push(&mut self, tok: Tok, start: usize) {
        let span = Span::new(start, self.byte());
        self.out.push(Token { tok, span });
    }

    fn run(mut self) -> Result<Vec<Token>, LexError> {
        while self.pos < self.chars.len() {
            match self.mode {
                Mode::Text => self.lex_text()?,
                Mode::Expr => self.lex_expr()?,
            }
        }
        let end = self.src.len();
        // Reaching the end while still in Expr mode means the placeholder was
        // never closed; the scan loop alone would not catch this.
        if self.mode == Mode::Expr {
            return Err(LexError {
                message: "unclosed `{{` — every placeholder needs a matching `}}`".to_string(),
                span: Span::new(self.open_expr, end),
            });
        }
        self.out.push(Token {
            tok: Tok::Eof,
            span: Span::new(end, end),
        });
        Ok(self.out)
    }

    /// Accumulate literal text until a structural delimiter, resolving escapes.
    fn lex_text(&mut self) -> Result<(), LexError> {
        let start = self.byte();
        let mut buf = String::new();

        loop {
            match self.at(0) {
                None => break,
                Some('{') if self.at(1) == Some('{') => break,
                Some('[') | Some(']') => break,
                Some('\\') => {
                    let esc_start = self.byte();
                    self.bump();
                    match self.bump() {
                        Some(c @ ('[' | ']' | '{' | '}' | '\\')) => buf.push(c),
                        Some(other) => {
                            return Err(LexError {
                                message: format!(
                                    "unknown escape `\\{other}` — only \\[ \\] \\{{ \\}} \\\\ are escapes"
                                ),
                                span: Span::new(esc_start, self.byte()),
                            })
                        }
                        None => {
                            return Err(LexError {
                                message: "template ends with a trailing `\\`".to_string(),
                                span: Span::new(esc_start, self.byte()),
                            })
                        }
                    }
                }
                Some(c) => {
                    self.bump();
                    buf.push(c);
                }
            }
        }

        if !buf.is_empty() {
            self.push(Tok::Text(buf), start);
        }

        // Now emit whatever delimiter stopped us.
        let d = self.byte();
        match self.at(0) {
            Some('[') => {
                self.bump();
                self.push(Tok::OpenGroup, d);
            }
            Some(']') => {
                self.bump();
                self.push(Tok::CloseGroup, d);
            }
            Some('{') => {
                self.bump();
                self.bump();
                self.push(Tok::OpenExpr, d);
                self.mode = Mode::Expr;
                self.open_expr = d;
            }
            _ => {}
        }
        Ok(())
    }

    /// Lex one token inside a placeholder.
    fn lex_expr(&mut self) -> Result<(), LexError> {
        // Whitespace inside a placeholder is insignificant.
        while self.at(0).is_some_and(|c| c.is_whitespace()) {
            self.bump();
        }
        let start = self.byte();
        match self.at(0) {
            None => Err(LexError {
                message: "unclosed `{{` — every placeholder needs a matching `}}`".to_string(),
                span: Span::new(start, start),
            }),
            Some('}') if self.at(1) == Some('}') => {
                self.bump();
                self.bump();
                self.push(Tok::CloseExpr, start);
                self.mode = Mode::Text;
                Ok(())
            }
            Some('|') => {
                self.bump();
                self.push(Tok::Pipe, start);
                Ok(())
            }
            Some('.') => {
                self.bump();
                self.push(Tok::Dot, start);
                Ok(())
            }
            Some('"') => {
                self.bump();
                let mut buf = String::new();
                loop {
                    match self.bump() {
                        None => {
                            return Err(LexError {
                                message: "unterminated quoted literal".to_string(),
                                span: Span::new(start, self.byte()),
                            })
                        }
                        Some('"') => break,
                        Some('\\') => match self.bump() {
                            Some(c @ ('"' | '\\')) => buf.push(c),
                            Some(other) => {
                                return Err(LexError {
                                    message: format!(
                                        "unknown escape `\\{other}` in a quoted literal"
                                    ),
                                    span: Span::new(start, self.byte()),
                                })
                            }
                            None => {
                                return Err(LexError {
                                    message: "unterminated quoted literal".to_string(),
                                    span: Span::new(start, self.byte()),
                                })
                            }
                        },
                        Some(c) => buf.push(c),
                    }
                }
                self.push(Tok::Str(buf), start);
                Ok(())
            }
            Some(c) if c.is_alphanumeric() || c == '_' => {
                let mut buf = String::new();
                while let Some(c) = self.at(0) {
                    if c.is_alphanumeric() || c == '_' {
                        buf.push(c);
                        self.bump();
                    } else {
                        break;
                    }
                }
                self.push(Tok::Ident(buf), start);
                Ok(())
            }
            Some('{') if self.at(1) == Some('{') => Err(LexError {
                message: "`{{` inside a placeholder — the previous one is not closed".to_string(),
                span: Span::new(start, start + 2),
            }),
            Some(c) => {
                self.bump();
                Err(LexError {
                    message: format!("unexpected `{c}` inside a placeholder"),
                    span: Span::new(start, self.byte()),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        lex(s).expect("lexes").into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn splits_text_and_placeholders() {
        assert_eq!(
            toks("a{{b}}c"),
            vec![
                Tok::Text("a".into()),
                Tok::OpenExpr,
                Tok::Ident("b".into()),
                Tok::CloseExpr,
                Tok::Text("c".into()),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn lexes_dotted_paths_pipes_and_literals() {
        assert_eq!(
            toks("{{a.b|\"x\"}}"),
            vec![
                Tok::OpenExpr,
                Tok::Ident("a".into()),
                Tok::Dot,
                Tok::Ident("b".into()),
                Tok::Pipe,
                Tok::Str("x".into()),
                Tok::CloseExpr,
                Tok::Eof
            ]
        );
    }

    #[test]
    fn lexes_group_delimiters() {
        assert_eq!(
            toks("[x]"),
            vec![
                Tok::OpenGroup,
                Tok::Text("x".into()),
                Tok::CloseGroup,
                Tok::Eof
            ]
        );
    }

    #[test]
    fn whitespace_inside_a_placeholder_is_insignificant() {
        assert_eq!(toks("{{ a . b }}"), toks("{{a.b}}"));
    }

    #[test]
    fn resolves_escapes_in_text() {
        assert_eq!(toks(r"\[a\]"), vec![Tok::Text("[a]".into()), Tok::Eof]);
        assert_eq!(toks(r"\\"), vec![Tok::Text(r"\".into()), Tok::Eof]);
        assert_eq!(toks(r"\{\}"), vec![Tok::Text("{}".into()), Tok::Eof]);
    }

    #[test]
    fn a_lone_brace_is_literal_text() {
        assert_eq!(toks("{x}"), vec![Tok::Text("{x}".into()), Tok::Eof]);
    }

    #[test]
    fn spans_are_byte_accurate_through_non_ascii() {
        // `é` is two bytes, so a naive char-count span would be wrong.
        let tokens = lex("é{{a}}").expect("lexes");
        let expr = tokens.iter().find(|t| t.tok == Tok::OpenExpr).unwrap();
        assert_eq!(expr.span.start, 2);
    }

    #[test]
    fn rejects_bad_escapes_and_unterminated_strings() {
        assert!(lex(r"\q").is_err());
        assert!(lex("x\\").is_err());
        assert!(lex("{{a|\"oops}}").is_err());
        assert!(lex("{{a").is_err());
        assert!(lex("{{a b!}}").is_err());
    }
}
