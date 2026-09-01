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
use super::FieldDoc;

#[derive(Debug, Clone)]
pub enum Alt {
    /// A field path, already validated against the schema, with the
    /// nullability the schema declared for it.
    Field { path: String, nullable: bool },
    /// A quoted literal, which always resolves.
    Literal(String),
}

#[derive(Debug, Clone)]
pub enum Node {
    Text {
        text: String,
        span: Span,
    },
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

pub fn parse(input: &str, schema: &[FieldDoc]) -> Result<Vec<Node>, ParseError> {
    let tokens = lex(input).map_err(|e| ParseError {
        message: e.message,
        span: e.span,
        help: None,
        code: "template_syntax_error",
    })?;
    let mut p = Parser {
        src: input,
        schema,
        tokens,
        pos: 0,
    };
    let items = p.parse_items(None)?;
    p.expect_eof()?;
    check_leading_separator(&items)?;
    check_literal_segments(&items, true)?;
    Ok(items)
}

struct Parser<'a> {
    src: &'a str,
    schema: &'a [FieldDoc],
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
                    if let Tok::Text(text) = t.tok {
                        items.push(Node::Text { text, span: t.span });
                    }
                }
                Tok::OpenGroup => items.push(self.parse_group()?),
                Tok::OpenExpr => {
                    let node = self.parse_placeholder(open_group.is_some())?;
                    items.push(node);
                }
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

        // A group is dropped when a placeholder *at its own level* resolves to
        // nothing: one inside a nested group drops that group instead. So the
        // question is not whether a placeholder appears somewhere below, but
        // whether one of this group's own can fail. If none can, the brackets
        // do nothing, which is always a mistake.
        let mut direct = items
            .iter()
            .filter_map(|n| match n {
                Node::Placeholder { alts, .. } => Some(alts),
                _ => None,
            })
            .peekable();
        if direct.peek().is_none() {
            return Err(ParseError::new(
                "template_empty_group",
                "this group contains no placeholder of its own, so it would always render",
                span,
            )
            .help(
                "A group `[…]` is dropped when a placeholder inside it turns out to be \
                 absent. For a literal bracket, write `\\[` and `\\]`.",
            ));
        }
        if !direct.any(|alts| can_fail(alts)) {
            return Err(ParseError::new(
                "template_dead_group",
                "every placeholder in this group is always present, so it would always render",
                span,
            )
            .help(
                "A group `[…]` exists to be dropped. Drop the brackets, or use a \
                 field that can be absent.",
            ));
        }
        Ok(Node::Group { items, span })
    }

    fn parse_placeholder(&mut self, in_group: bool) -> Result<Node, ParseError> {
        let open = self.bump().span; // `{{`
        let mut alts = Vec::new();
        let mut spans = Vec::new();

        let mut is_first = true;
        loop {
            let (alt, span) = self.parse_alt(open, is_first)?;
            alts.push(alt);
            spans.push(span);
            is_first = false;
            match &self.peek().tok {
                Tok::Pipe => {
                    self.bump();
                }
                _ => break,
            }
        }

        // Anything after an alternative that always resolves is dead code.
        if let Some(i) = alts.iter().position(always_resolves) {
            if let Some(dead) = spans.get(i + 1) {
                let sure = match &alts[i] {
                    Alt::Literal(_) => "a quoted literal is always there".to_string(),
                    Alt::Field { path, .. } => format!("`{path}` is never absent"),
                };
                return Err(ParseError::new(
                    "template_unreachable_alternative",
                    format!("this alternative is unreachable — {sure}"),
                    *dead,
                )
                .help("Remove it, or put it before the alternative that is always there."));
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
        let source = self.src[span.start..span.end].to_string();

        // Outside a group there is nowhere for an absent value to go, so a
        // placeholder that might resolve to nothing is rejected here rather
        // than on whichever record first happens to lack it.
        if !in_group && can_fail(&alts) {
            return Err(ParseError::new(
                "template_unresolvable",
                format!("{source} may be absent, and is not inside a group"),
                span,
            )
            .help(format!(
                "Add a fallback like {{{{…|\"Unknown\"}}}}, or wrap it in a group so it \
                 can be dropped: [{source}]"
            )));
        }
        Ok(Node::Placeholder { alts, span, source })
    }

    /// `is_first` distinguishes `{{}}` from a dangling `{{year|}}`.
    ///
    /// Returns the alternative with its own span, so a diagnostic can point at
    /// one alternative inside a placeholder rather than the whole thing.
    fn parse_alt(&mut self, open: Span, is_first: bool) -> Result<(Alt, Span), ParseError> {
        match self.peek().tok.clone() {
            Tok::Str(s) => {
                let span = self.bump().span;
                Ok((Alt::Literal(s), span))
            }
            Tok::Ident(head) => {
                let start = self.bump().span;
                let mut path = head;
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
                let Some(doc) = self.schema.iter().find(|f| f.path == path) else {
                    return Err(ParseError::new(
                        "template_unknown_field",
                        format!("unknown field `{path}`"),
                        span,
                    )
                    .help(suggest_field(&path, self.schema)));
                };
                Ok((
                    Alt::Field {
                        path,
                        nullable: doc.nullable,
                    },
                    span,
                ))
            }
            // Distinguish `{{}}` from `{{year|}}`: the second is a dangling
            // fallback, not an empty placeholder.
            Tok::CloseExpr if is_first => Err(ParseError::new(
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

/// Does this alternative resolve for every record?
fn always_resolves(alt: &Alt) -> bool {
    match alt {
        Alt::Literal(_) => true,
        Alt::Field { nullable, .. } => !nullable,
    }
}

/// Can this placeholder resolve to nothing for some record?
///
/// This is the invariant `render` leans on: `parse` rejects a placeholder for
/// which this holds unless it sits in a group, so rendering can treat every
/// other placeholder as certain to produce a value.
fn can_fail(alts: &[Alt]) -> bool {
    !alts.iter().any(always_resolves)
}

/// A `/` at the very start of the template makes every render absolute.
///
/// Only a leading run of literal text can be known to do this: when a group
/// or placeholder comes first, whether a separator ends up leading depends on
/// what resolves, and that is left to the renderer.
fn check_leading_separator(items: &[Node]) -> Result<(), ParseError> {
    if let Some(Node::Text { text, span }) = items.first() {
        if text.starts_with('/') {
            return Err(ParseError::new(
                "template_absolute_path",
                "template renders an absolute path",
                Span::new(span.start, span.start + 1),
            )
            .help("Drop the leading `/`; a rendered result is always relative."));
        }
    }
    Ok(())
}

/// Reject a `.` or `..` written as a whole path segment.
///
/// A value can never produce one — [`sanitize_value`] turns a dots-only value
/// into underscores and a `/` into `-` — so a traversal segment can only come
/// from the template's own text, which makes this entirely a parse-time
/// question.
///
/// A piece of text counts as a whole segment when a `/` bounds it on both
/// sides. Inside a text run that is any interior piece; at the very start or
/// end of the template the outer edge serves as the other bound. A piece that
/// abuts a placeholder or a group is not checked, because whatever that
/// contributes joins the same segment.
///
/// [`sanitize_value`]: crate::template::render
fn check_literal_segments(items: &[Node], top: bool) -> Result<(), ParseError> {
    for (i, node) in items.iter().enumerate() {
        match node {
            Node::Text { text, span } => {
                let pieces: Vec<&str> = text.split('/').collect();
                for (j, piece) in pieces.iter().enumerate() {
                    let bounded_left = j > 0 || (top && i == 0);
                    let bounded_right = j + 1 < pieces.len() || (top && i + 1 == items.len());
                    if !bounded_left || !bounded_right {
                        continue;
                    }
                    let seg = piece.trim();
                    if seg == "." || seg == ".." {
                        return Err(ParseError::new(
                            "template_bad_path_segment",
                            format!("`{seg}` is not a usable path segment"),
                            *span,
                        )
                        .help(
                            "A rendered result may not name a parent or the current \
                             directory. Remove the segment.",
                        ));
                    }
                }
            }
            Node::Group { items, .. } => check_literal_segments(items, false)?,
            Node::Placeholder { .. } => {}
        }
    }
    Ok(())
}

/// Point at the right field.
///
/// When the namespace is real (`composer.` in `composer.surname`), listing its
/// actual fields beats guessing: edit distance has no idea that "surname"
/// means "lastName", and would offer `composer.fullName` instead.
fn suggest_field(path: &str, schema: &[FieldDoc]) -> String {
    if let Some((prefix, _)) = path.rsplit_once('.') {
        let siblings: Vec<&str> = schema
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
            return format!("Fields on `{prefix}`: {}.", leaves.join(", "));
        }
    }
    let mut best: Option<(usize, &str)> = None;
    for f in schema {
        let d = edit_distance(path, f.path);
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, f.path));
        }
    }
    match best {
        // Only suggest when it is plausibly the same word mistyped.
        Some((d, name)) if d <= path.len() / 2 + 1 => format!("Did you mean `{name}`?"),
        _ => "No field by that name.".to_string(),
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
