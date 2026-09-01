//! A small template language for turning record metadata into path-safe names.
//!
//! ```text
//! {{a|b|"lit"}}   first alternative that resolves; a quoted literal always does
//! [ … ]           optional group, dropped entirely if a placeholder inside
//!                 resolves to nothing
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
pub mod model;
mod template;

pub use template::{to_path, FieldDoc, Fields, PathError, Span, Template, TemplateError};
