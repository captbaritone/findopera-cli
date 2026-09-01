//! Turning markers and recordings into a naming plan.
//!
//! This is where the awkward part of the job lives, so it is worth stating
//! plainly. A template answers "what is this recording called", and two rips
//! of one performance are one recording — so they get one answer, and acting
//! on that would lose a directory. Nothing in the metadata separates them,
//! because there is nothing to separate: only the person holding both knows
//! what makes them different, which is what a marker's variant is for.
//!
//! So the rules are:
//!
//! - a variant the marker declares is always used, clash or no clash
//! - where none was declared and two directories still want one name, they
//!   are numbered — but only then
//!
//! A number taken from walk order is not a name: adding a third rip that
//! sorts first renumbers the two that were already there. So the plan says
//! when it has done this. Whether that is worth stopping for depends on what
//! you are doing — regenerating a view wholesale, it does not matter; renaming
//! in place, it matters a great deal — so it is a mode rather than a verdict,
//! and `strict` is how the caller says which.
//!
//! Keeping this out of the binary is what lets it be tested against canned
//! recordings instead of the live API.

use crate::model::Recording;
use crate::scan::Marker;
use crate::{to_path, Fields, Template};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// One directory and the name it should have.
#[derive(Debug)]
pub struct Row<'a> {
    pub marker: &'a Marker,
    pub path: String,
    /// A number stood in because no variant was declared and this clashed.
    pub derived: Option<String>,
}

#[derive(Debug)]
pub enum Problem<'a> {
    /// The id is not in the database.
    Missing { marker: &'a Marker },
    /// The template rendered something that is not a usable relative path.
    Unusable { marker: &'a Marker, reason: String },
    /// Numbered by walk order, which will not survive the library changing.
    ///
    /// Whether this blocks is the caller's call; see [`Plan::report`].
    Numbered { markers: Vec<&'a Marker> },
    /// Still sharing one name after numbering.
    Clash {
        path: String,
        markers: Vec<&'a Marker>,
        cause: Cause,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Cause {
    /// The markers name the same variant, so they say the same thing.
    SameVariant,
    /// The template never mentions `{{variant}}`, so nothing can differ.
    TemplateIgnoresVariant,
}

#[derive(Debug, Default)]
pub struct Plan<'a> {
    pub rows: Vec<Row<'a>>,
    pub problems: Vec<Problem<'a>>,
}

/// A recording plus the variant its marker carried.
///
/// The template sees one field set; where each field comes from is this
/// wrapper's business. `variant` is nullable, so it only ever arrives through
/// `optional` and the generated model never has to know it exists.
struct Marked<'a> {
    recording: &'a Recording,
    variant: Option<&'a str>,
}

impl Fields for Marked<'_> {
    fn required(&self, path: &str) -> String {
        self.recording.required(path)
    }
    fn optional(&self, path: &str) -> Option<String> {
        if path == crate::scan::VARIANT.path {
            return self.variant.map(str::to_string);
        }
        self.recording.optional(path)
    }
}

pub fn plan<'a>(
    markers: &'a [Marker],
    recordings: &BTreeMap<String, Recording>,
    tmpl: &Template,
) -> Plan<'a> {
    let mut out = Plan::default();

    let render = |marker: &Marker, variant: Option<&str>| -> Result<String, Option<String>> {
        let Some(recording) = recordings.get(&marker.id) else {
            return Err(None);
        };
        // Rendering cannot fail; only judging the result as a path can.
        let rendered = tmpl.render(&Marked { recording, variant });
        to_path(&rendered)
            .map(|segments| segments.join("/"))
            .map_err(|e| Some(e.to_string()))
    };

    for marker in markers {
        match render(marker, marker.variant.as_deref()) {
            Ok(path) => out.rows.push(Row {
                marker,
                path,
                derived: None,
            }),
            Err(None) => out.problems.push(Problem::Missing { marker }),
            Err(Some(reason)) => out.problems.push(Problem::Unusable { marker, reason }),
        }
    }

    // Number every directory in a clash, not all but the first: a lone `(2)`
    // with no `(1)` reads as an anomaly, where `(1)` and `(2)` read as what
    // they are — two copies of one recording.
    //
    // A number must not land on a name something else already has. A marker
    // declaring the variant `2` is not in this clash — it renders differently
    // — but numbering a neighbour `2` would walk straight into it, so every
    // name not being renumbered is counted as taken first.
    let groups = clashing(&out.rows);
    let renumbering: BTreeSet<usize> = groups
        .iter()
        .flatten()
        .copied()
        .filter(|&i| out.rows[i].marker.variant.is_none())
        .collect();
    let mut taken: BTreeSet<String> = out
        .rows
        .iter()
        .enumerate()
        .filter(|(i, _)| !renumbering.contains(i))
        .map(|(_, r)| r.path.clone())
        .collect();

    for &i in &renumbering {
        let original = out.rows[i].path.clone();
        // At most one number per row can be taken, so a free one is always
        // within reach for a template that distinguishes them at all.
        for n in 1..=out.rows.len() + 1 {
            let derived = n.to_string();
            let Ok(path) = render(out.rows[i].marker, Some(&derived)) else {
                break;
            };
            // A template that never mentions `{{variant}}` renders the same
            // string whatever it is given. Saying these were numbered would
            // then be a lie, and the clash below is the honest report.
            if path == original {
                break;
            }
            if !taken.contains(&path) {
                taken.insert(path.clone());
                out.rows[i].path = path;
                out.rows[i].derived = Some(derived);
                break;
            }
        }
    }

    let numbered: Vec<&Marker> = out
        .rows
        .iter()
        .filter(|r| r.derived.is_some())
        .map(|r| r.marker)
        .collect();
    if !numbered.is_empty() {
        out.problems.push(Problem::Numbered { markers: numbered });
    }

    for group in clashing(&out.rows) {
        // Why numbering could not separate these matters: either the markers
        // say the same thing, or the template never asks what they say.
        // Blaming the template for the first would send the reader to the
        // wrong file.
        let cause = if group.iter().all(|&i| out.rows[i].marker.variant.is_some()) {
            Cause::SameVariant
        } else {
            Cause::TemplateIgnoresVariant
        };
        out.problems.push(Problem::Clash {
            path: out.rows[group[0]].path.clone(),
            markers: group.iter().map(|&i| out.rows[i].marker).collect(),
            cause,
        });
    }
    out
}

/// Indices of rows sharing a rendered path, grouped, in a stable order.
fn clashing(rows: &[Row]) -> Vec<Vec<usize>> {
    let mut by_path: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() {
        by_path.entry(&r.path).or_default().push(i);
    }
    by_path.into_values().filter(|g| g.len() > 1).collect()
}

impl Plan<'_> {
    /// The listing, one line per directory.
    ///
    /// Padded into two columns unless `tabs`, which is for feeding another
    /// program.
    pub fn listing(&self, tabs: bool) -> Vec<String> {
        let width = if tabs {
            0
        } else {
            self.rows
                .iter()
                .map(|r| r.marker.directory.display().to_string().chars().count())
                .max()
                .unwrap_or(0)
        };
        self.rows
            .iter()
            .map(|r| {
                let dir = r.marker.directory.display().to_string();
                if tabs {
                    format!("{dir}\t{}", r.path)
                } else {
                    format!("{dir:<width$}  {}", r.path)
                }
            })
            .collect()
    }

    /// Everything that wants saying about the plan, one line at a time.
    ///
    /// Under `strict`, numbering is spelled out marker by marker, because the
    /// caller has said they intend to fix every one. Otherwise it is a single
    /// line with one example: a library can easily have hundreds of duplicate
    /// rips, and a wall of `mv` is not a warning anybody reads.
    pub fn report(&self, strict: bool) -> Vec<String> {
        let mut out = Vec::new();
        for problem in &self.problems {
            match problem {
                Problem::Missing { marker } => out.push(format!(
                    "recording {} is not in the FindOpera database (from {})",
                    marker.id,
                    marker.marker_path.display()
                )),
                Problem::Unusable { marker, reason } => {
                    out.push(format!("recording {} {reason}", marker.id));
                }
                Problem::Numbered { markers } => {
                    if strict {
                        out.push(format!(
                            "{} directories would take the same name and were numbered by walk \
                             order. Write a word into each marker instead — one you choose, \
                             which `{{{{variant}}}}` then picks up:",
                            markers.len()
                        ));
                        for m in markers {
                            out.push(format!("    {}", suggest_rename(m)));
                        }
                    } else {
                        out.push(format!(
                            "{} directories were numbered by walk order because no variant was \
                             declared; those numbers shift as the library changes. Write a word \
                             into each marker to fix them — {} — or pass --require-variants to \
                             make this an error.",
                            markers.len(),
                            suggest_rename(markers[0])
                        ));
                    }
                }
                Problem::Clash {
                    path,
                    markers,
                    cause,
                } => {
                    out.push(format!(
                        "{} directories want the name {path:?}:",
                        markers.len()
                    ));
                    for m in markers {
                        // The marker, not just its directory: the fix is to
                        // rename or re-word one of these files, and the
                        // directory alone does not say which file that is. It
                        // is the marker's parent, so nothing is lost.
                        out.push(format!("    {}", m.marker_path.display()));
                    }
                    out.push(match cause {
                        Cause::SameVariant => "    ^ these markers declare the same variant; \
                                               give one a different word"
                            .to_string(),
                        Cause::TemplateIgnoresVariant => "    ^ the template has no \
                                                          `{{variant}}` for them to differ in"
                            .to_string(),
                    });
                }
            }
        }
        out
    }

    /// Is there anything here that should stop the caller acting on the plan?
    ///
    /// A numbered plan is *complete* — every directory has a distinct name and
    /// it can be acted on. It is only the names' durability that is in doubt,
    /// which is why it blocks under `strict` and not otherwise. A clash is a
    /// different thing: two directories share one name, and acting on it loses
    /// one of them.
    pub fn blocked(&self, strict: bool) -> bool {
        self.problems.iter().any(|p| match p {
            Problem::Numbered { .. } => strict,
            _ => true,
        })
    }

    /// Is there anything worth printing at all?
    pub fn has_problems(&self) -> bool {
        !self.problems.is_empty()
    }

    /// Directories in the plan, for a caller that wants them without the text.
    pub fn directories(&self) -> impl Iterator<Item = &PathBuf> {
        self.rows.iter().map(|r| &r.marker.directory)
    }
}

/// The `mv` that would give one marker a durable variant.
fn suggest_rename(m: &Marker) -> String {
    let p = &m.marker_path;
    let stem = p.file_stem().unwrap_or_default().to_string_lossy();
    format!(
        "mv {} {}",
        quote(&p.display().to_string()),
        quote(&format!(
            "{}/{stem} <word>.txt",
            p.parent().unwrap_or(Path::new(".")).display()
        ))
    )
}

/// Quote a path for a shell, so a suggested `mv` can be pasted as it stands.
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
