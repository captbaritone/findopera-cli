//! Naming a music library from FindOpera metadata.
//!
//! [`scan`] finds the marker files in a library, [`api`] fetches what they
//! name, [`plan`] works out what each folder should be called, and [`apply`]
//! builds a tree of those names. The naming itself is a small template
//! language, re-exported here.
//!
//! ```text
//! {{a|b|"lit"}}   first alternative that has a value; a quoted literal always has
//! [ … ]           optional group, dropped entirely if a placeholder inside
//!                 turns out to be absent
//! \[ \] \{ \} \\  escapes
//! ```
//!
//! The `template` module holds the language itself and the seam it reads data
//! through: a schema of [`FieldDoc`]s saying which paths exist and which are
//! always present, and a [`Fields`] resolver supplying values. Everything
//! public is re-exported here, so a caller never names the module.
//!
//! [`model`] supplies both for FindOpera recordings, generated from the
//! GraphQL schema by `codegen/generate.mjs`.

pub mod api;
pub mod apply;
pub mod config;
pub mod model;
pub mod plan;
pub mod scan;
mod template;

pub use template::{to_path, FieldDoc, Fields, PathError, Span, Template, TemplateError, SYNTAX};
