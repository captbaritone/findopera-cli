//! A small `{{ }}` template renderer for canonical paths.
//!
//! Syntax is deliberately narrow: a placeholder is a `|`-separated list of
//! alternatives tried left to right, where each alternative is either a field
//! path (`opera.title`) or a quoted literal (`"Unknown"`). The first one that
//! resolves to a present value wins.
//!
//!     {{composer.lastName}}/{{opera.englishTitle|opera.title}}/{{year|"n.d."}}
//!
//! Interpolated values are sanitized for the filesystem; `/` in the *template*
//! stays a directory separator, but a `/` inside an opera title does not.

use crate::model::Recording;

#[derive(Debug)]
pub enum TemplateError {
    UnclosedPlaceholder {
        at: usize,
    },
    EmptyPlaceholder {
        at: usize,
    },
    UnknownField {
        path: String,
    },
    UnterminatedLiteral {
        at: usize,
    },
    /// Every alternative resolved to absent and no literal fallback was given.
    Unresolved {
        placeholder: String,
    },
    /// Rendering produced a path that is empty or escapes the destination.
    BadPath {
        rendered: String,
        reason: String,
    },
}

enum Part {
    Literal(String),
    Placeholder { alts: Vec<Alt>, source: String },
}

enum Alt {
    Field(String),
    Literal(String),
}

pub struct Template {
    parts: Vec<Part>,
}

impl Template {
    /// Parse a template, rejecting unknown field paths up front so a bad
    /// `--template` fails before any directory is touched.
    pub fn parse(input: &str) -> Result<Self, TemplateError> {
        let mut parts = Vec::new();
        let mut rest = input;
        let mut offset = 0usize;

        while let Some(start) = rest.find("{{") {
            if start > 0 {
                parts.push(Part::Literal(rest[..start].to_string()));
            }
            let after = &rest[start + 2..];
            let end = after
                .find("}}")
                .ok_or(TemplateError::UnclosedPlaceholder { at: offset + start })?;
            let body = &after[..end];
            let source = format!("{{{{{body}}}}}");

            let mut alts = Vec::new();
            for raw in body.split('|') {
                let alt = raw.trim();
                if alt.is_empty() {
                    return Err(TemplateError::EmptyPlaceholder { at: offset + start });
                }
                if let Some(stripped) = alt.strip_prefix('"') {
                    let lit = stripped
                        .strip_suffix('"')
                        .ok_or(TemplateError::UnterminatedLiteral { at: offset + start })?;
                    alts.push(Alt::Literal(lit.to_string()));
                } else {
                    if !Recording::is_known(alt) {
                        return Err(TemplateError::UnknownField {
                            path: alt.to_string(),
                        });
                    }
                    alts.push(Alt::Field(alt.to_string()));
                }
            }
            if alts.is_empty() {
                return Err(TemplateError::EmptyPlaceholder { at: offset + start });
            }
            parts.push(Part::Placeholder { alts, source });

            let consumed = start + 2 + end + 2;
            offset += consumed;
            rest = &rest[consumed..];
        }
        if !rest.is_empty() {
            parts.push(Part::Literal(rest.to_string()));
        }
        Ok(Template { parts })
    }

    /// Render to a relative path. Returns the path split into segments.
    pub fn render(&self, rec: &Recording) -> Result<Vec<String>, TemplateError> {
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(s) => out.push_str(s),
                Part::Placeholder { alts, source } => {
                    let mut resolved = None;
                    for alt in alts {
                        match alt {
                            Alt::Literal(l) => {
                                resolved = Some(l.clone());
                                break;
                            }
                            Alt::Field(path) => {
                                // `parse` already validated the path, so an
                                // error here is impossible; treat it as absent.
                                if let Ok(Some(v)) = rec.get(path) {
                                    resolved = Some(v);
                                    break;
                                }
                            }
                        }
                    }
                    let value = resolved.ok_or_else(|| TemplateError::Unresolved {
                        placeholder: source.clone(),
                    })?;
                    out.push_str(&sanitize_value(&value));
                }
            }
        }
        split_path(&out)
    }
}

/// Make one interpolated value safe to sit inside a single path segment.
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
    // A value that is entirely separators would vanish and collapse the path.
    if s.chars().all(|c| c == '.' || c == '-' || c.is_whitespace()) && !s.is_empty() {
        s = s.replace('.', "_");
    }
    s
}

/// Split a rendered template into segments, rejecting anything that would
/// escape the destination root.
fn split_path(rendered: &str) -> Result<Vec<String>, TemplateError> {
    let bad = |reason: &str| TemplateError::BadPath {
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
            continue; // tolerate `//` and a trailing `/`
        }
        if seg == "." || seg == ".." {
            return Err(bad("template renders a `.` or `..` path segment"));
        }
        // Trailing dots and spaces are silently stripped by some filesystems,
        // which would make the tree non-idempotent.
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

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnclosedPlaceholder { at } => {
                write!(
                    f,
                    "unclosed `{{{{` at byte {at} — every `{{{{` needs a matching `}}}}`"
                )
            }
            Self::EmptyPlaceholder { at } => {
                write!(f, "empty placeholder at byte {at}")
            }
            Self::UnterminatedLiteral { at } => {
                write!(f, "unterminated quoted literal at byte {at}")
            }
            Self::UnknownField { path } => {
                write!(f, "unknown field `{path}`")
            }
            Self::Unresolved { placeholder } => write!(
                f,
                "{placeholder} resolved to nothing for this recording — \
                 add a fallback, e.g. {{{{…|\"Unknown\"}}}}"
            ),
            Self::BadPath { rendered, reason } => {
                write!(f, "{reason} (rendered: {rendered:?})")
            }
        }
    }
}

impl TemplateError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnclosedPlaceholder { .. }
            | Self::EmptyPlaceholder { .. }
            | Self::UnterminatedLiteral { .. } => "template_syntax_error",
            Self::UnknownField { .. } => "template_unknown_field",
            Self::Unresolved { .. } => "template_unresolved_field",
            Self::BadPath { .. } => "template_bad_path",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{sanitize_value, split_path, Template, TemplateError};

    #[test]
    fn rejects_unknown_fields_at_parse_time() {
        assert!(matches!(
            Template::parse("{{composer.surname}}"),
            Err(TemplateError::UnknownField { .. })
        ));
        assert!(Template::parse("{{composer.lastName}}").is_ok());
    }

    #[test]
    fn rejects_malformed_syntax() {
        assert!(matches!(
            Template::parse("{{opera.title"),
            Err(TemplateError::UnclosedPlaceholder { .. })
        ));
        assert!(matches!(
            Template::parse("{{}}"),
            Err(TemplateError::EmptyPlaceholder { .. })
        ));
        assert!(matches!(
            Template::parse("{{opera.title|\"oops}}"),
            Err(TemplateError::UnterminatedLiteral { .. })
        ));
    }

    #[test]
    fn accepts_alternatives_and_quoted_literals() {
        assert!(Template::parse("{{opera.englishTitle|opera.title|\"Untitled\"}}").is_ok());
    }

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
    fn rejects_paths_that_escape_the_destination() {
        for bad in ["../etc/passwd", "/etc/passwd", "a/../../b", "."] {
            assert!(
                split_path(bad).is_err(),
                "{bad:?} should be rejected as a path"
            );
        }
    }

    #[test]
    fn tolerates_redundant_and_trailing_separators() {
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
