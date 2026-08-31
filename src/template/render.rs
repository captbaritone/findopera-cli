//! Walking the AST to produce a string.
//!
//! The one interesting rule is how an unresolved placeholder is treated:
//! inside a `[…]` group it drops the whole group, and outside one it is an
//! error. That single rule is what makes separators vanish along with their
//! value — `[{{year}} - ]` contributes nothing at all when the year is absent,
//! rather than leaving a dangling ` - `.

use super::parser::{Alt, Node};
use crate::model::Recording;

#[derive(Debug)]
pub enum RenderError {
    /// A placeholder outside any group resolved to nothing.
    Unresolved { placeholder: String },
    /// The rendered result is not a usable relative path.
    BadPath { rendered: String, reason: String },
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unresolved { placeholder } => write!(
                f,
                "{placeholder} resolved to nothing for this recording — add a \
                 fallback like {{{{…|\"Unknown\"}}}}, or wrap it in a group so it \
                 can be dropped: [{placeholder}]"
            ),
            Self::BadPath { rendered, reason } => write!(f, "{reason} (rendered: {rendered:?})"),
        }
    }
}

impl RenderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unresolved { .. } => "template_unresolved_field",
            Self::BadPath { .. } => "template_bad_path",
        }
    }
}

/// Render the whole template, returning the result split into path segments.
pub fn render(items: &[Node], rec: &Recording) -> Result<Vec<String>, RenderError> {
    let mut out = String::new();
    // At the top level an unresolved placeholder is an error, so the boolean
    // result cannot be false here.
    render_seq(items, rec, &mut out, false)?;
    split_path(&out)
}

/// Append `items` to `out`.
///
/// Returns `Ok(false)` when a placeholder resolved to nothing and `in_group`
/// is set, meaning the caller should discard what was written.
fn render_seq(
    items: &[Node],
    rec: &Recording,
    out: &mut String,
    in_group: bool,
) -> Result<bool, RenderError> {
    for item in items {
        match item {
            Node::Text(s) => out.push_str(s),

            Node::Placeholder { alts, source, .. } => match resolve(alts, rec) {
                Some(v) => out.push_str(&sanitize_value(&v)),
                None if in_group => return Ok(false),
                None => {
                    return Err(RenderError::Unresolved {
                        placeholder: source.clone(),
                    })
                }
            },

            Node::Group { items, .. } => {
                // Render into a scratch buffer so a later failure inside the
                // group leaves nothing behind. An omitted *inner* group is not
                // a failure of the outer one, so the result is not propagated.
                let mut buf = String::new();
                if render_seq(items, rec, &mut buf, true)? {
                    out.push_str(&buf);
                }
            }
        }
    }
    Ok(true)
}

/// First alternative that yields a value, left to right.
fn resolve(alts: &[Alt], rec: &Recording) -> Option<String> {
    for alt in alts {
        match alt {
            Alt::Literal(l) => return Some(l.clone()),
            Alt::Field { path, .. } => {
                // The parser validated every path, so an Err is impossible;
                // treat it as absent rather than panicking.
                if let Ok(Some(v)) = rec.get(path) {
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
pub fn sanitize_value(value: &str) -> String {
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

/// Split a rendered template into path segments, rejecting anything that would
/// escape a destination directory.
pub fn split_path(rendered: &str) -> Result<Vec<String>, RenderError> {
    let bad = |reason: &str| RenderError::BadPath {
        rendered: rendered.to_string(),
        reason: reason.to_string(),
    };
    if rendered.starts_with('/') {
        return Err(bad("template renders an absolute path"));
    }
    let mut segments = Vec::new();
    for raw in rendered.split('/') {
        let seg = raw.trim();
        if seg.is_empty() {
            continue; // tolerate `//` and a trailing `/`, e.g. from an omitted group
        }
        if seg == "." || seg == ".." {
            return Err(bad("template renders a `.` or `..` path segment"));
        }
        // Trailing dots and spaces are silently dropped by some filesystems,
        // which would make the result unstable.
        let seg = seg.trim_end_matches([' ', '.']).to_string();
        if seg.is_empty() {
            return Err(bad("template renders an empty path segment"));
        }
        segments.push(seg);
    }
    if segments.is_empty() {
        return Err(bad("template renders an empty path"));
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slash_inside_a_value_cannot_create_a_directory_level() {
        assert_eq!(
            sanitize_value("Cavalleria/Pagliacci"),
            "Cavalleria-Pagliacci"
        );
        assert_eq!(sanitize_value("back\\slash"), "back-slash");
    }

    #[test]
    fn strips_control_characters_and_surrounding_space() {
        assert_eq!(sanitize_value("  Aida\u{7}  "), "Aida");
    }

    #[test]
    fn a_dot_only_value_cannot_become_a_traversal_segment() {
        assert_eq!(sanitize_value(".."), "__");
        assert_eq!(sanitize_value("."), "_");
    }

    #[test]
    fn rejects_paths_that_escape() {
        for bad in ["../etc/passwd", "/etc/passwd", "a/../../b", "."] {
            assert!(split_path(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn tolerates_separators_left_behind_by_an_omitted_group() {
        assert_eq!(
            split_path("Britten//Billy Budd/").unwrap(),
            vec!["Britten", "Billy Budd"]
        );
    }

    #[test]
    fn trims_trailing_dots_and_spaces_that_filesystems_would_drop() {
        assert_eq!(
            split_path("Strauss /Salome.").unwrap(),
            vec!["Strauss", "Salome"]
        );
    }

    #[test]
    fn rejects_an_empty_render() {
        assert!(split_path("").is_err());
        assert!(split_path("///").is_err());
    }
}
