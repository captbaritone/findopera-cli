//! The template language: lexer, parser, renderer, and the schema seam.
//!
//! ```text
//! {{a|b|"lit"}}   first alternative that resolves; a quoted literal always does
//! [ … ]           optional group, dropped entirely if a placeholder inside
//!                 resolves to nothing
//! \[ \] \{ \} \\  escapes
//! ```
//!
//! Absent data can never silently produce a malformed name: a placeholder
//! that resolves to nothing is an error unless it sits in a group.
//!
//! # What parsing settles
//!
//! As much as possible is decided against the schema alone, with no record in
//! hand, so a bad template fails once rather than on whichever record first
//! exposes it:
//!
//! - a field path that is not in the schema
//! - a placeholder that might resolve to nothing outside any group, which
//!   needs [`FieldDoc::non_null`] to know
//! - an alternative after one that always resolves, which can never be reached
//! - a group that can never be dropped, because every placeholder at its own
//!   level always resolves
//! - a leading `/`, or a literal `.` or `..` path segment
//!
//! That leaves [`Template::render`] with nothing to fail at: it returns a
//! `String` for any record. The one thing still decided per record is whether
//! that string is a usable relative path, which is [`to_path`]'s job — a
//! template can render `""` or `/Salome` for one record and something
//! perfectly good for the next.
//!
//! # The data seam
//!
//! The engine knows nothing about any particular record type. It is given two
//! things, deliberately kept apart because they are needed at different times:
//!
//! - a **schema**, a `&[FieldDoc]` naming every path a template may reference.
//!   Plain data, because parsing must validate paths with no value in hand.
//! - a **resolver**, some [`Fields`] implementation that turns a path into a
//!   value. Behavior, because only rendering needs it.
//!
//! ```
//! use findopera::{FieldDoc, Fields, Template};
//!
//! static FIELDS: &[FieldDoc] = &[
//!     FieldDoc::new("year", "Year recorded"),                       // may be absent
//!     FieldDoc::non_null("opera.title", "Title in the original"),   // always there
//! ];
//!
//! struct Recording;
//! impl Fields for Recording {
//!     fn required(&self, path: &str) -> String {
//!         match path {
//!             "opera.title" => "Salome".to_string(),
//!             _ => unreachable!(),
//!         }
//!     }
//!     fn optional(&self, _path: &str) -> Option<String> {
//!         None // no year on this one
//!     }
//! }
//!
//! let tmpl = Template::parse("{{opera.title}}[ ({{year}})]", FIELDS)?;
//! let rendered = tmpl.render(&Recording);
//! assert_eq!(rendered, "Salome");
//! assert_eq!(findopera::to_path(&rendered)?, vec!["Salome"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! A generated model implements this by emitting a `static FIELDS` table
//! alongside a `Fields` impl, so the two cannot drift apart.

mod lexer;
mod parser;
mod render;

pub use lexer::Span;
pub use render::{to_path, PathError};

/// One field a template may reference, and what it means.
///
/// The set of these is the single source of truth for which paths are valid;
/// [`Template::parse`] rejects anything not named here.
pub struct FieldDoc {
    pub path: &'static str,
    pub description: &'static str,
    /// Whether this field can be absent for some record.
    ///
    /// This is what lets `parse` decide, with no data in hand, whether a
    /// placeholder might resolve to nothing. Nullable is the safe direction to
    /// be wrong in: understating it only costs a check that could have been
    /// made, while claiming a field is always present when it is not turns a
    /// parse-time error into a render-time one for whichever record first
    /// lacks it — exactly what the check exists to prevent.
    pub nullable: bool,
}

impl FieldDoc {
    /// A field that may be absent for some records.
    pub const fn new(path: &'static str, description: &'static str) -> Self {
        FieldDoc {
            path,
            description,
            nullable: true,
        }
    }

    /// A field present for every record, so a placeholder over it always
    /// resolves and needs no fallback.
    pub const fn non_null(path: &'static str, description: &'static str) -> Self {
        FieldDoc {
            path,
            description,
            nullable: false,
        }
    }
}

/// Resolves a validated field path against one record.
///
/// The two methods mirror the split the schema already declares, which is what
/// makes rendering total: a path this resolver is asked for through
/// [`required`](Fields::required) was declared [`non_null`](FieldDoc::non_null),
/// and returning a `String` for it is not optional.
///
/// For [`optional`](Fields::optional), `None` means absent — the signal that
/// drives group omission. An implementation should collapse its own sentinels
/// for unknown (SQL `NULL`, `0`, `""`) into `None`, so templates only ever
/// reason about present-versus-absent. Better still, collapse them where the
/// record is deserialized, so the type itself carries the distinction and this
/// impl has nothing to decide.
///
/// The parser has already checked every path against the schema and routed it
/// to the right method, so neither needs to handle an unknown path.
pub trait Fields {
    /// A field the schema declares always present.
    fn required(&self, path: &str) -> String;
    /// A field the schema declares may be absent.
    fn optional(&self, path: &str) -> Option<String>;
}

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

impl std::error::Error for TemplateError {}

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
    /// Parse a template, validating every field path against `schema`.
    pub fn parse(input: &str, schema: &[FieldDoc]) -> Result<Self, TemplateError> {
        let nodes = parser::parse(input, schema).map_err(|e| TemplateError {
            message: e.message,
            span: e.span,
            help: e.help,
            code: e.code,
        })?;
        Ok(Template { nodes })
    }

    /// Render for one record.
    ///
    /// Total: for any template this crate parsed and any record, there is a
    /// string. Whether that string is a usable relative path is a separate
    /// question — see [`to_path`].
    pub fn render(&self, data: &dyn Fields) -> String {
        render::render(&self.nodes, data)
    }
}
