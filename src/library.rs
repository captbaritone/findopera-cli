//! Building the canonical tree.
//!
//! The destination is treated as fully derived from the markers: a sync wipes
//! it and rebuilds. To keep that from ever being aimed at a directory the user
//! cares about, an applied destination carries a stamp file, and a non-empty
//! destination without one is refused unless `--force` is passed.

use crate::model::Recording;
use crate::scan::Marker;
use crate::template::Template;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Marks a directory as owned by this tool and therefore safe to wipe.
pub const STAMP: &str = ".findopera-library.json";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// Path relative to the destination root.
    pub path: String,
    /// Absolute path to the recording directory the link points at.
    pub target: String,
    pub recording_id: String,
    /// The marker file this link came from.
    pub marker: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub path: String,
    pub recording_ids: Vec<String>,
    pub targets: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub destination: String,
    pub template: String,
    /// Links that would be created, sorted by path.
    pub links: Vec<Link>,
    /// Two markers that render to the same canonical path.
    pub conflicts: Vec<Conflict>,
    /// Markers that could not be turned into a link.
    pub problems: Vec<Problem>,
    pub summary: Summary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub markers_found: usize,
    pub links_planned: usize,
    pub conflicts: usize,
    pub problems: usize,
    /// `.txt` files that held no FindOpera URL.
    pub txt_files_skipped: usize,
}

/// Turn markers plus fetched recordings into a plan. Never touches the disk.
pub fn plan(
    destination: &Path,
    template_source: &str,
    template: &Template,
    markers: &[Marker],
    recordings: &BTreeMap<String, Recording>,
    txt_files_skipped: usize,
) -> Plan {
    // Keyed by rendered path so collisions surface instead of one link
    // silently overwriting another.
    let mut by_path: BTreeMap<String, Vec<Link>> = BTreeMap::new();
    let mut problems = Vec::new();

    for marker in markers {
        let marker_display = marker.marker_path.display().to_string();
        let Some(rec) = recordings.get(&marker.recording_id) else {
            problems.push(Problem {
                error: "recording_not_found".to_string(),
                message: format!(
                    "recording {} is not in the FindOpera database",
                    marker.recording_id
                ),
                recording_id: Some(marker.recording_id.clone()),
                marker: Some(marker_display),
            });
            continue;
        };
        let segments = match template.render(rec) {
            Ok(s) => s,
            Err(e) => {
                problems.push(Problem {
                    error: e.code().to_string(),
                    message: e.to_string(),
                    recording_id: Some(marker.recording_id.clone()),
                    marker: Some(marker_display),
                });
                continue;
            }
        };
        let path = segments.join("/");
        let target = match marker.recording_dir.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                problems.push(Problem {
                    error: "unreadable_source".to_string(),
                    message: format!("could not resolve {}: {e}", marker.recording_dir.display()),
                    recording_id: Some(marker.recording_id.clone()),
                    marker: Some(marker_display),
                });
                continue;
            }
        };
        by_path.entry(path.clone()).or_default().push(Link {
            path,
            target: target.display().to_string(),
            recording_id: marker.recording_id.clone(),
            marker: marker_display,
        });
    }

    let mut links = Vec::new();
    let mut conflicts = Vec::new();
    for (path, mut group) in by_path {
        // Two markers in the same directory for the same recording already
        // collapsed during the scan; anything left is a real collision unless
        // both point at the same place.
        group.sort_by(|a, b| a.target.cmp(&b.target));
        group.dedup_by(|a, b| a.target == b.target);
        if group.len() == 1 {
            links.push(group.pop().expect("group is non-empty"));
        } else {
            conflicts.push(Conflict {
                path,
                recording_ids: group.iter().map(|l| l.recording_id.clone()).collect(),
                targets: group.iter().map(|l| l.target.clone()).collect(),
            });
        }
    }

    let summary = Summary {
        markers_found: markers.len(),
        links_planned: links.len(),
        conflicts: conflicts.len(),
        problems: problems.len(),
        txt_files_skipped,
    };
    Plan {
        destination: destination.display().to_string(),
        template: template_source.to_string(),
        links,
        conflicts,
        problems,
        summary,
    }
}

#[derive(Debug)]
pub enum ApplyError {
    /// The destination holds files this tool did not create.
    Unmanaged { path: PathBuf },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unmanaged { path } => write!(
                f,
                "{} is not empty and was not created by findopera",
                path.display()
            ),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

/// Is this destination safe to wipe? True if it is missing, empty, or stamped.
pub fn is_managed(destination: &Path) -> Result<bool, std::io::Error> {
    if !destination.exists() {
        return Ok(true);
    }
    if destination.join(STAMP).is_file() {
        return Ok(true);
    }
    Ok(destination.read_dir()?.next().is_none())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub destination: String,
    pub links_created: usize,
    pub directories_created: usize,
}

/// Wipe the destination and rebuild it from the plan.
pub fn apply(destination: &Path, plan: &Plan, force: bool) -> Result<ApplyResult, ApplyError> {
    if !force
        && !is_managed(destination).map_err(|e| ApplyError::Io {
            path: destination.to_path_buf(),
            source: e,
        })?
    {
        return Err(ApplyError::Unmanaged {
            path: destination.to_path_buf(),
        });
    }

    if destination.exists() {
        std::fs::remove_dir_all(destination).map_err(|e| ApplyError::Io {
            path: destination.to_path_buf(),
            source: e,
        })?;
    }
    std::fs::create_dir_all(destination).map_err(|e| ApplyError::Io {
        path: destination.to_path_buf(),
        source: e,
    })?;

    // Write the stamp first: if the run dies midway the directory is still
    // recognizably ours, so the next run can wipe it without --force.
    let stamp_body = serde_json::json!({
        "tool": "findopera",
        "version": env!("CARGO_PKG_VERSION"),
        "template": plan.template,
        "warning": "This directory is rebuilt by `findopera library sync`. \
                    Anything added here by hand will be deleted.",
    });
    std::fs::write(destination.join(STAMP), format!("{stamp_body:#}\n")).map_err(|e| {
        ApplyError::Io {
            path: destination.join(STAMP),
            source: e,
        }
    })?;

    let mut directories_created = 0usize;
    let mut links_created = 0usize;
    for link in &plan.links {
        let full = destination.join(&link.path);
        if let Some(parent) = full.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| ApplyError::Io {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
                directories_created += 1;
            }
        }
        std::os::unix::fs::symlink(&link.target, &full).map_err(|e| ApplyError::Io {
            path: full.clone(),
            source: e,
        })?;
        links_created += 1;
    }

    Ok(ApplyResult {
        destination: destination.display().to_string(),
        links_created,
        directories_created,
    })
}
