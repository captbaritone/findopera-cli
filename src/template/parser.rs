//! Recursive-descent parser over the token stream.
//!
//! ```text
//! template   := item*
//! item       := TEXT | group | placeholder
//! group      := '[' item* ']'
//! placeholder:= '{{' alt ('|' alt)* '}}'
//! alt        := path | STRING
//! path       := IDENT ('.' IDENT)*
//! ```
//!
//! Field paths are checked against the known field set here, at parse time, so
//! a mistyped field fails before any network request is made.

use super::lexer::{lex, Span, Tok, Token};
use crate::model::Recording;

#[derive(Debug, Clone)]
pub enum Alt {
    /// A field path, already validated against [`Recording::is_known`].
    /// The span is kept for diagnostics that point inside a placeholder.
    Field {
        path: String,
        #[allow(dead_code)]
        span: Span,
    },
    /// A quoted literal, which always resolves.
    Literal(String),
}

#[derive(Debug, Clone)]
pub enum Node {
    Text(String),
    Placeholder {
        alts: Vec<Alt>,
        #[allow(dead_code)]
        span: Span,
        /// The placeholder as written, for error messages.
        source: String,
    },
    /// An optional group: rendered only if every placeholder inside resolves.
    Group {
        items: Vec<Node>,
        #[allow(dead_code)]
        span: Span,
    },
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    /// Extra guidance, when there is something specific to suggest.
    pub help: Option<String>,
    pub code: &'static str,
}

impl ParseError {
    fn new(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        ParseError {
            message: message.into(),
            span,
            help: None,
            code,
        }
    }
    fn help(mut self, h: impl Into<String>) -> Self {
        self.help = Some(h.into());
        self
    }
}

pub fn parse(input: &str) -> Result<Vec<Node>, ParseError> {
    let tokens = lex(input).map_err(|e| ParseError {
        message: e.message,
        span: e.span,
        help: None,
        code: "template_syntax_error",
    })?;
    let mut p = Parser {
        src: input,
        tokens,
        pos: 0,
    };
    let items = p.parse_items(None)?;
    p.expect_eof()?;
    Ok(items)
}

struct Parser<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> &Token {
        // `lex` always terminates the stream with Eof, so this cannot fail.
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn expect_eof(&mut self) -> Result<(), ParseError> {
        match &self.peek().tok {
            Tok::Eof => Ok(()),
            Tok::CloseGroup => {
                Err(
                    ParseError::new("template_syntax_error", "unmatched `]`", self.peek().span)
                        .help("Write `\\]` for a literal bracket."),
                )
            }
            other => Err(ParseError::new(
                "template_syntax_error",
                format!("unexpected {}", other.describe()),
                self.peek().span,
            )),
        }
    }

    /// Parse items until Eof, or until the `]` closing `open_group`.
    fn parse_items(&mut self, open_group: Option<Span>) -> Result<Vec<Node>, ParseError> {
        let mut items = Vec::new();
        loop {
            match &self.peek().tok {
                Tok::Eof => {
                    if let Some(open) = open_group {
                        return Err(ParseError::new(
                            "template_syntax_error",
                            "unclosed `[` — every group needs a matching `]`",
                            open,
                        )
                        .help("Write `\\[` for a literal bracket."));
                    }
                    return Ok(items);
                }
                Tok::CloseGroup => {
                    if open_group.is_some() {
                        return Ok(items);
                    }
                    // Handled by expect_eof, which produces a better message.
                    return Ok(items);
                }
                Tok::Text(_) => {
                    let t = self.bump();
                    if let Tok::Text(s) = t.tok {
                        items.push(Node::Text(s));
                    }
                }
                Tok::OpenGroup => items.push(self.parse_group()?),
                Tok::OpenExpr => items.push(self.parse_placeholder()?),
                other => {
                    let d = other.describe();
                    let span = self.peek().span;
                    return Err(ParseError::new(
                        "template_syntax_error",
                        format!("unexpected {d}"),
                        span,
                    ));
                }
            }
        }
    }

    fn parse_group(&mut self) -> Result<Node, ParseError> {
        let open = self.bump().span; // `[`
        let items = self.parse_items(Some(open))?;
        let close = match &self.peek().tok {
            Tok::CloseGroup => self.bump().span,
            // parse_items only returns on Eof or CloseGroup, and Eof already
            // errored above, so this is unreachable in practice.
            _ => {
                return Err(ParseError::new(
                    "template_syntax_error",
                    "unclosed `[`",
                    open,
                ))
            }
        };
        let span = open.to(close);

        // A group with no placeholder can never be omitted, so it is always a
        // mistake — most often a literal bracket that wanted escaping.
        if !contains_placeholder(&items) {
            return Err(ParseError::new(
                "template_empty_group",
                "this group contains no placeholder, so it would always render",
                span,
            )
            .help(
                "A group `[…]` is dropped when a placeholder inside it resolves to \
                 nothing. For a literal bracket, write `\\[` and `\\]`.",
            ));
        }
        Ok(Node::Group { items, span })
    }

    fn parse_placeholder(&mut self) -> Result<Node, ParseError> {
        let open = self.bump().span; // `{{`
        let mut alts = Vec::new();

        let mut first = true;
        loop {
            alts.push(self.parse_alt(open, first)?);
            first = false;
            match &self.peek().tok {
                Tok::Pipe => {
                    self.bump();
                }
                _ => break,
            }
        }

        let close = match &self.peek().tok {
            Tok::CloseExpr => self.bump().span,
            other => {
                let d = other.describe();
                let span = self.peek().span;
                return Err(ParseError::new(
                    "template_syntax_error",
                    format!("expected `}}}}` or `|`, found {d}"),
                    span,
                ));
            }
        };

        let span = open.to(close);
        Ok(Node::Placeholder {
            alts,
            span,
            source: self.src[span.start..span.end].to_string(),
        })
    }

    fn parse_alt(&mut self, open: Span, first: bool) -> Result<Alt, ParseError> {
        match self.peek().tok.clone() {
            Tok::Str(s) => {
                self.bump();
                Ok(Alt::Literal(s))
            }
            Tok::Ident(first) => {
                let start = self.bump().span;
                let mut path = first;
                let mut end = start;
                while matches!(self.peek().tok, Tok::Dot) {
                    self.bump();
                    match self.peek().tok.clone() {
                        Tok::Ident(seg) => {
                            end = self.bump().span;
                            path.push('.');
                            path.push_str(&seg);
                        }
                        other => {
                            let d = other.describe();
                            let span = self.peek().span;
                            return Err(ParseError::new(
                                "template_syntax_error",
                                format!("expected a field name after `.`, found {d}"),
                                span,
                            ));
                        }
                    }
                }
                let span = start.to(end);
                if !Recording::is_known(&path) {
                    return Err(ParseError::new(
                        "template_unknown_field",
                        format!("unknown field `{path}`"),
                        span,
                    )
                    .help(suggest_field(&path)));
                }
                Ok(Alt::Field { path, span })
            }
            // Distinguish `{{}}` from `{{year|}}`: the second is a dangling
            // fallback, not an empty placeholder.
            Tok::CloseExpr if first => Err(ParseError::new(
                "template_syntax_error",
                "empty placeholder",
                open.to(self.peek().span),
            )),
            Tok::CloseExpr => Err(ParseError::new(
                "template_syntax_error",
                "expected a field name or quoted literal after `|`",
                self.peek().span,
            )),
            other => {
                let d = other.describe();
                let span = self.peek().span;
                Err(ParseError::new(
                    "template_syntax_error",
                    format!("expected a field name or quoted literal, found {d}"),
                    span,
                ))
            }
        }
    }
}

fn contains_placeholder(items: &[Node]) -> bool {
    items.iter().any(|n| match n {
        Node::Placeholder { .. } => true,
        Node::Group { items, .. } => contains_placeholder(items),
        Node::Text(_) => false,
    })
}

/// Point at the right field.
///
/// When the namespace is real (`composer.` in `composer.surname`), listing its
/// actual fields beats guessing: edit distance has no idea that "surname"
/// means "lastName", and would offer `composer.fullName` instead.
fn suggest_field(path: &str) -> String {
    if let Some((prefix, _)) = path.rsplit_once('.') {
        let siblings: Vec<&str> = Recording::FIELDS
            .iter()
            .map(|f| f.path)
            .filter(|p| p.strip_prefix(prefix).is_some_and(|r| r.starts_with('.')))
            .collect();
        if !siblings.is_empty() {
            // Show leaf names; the prefix is already in the message.
            let leaves: Vec<&str> = siblings
                .iter()
                .filter_map(|p| p.strip_prefix(prefix)?.strip_prefix('.'))
                .collect();
            return format!(
                "Fields on `{prefix}`: {}. Run `findopera fields` for the full list.",
                leaves.join(", ")
            );
        }
    }
    let mut best: Option<(usize, &str)> = None;
    for f in Recording::FIELDS {
        let d = edit_distance(path, f.path);
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, f.path));
        }
    }
    match best {
        // Only suggest when it is plausibly the same word mistyped.
        Some((d, name)) if d <= path.len() / 2 + 1 => {
            format!("Did you mean `{name}`? Run `findopera fields` for the full list.")
        }
        _ => "Run `findopera fields` for the full list.".to_string(),
    }
}

/// Levenshtein distance, two rows at a time.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(s: &str) -> ParseError {
        parse(s).expect_err("should not parse")
    }

    #[test]
    fn parses_text_placeholders_and_groups() {
        let nodes = parse("a{{opera.title}}[ - {{year}}]").expect("parses");
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0], Node::Text(_)));
        assert!(matches!(nodes[1], Node::Placeholder { .. }));
        assert!(matches!(nodes[2], Node::Group { .. }));
    }

    #[test]
    fn parses_nested_groups() {
        let nodes = parse("[{{year}}[ - {{conductor.lastName}}]]").expect("parses");
        let Node::Group { items, .. } = &nodes[0] else {
            panic!("expected a group")
        };
        assert!(items.iter().any(|n| matches!(n, Node::Group { .. })));
    }

    #[test]
    fn parses_fallback_chains() {
        let nodes = parse("{{opera.englishTitle|opera.title|\"Untitled\"}}").expect("parses");
        let Node::Placeholder { alts, .. } = &nodes[0] else {
            panic!("expected a placeholder")
        };
        assert_eq!(alts.len(), 3);
        assert!(matches!(alts[2], Alt::Literal(_)));
    }

    #[test]
    fn rejects_unknown_fields_with_a_suggestion() {
        let e = err("{{composer.surname}}");
        assert_eq!(e.code, "template_unknown_field");
        let help = e.help.unwrap();
        assert!(help.contains("lastName"), "got: {help}");
        assert!(help.contains("born"), "should list the namespace");
        assert!(help.contains("findopera fields"));
    }

    #[test]
    fn rejects_unbalanced_brackets() {
        assert!(err("[{{year}}").message.contains("unclosed `[`"));
        assert!(err("{{year}}]").message.contains("unmatched `]`"));
    }

    #[test]
    fn rejects_a_group_that_could_never_be_omitted() {
        let e = err("[ - ]");
        assert_eq!(e.code, "template_empty_group");
        assert!(e.help.unwrap().contains("\\["));
    }

    #[test]
    fn a_nested_placeholder_satisfies_the_group_check() {
        assert!(parse("[ - [{{year}}]]").is_ok());
    }

    #[test]
    fn rejects_malformed_placeholders() {
        assert!(err("{{}}").message.contains("empty placeholder"));
        assert!(err("{{opera.}}").message.contains("after `.`"));
        assert!(err("{{year|}}").message.contains("after `|`"));
        assert!(err("{{nosuchthing}}").message.contains("unknown field"));
    }

    #[test]
    fn spans_point_at_the_offending_text() {
        let e = err("{{composer.surname}}");
        assert_eq!(
            &"{{composer.surname}}"[e.span.start..e.span.end],
            "composer.surname"
        );
    }

    #[test]
    fn escaped_brackets_are_text_not_groups() {
        let nodes = parse(r"\[{{year}}\]").expect("parses");
        assert!(!nodes.iter().any(|n| matches!(n, Node::Group { .. })));
    }
}
