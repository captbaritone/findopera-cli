//! Discovery of FindOpera marker files on disk.
//!
//! A marker is any `.txt` file containing a `findopera.com/recording/<id>` URL
//! — which is exactly what `https://findopera.com/recording/<id>.txt` serves,
//! so the intended workflow is "download the .txt into the recording's folder".
//! A directory may hold several markers (a box set covering several operas), in
//! which case it is linked once per recording.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct Marker {
    /// The `.txt` file itself.
    pub marker_path: PathBuf,
    /// The directory that gets linked — the marker's parent.
    pub recording_dir: PathBuf,
    pub recording_id: String,
}

/// Extract every `findopera.com/recording/<id>` id from a marker's text.
///
/// Hand-rolled rather than a regex dependency: scan for the host, then the
/// path prefix, then take the digit run.
fn extract_ids(text: &str) -> Vec<String> {
    const HOST: &str = "findopera.com/recording/";
    let mut ids = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(HOST) {
        let after = &rest[pos + HOST.len()..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            // Trim leading zeros so `075` and `75` don't produce two links.
            let normalized = digits.trim_start_matches('0');
            ids.push(if normalized.is_empty() {
                "0".to_string()
            } else {
                normalized.to_string()
            });
        }
        rest = &rest[pos + HOST.len()..];
    }
    ids
}

#[derive(Debug)]
pub struct ScanReport {
    pub markers: Vec<Marker>,
    /// `.txt` files that were read but held no FindOpera URL.
    pub skipped: Vec<PathBuf>,
    /// Paths that could not be read, with the reason.
    pub unreadable: Vec<(PathBuf, String)>,
}

/// Walk `sources`, following no symlinks, collecting markers.
pub fn scan(sources: &[PathBuf], follow_links: bool) -> ScanReport {
    let mut markers = Vec::new();
    let mut skipped = Vec::new();
    let mut unreadable = Vec::new();
    // One directory can legitimately contain two markers for the same
    // recording (say, a .txt and a renamed copy); link it only once.
    let mut seen: BTreeSet<(PathBuf, String)> = BTreeSet::new();

    for source in sources {
        let walker = WalkDir::new(source)
            .follow_links(follow_links)
            .sort_by_file_name();
        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    let path = e.path().unwrap_or(Path::new("?")).to_path_buf();
                    unreadable.push((path, e.to_string()));
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
            {
                continue;
            }
            let text = match std::fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => {
                    unreadable.push((path.to_path_buf(), e.to_string()));
                    continue;
                }
            };
            let ids = extract_ids(&text);
            if ids.is_empty() {
                skipped.push(path.to_path_buf());
                continue;
            }
            let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
            for id in ids {
                if seen.insert((dir.clone(), id.clone())) {
                    markers.push(Marker {
                        marker_path: path.to_path_buf(),
                        recording_dir: dir.clone(),
                        recording_id: id,
                    });
                }
            }
        }
    }
    ScanReport {
        markers,
        skipped,
        unreadable,
    }
}

#[cfg(test)]
mod tests {
    use super::extract_ids;

    #[test]
    fn finds_the_canonical_trailing_url() {
        let txt = "      SOSARME\n\nConductor: X\n\n   https://findopera.com/recording/10655   ";
        assert_eq!(extract_ids(txt), vec!["10655"]);
    }

    #[test]
    fn accepts_the_slug_form_and_bare_host() {
        let txt = "https://findopera.com/recording/75/billy-budd-1967-britten\n\
                   http://www.findopera.com/recording/500";
        assert_eq!(extract_ids(txt), vec!["75", "500"]);
    }

    #[test]
    fn normalizes_leading_zeros_so_links_do_not_duplicate() {
        assert_eq!(extract_ids("findopera.com/recording/075"), vec!["75"]);
    }

    #[test]
    fn ignores_text_without_a_recording_url() {
        assert!(extract_ids("Ripped from CD in 2019. See findopera.com").is_empty());
        assert!(extract_ids("findopera.com/opera/93").is_empty());
    }
}
