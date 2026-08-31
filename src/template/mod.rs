//! The template language: lexer, parser, renderer.
//!
//! ```text
//! {{a|b|"lit"}}   first alternative that resolves; a quoted literal always does
//! [ … ]           optional group, dropped entirely if a placeholder inside
//!                 resolves to nothing
//! \[ \] \{ \} \\  escapes
//! ```
//!
//! Two properties are deliberate. Field paths are validated when the template
//! is parsed, so a typo fails before any request is made. And a placeholder
//! that resolves to nothing is an error unless it sits in a group — absent
//! data can never silently produce a malformed name.

mod lexer;
mod parser;
mod render;

pub use lexer::Span;
pub use render::RenderError;

use crate::model::Recording;

pub struct Template {
    nodes: Vec<parser::Node>,
}

#[derive(Debug)]
pub struct TemplateError {
    pub message: String,
    pub span: Span,
    pub help: Option<String>,
    pub code: &'static str,
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl TemplateError {
    /// The template with the offending span underlined, as two lines:
    ///
    /// ```text
    /// {{composer.surname}}/{{opera.title}}
    /// ^^^^^^^^^^^^^^^^^^
    /// ```
    ///
    /// Returned as separate lines so the caller can indent them to suit.
    pub fn underline(&self, source: &str) -> Vec<String> {
        // Columns are counted in characters, not bytes, so the caret lines up
        // under templates containing non-ASCII.
        let before = source.get(..self.span.start).unwrap_or("").chars().count();
        let width = source
            .get(self.span.start..self.span.end)
            .unwrap_or("")
            .chars()
            .count()
            .max(1);
        vec![
            source.to_string(),
            format!("{}{}", " ".repeat(before), "^".repeat(width)),
        ]
    }
}

impl Template {
    pub fn parse(input: &str) -> Result<Self, TemplateError> {
        let nodes = parser::parse(input).map_err(|e| TemplateError {
            message: e.message,
            span: e.span,
            help: e.help,
            code: e.code,
        })?;
        Ok(Template { nodes })
    }

    /// Render for one recording, returning the result split on `/`.
    pub fn render(&self, rec: &Recording) -> Result<Vec<String>, RenderError> {
        render::render(&self.nodes, rec)
    }
}
