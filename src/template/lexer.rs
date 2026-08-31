//! Tokenizer for the template language, built on `logos`.
//!
//! Lexing is mode-switching, as it must be for a template language: outside
//! `{{ }}` almost every byte is literal text, while inside it we want
//! identifiers, dots, pipes and quoted strings. A single token set cannot
//! express that, so there are two — [`TextTok`] and [`ExprTok`] — and the
//! driver flips between them with `Lexer::morph` at the placeholder
//! delimiters. This is logos' documented context-dependent lexing pattern.
//!
//! logos yields one token per match, but the parser wants literal text as a
//! single string with escapes already resolved, so [`lex`] coalesces runs of
//! text, escapes and lone braces as it goes.
//!
//! Every token carries a byte [`Span`] so errors can point at the offending
//! part of the template rather than describing it in prose.

use logos::Logos;
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

impl From<std::ops::Range<usize>> for Span {
    fn from(r: std::ops::Range<usize>) -> Self {
        Span::new(r.start, r.end)
    }
}

/// The token kinds the parser consumes.
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

/// Outside a placeholder: literal text and the structural delimiters.
#[derive(Logos, Debug, PartialEq)]
enum TextTok {
    #[token("[")]
    OpenGroup,
    #[token("]")]
    CloseGroup,
    #[token("{{")]
    OpenExpr,
    /// A recognized escape, carrying the character it stands for.
    #[regex(r"\\[\[\]{}\\]", |lex| lex.slice().chars().nth(1))]
    Escape(char),
    /// A backslash that did not form a valid escape. Longest-match means the
    /// two-character `Escape` wins whenever it applies, so reaching this token
    /// is always an error — it exists only to carry a precise message.
    #[token("\\")]
    BadEscape,
    /// A run of ordinary text. `{` is excluded so `{{` can win by longest
    /// match; a lone `{` is picked up by `LoneBrace` below.
    #[regex(r"[^\[\]{\\]+", |lex| lex.slice().to_string())]
    Text(String),
    #[token("{")]
    LoneBrace,
}

/// Inside `{{ }}`.
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"\s+")]
enum ExprTok {
    #[token("}}")]
    CloseExpr,
    #[token("|")]
    Pipe,
    #[token(".")]
    Dot,
    #[regex(r"[A-Za-z0-9_]+", |lex| lex.slice().to_string())]
    Ident(String),
    #[regex(r#""([^"\\]|\\["\\])*""#, |lex| unescape_str(lex.slice()))]
    Str(String),
    /// Only reachable when a previous placeholder was left unclosed.
    #[token("{{")]
    NestedOpen,
}

/// Strip the surrounding quotes and resolve `\"` and `\\`.
fn unescape_str(raw: &str) -> String {
    let inner = &raw[1..raw.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut out: Vec<Token> = Vec::new();
    let mut text = TextTok::lexer(src);

    // Literal text arrives as several tokens (Text / Escape / LoneBrace) but
    // the parser wants one string, so accumulate until a delimiter.
    let mut pending: Option<(String, Span)> = None;

    macro_rules! flush {
        () => {
            if let Some((s, span)) = pending.take() {
                out.push(Token {
                    tok: Tok::Text(s),
                    span,
                });
            }
        };
    }
    macro_rules! push_text {
        ($s:expr, $span:expr) => {{
            let span: Span = $span;
            match &mut pending {
                Some((buf, sp)) => {
                    buf.push_str($s);
                    sp.end = span.end;
                }
                None => pending = Some(($s.to_string(), span)),
            }
        }};
    }

    while let Some(next) = text.next() {
        let span: Span = text.span().into();
        match next {
            Err(()) => {
                return Err(LexError {
                    message: format!("unexpected `{}`", &src[span.start..span.end]),
                    span,
                })
            }
            Ok(TextTok::Text(s)) => push_text!(&s, span),
            Ok(TextTok::Escape(c)) => push_text!(c.to_string().as_str(), span),
            Ok(TextTok::LoneBrace) => push_text!("{", span),

            Ok(TextTok::BadEscape) => {
                // Report what actually follows the backslash; logos did not
                // consume it, because no escape rule matched.
                let rest = &src[span.end..];
                return Err(match rest.chars().next() {
                    Some(c) => LexError {
                        message: format!(
                            "unknown escape `\\{c}` — only \\[ \\] \\{{ \\}} \\\\ are escapes"
                        ),
                        span: Span::new(span.start, span.end + c.len_utf8()),
                    },
                    None => LexError {
                        message: "template ends with a trailing `\\`".to_string(),
                        span,
                    },
                });
            }

            Ok(TextTok::OpenGroup) => {
                flush!();
                out.push(Token {
                    tok: Tok::OpenGroup,
                    span,
                });
            }
            Ok(TextTok::CloseGroup) => {
                flush!();
                out.push(Token {
                    tok: Tok::CloseGroup,
                    span,
                });
            }

            Ok(TextTok::OpenExpr) => {
                flush!();
                out.push(Token {
                    tok: Tok::OpenExpr,
                    span,
                });
                // Switch to expression mode for the body of the placeholder.
                let mut expr = text.morph::<ExprTok>();
                let closed = loop {
                    let Some(t) = expr.next() else { break false };
                    let s: Span = expr.span().into();
                    match t {
                        Err(()) => {
                            let slice = &src[s.start..s.end];
                            return Err(LexError {
                                message: if slice.starts_with('"') {
                                    "unterminated quoted literal".to_string()
                                } else {
                                    format!(
                                        "unexpected `{}` inside a placeholder",
                                        slice.chars().next().unwrap_or('?')
                                    )
                                },
                                span: s,
                            });
                        }
                        Ok(ExprTok::NestedOpen) => {
                            return Err(LexError {
                                message:
                                    "`{{` inside a placeholder — the previous one is not closed"
                                        .to_string(),
                                span: s,
                            })
                        }
                        Ok(ExprTok::CloseExpr) => {
                            out.push(Token {
                                tok: Tok::CloseExpr,
                                span: s,
                            });
                            break true;
                        }
                        Ok(ExprTok::Pipe) => out.push(Token {
                            tok: Tok::Pipe,
                            span: s,
                        }),
                        Ok(ExprTok::Dot) => out.push(Token {
                            tok: Tok::Dot,
                            span: s,
                        }),
                        Ok(ExprTok::Ident(i)) => out.push(Token {
                            tok: Tok::Ident(i),
                            span: s,
                        }),
                        Ok(ExprTok::Str(v)) => out.push(Token {
                            tok: Tok::Str(v),
                            span: s,
                        }),
                    }
                };
                if !closed {
                    // Running out of input mid-placeholder is its own branch
                    // here, which is what makes it hard to forget.
                    return Err(LexError {
                        message: "unclosed `{{` — every placeholder needs a matching `}}`"
                            .to_string(),
                        span: Span::new(span.start, src.len()),
                    });
                }
                text = expr.morph();
            }
        }
    }

    flush!();
    let end = src.len();
    out.push(Token {
        tok: Tok::Eof,
        span: Span::new(end, end),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        lex(s).expect("lexes").into_iter().map(|t| t.tok).collect()
    }

    fn err(s: &str) -> LexError {
        lex(s).expect_err("should not lex")
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

    /// logos emits text, escapes and lone braces as separate tokens; the
    /// parser must see a single run.
    #[test]
    fn coalesces_text_escapes_and_braces_into_one_token() {
        assert_eq!(
            toks(r"a\[b{c}d"),
            vec![Tok::Text("a[b{c}d".into()), Tok::Eof]
        );
    }

    #[test]
    fn spans_are_byte_accurate_through_non_ascii() {
        // `é` is two bytes, so a naive char-count span would be wrong.
        let tokens = lex("é{{a}}").expect("lexes");
        let expr = tokens.iter().find(|t| t.tok == Tok::OpenExpr).unwrap();
        assert_eq!(expr.span.start, 2);
    }

    #[test]
    fn a_coalesced_text_span_covers_the_whole_run() {
        // ab + \[ + cd occupies bytes 0..6; `{{` begins at 6.
        let tokens = lex(r"ab\[cd{{x}}").expect("lexes");
        assert_eq!(tokens[0].span, Span::new(0, 6));
    }

    #[test]
    fn reports_the_offending_character_for_a_bad_escape() {
        let e = err(r"\q");
        assert!(
            e.message.contains(r"unknown escape `\q`"),
            "got: {}",
            e.message
        );
        assert_eq!(e.span, Span::new(0, 2));
    }

    #[test]
    fn reports_a_trailing_backslash() {
        assert!(err("x\\").message.contains("trailing"));
    }

    #[test]
    fn reports_an_unterminated_quoted_literal() {
        assert!(err("{{a|\"oops}}").message.contains("unterminated"));
    }

    #[test]
    fn reports_an_unclosed_placeholder() {
        let e = err("{{a");
        assert!(e.message.contains("unclosed `{{`"), "got: {}", e.message);
    }

    #[test]
    fn reports_a_nested_open_placeholder() {
        assert!(err("{{a {{b}}").message.contains("not closed"));
    }

    #[test]
    fn reports_an_unexpected_character_inside_a_placeholder() {
        assert!(err("{{a b!}}").message.contains("inside a placeholder"));
    }
}
