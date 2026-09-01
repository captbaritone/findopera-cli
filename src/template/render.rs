//! Walking the AST to produce a string, and judging that string as a path.
//!
//! These are two jobs and the split is deliberate. Rendering is **total**: for
//! any template `parse` accepted and any record, it produces a string. That is
//! what the schema buys — a placeholder outside a group is only accepted when
//! some alternative always resolves, so there is no case left where rendering
//! has nothing to write.
//!
//! Whether that string is a usable relative path is a separate question, and
//! the only one that can still fail per record. A template can render `""` or
//! `/Salome` for one record and something perfectly good for the next.
//!
//! The one rule worth knowing about rendering: an unresolved placeholder
//! inside a `[…]` group drops the whole group, which is what makes separators
//! vanish along with their value — `[{{year}} - ]` contributes nothing at all
//! when the year is absent, rather than leaving a dangling ` - `.

use super::parser::{Alt, Node};
use super::Fields;

/// A rendered string that cannot be used as a relative path.
#[derive(Debug)]
pub enum PathError {
    /// Names an absolute path.
    Absolute { rendered: String },
    /// Contains a `.` or `..` segment.
    Traversal { rendered: String },
    /// Has no segments at all.
    Empty { rendered: String },
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Absolute { rendered } => write!(
                f,
                "rendered {rendered:?}, which names an absolute path — something \
                 before the leading `/` resolved to nothing"
            ),
            Self::Traversal { rendered } => write!(
                f,
                "rendered {rendered:?}, which contains a `.` or `..` segment"
            ),
            Self::Empty { rendered } => {
                write!(f, "rendered {rendered:?}, which has no path segments")
            }
        }
    }
}

impl std::error::Error for PathError {}

impl PathError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Absolute { .. } => "path_absolute",
            Self::Traversal { .. } => "path_traversal",
            Self::Empty { .. } => "path_empty",
        }
    }

    /// The string that was judged, whatever the verdict.
    pub fn rendered(&self) -> &str {
        match self {
            Self::Absolute { rendered }
            | Self::Traversal { rendered }
            | Self::Empty { rendered } => rendered,
        }
    }
}

/// Render the whole template. Cannot fail; see the module docs.
pub fn render(items: &[Node], data: &dyn Fields) -> String {
    let mut out = String::new();
    render_seq(items, data, &mut out, false);
    out
}

/// Append `items` to `out`.
///
/// Returns `false` when a placeholder resolved to nothing and `in_group` is
/// set, meaning the caller should discard what was written.
fn render_seq(items: &[Node], data: &dyn Fields, out: &mut String, in_group: bool) -> bool {
    for item in items {
        match item {
            Node::Text { text, .. } => out.push_str(text),

            Node::Placeholder { alts, source, .. } => match resolve(alts, data) {
                Some(v) => out.push_str(&sanitize_value(&v)),
                None if in_group => return false,
                // `parse` rejects a placeholder that can resolve to nothing
                // unless it is inside a group, so this cannot be reached
                // through the public API.
                None => unreachable!(
                    "{source} resolved to nothing outside a group, which parsing rejects"
                ),
            },

            Node::Group { items, .. } => {
                // Render into a scratch buffer so a dropped group leaves
                // nothing behind. An omitted *inner* group is not a failure of
                // the outer one, so the result is not propagated.
                let mut buf = String::new();
                if render_seq(items, data, &mut buf, true) {
                    out.push_str(&buf);
                }
            }
        }
    }
    true
}

/// First alternative that yields a value, left to right.
///
/// The schema decides which accessor each alternative uses, so a field the
/// schema calls non-null is fetched through [`Fields::required`] and cannot
/// come back empty-handed.
fn resolve(alts: &[Alt], data: &dyn Fields) -> Option<String> {
    for alt in alts {
        match alt {
            Alt::Literal(l) => return Some(l.clone()),
            Alt::Field {
                path,
                nullable: false,
            } => return Some(data.required(path)),
            Alt::Field {
                path,
                nullable: true,
            } => {
                if let Some(v) = data.optional(path) {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Make one interpolated value safe to sit inside a single path segment.
///
/// Separators in the *template* are structural; separators inside a *value*
/// are not, so a title containing `/` must not introduce a directory level.
/// This runs during rendering because that is the only point that still knows
/// which text came from a value and which from the template.
fn sanitize_value(value: &str) -> String {
    let mut s: String = value
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    s = s.trim().to_string();
    // A value that is entirely dots would otherwise become `.` or `..`.
    if !s.is_empty() && s.chars().all(|c| c == '.' || c == '-' || c.is_whitespace()) {
        s = s.replace('.', "_");
    }
    s
}

/// Split a rendered string into path segments, rejecting anything that would
/// escape a destination directory.
///
/// This is the only per-record failure the crate has left. Everything about
/// the template itself was settled by `parse`.
pub fn to_path(rendered: &str) -> Result<Vec<String>, PathError> {
    let owned = || rendered.to_string();
    if rendered.starts_with('/') {
        return Err(PathError::Absolute { rendered: owned() });
    }
    let mut segments = Vec::new();
    for raw in rendered.split('/') {
        let seg = raw.trim();
        if seg.is_empty() {
            continue; // tolerate `//` and a trailing `/`, e.g. from a dropped group
        }
        if seg == "." || seg == ".." {
            return Err(PathError::Traversal { rendered: owned() });
        }
        // Trailing dots and spaces are silently dropped by some filesystems,
        // which would make the result unstable.
        let seg = seg.trim_end_matches([' ', '.']).to_string();
        if seg.is_empty() {
            return Err(PathError::Empty { rendered: owned() });
        }
        segments.push(seg);
    }
    if segments.is_empty() {
        return Err(PathError::Empty { rendered: owned() });
    }
    Ok(segments)
}
