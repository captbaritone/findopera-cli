//! Finding FindOpera marker files on disk.
//!
//! A marker is a `.txt` file whose *name* carries a recording id. What it
//! identifies is its parent directory — the folder holding the recording — so
//! the workflow is to save `https://findopera.com/recording/<id>.txt` into
//! each recording's folder. One directory can hold several markers, a box set
//! covering several operas, in which case it stands for each of them.
//!
//! # The name is the whole convention
//!
//! Two spellings count, and both are things you actually end up with:
//!
//! ```text
//! 10655.txt                                          curl -O, wget
//! Sosarme, Re di Media-2026 [findopera-10655].txt    the site's suggested name
//! ```
//!
//! # Telling two rips apart
//!
//! Anything left in the name after the id is a *variant*: a word the person
//! organising the library chose to separate two copies of one recording.
//!
//! ```text
//! 332 flac.txt        variant "flac"
//! 332 mp3.txt         variant "mp3"
//! ```
//!
//! This is the only thing here that the recording metadata cannot supply. Two
//! rips of one performance *are* one recording, so no template can separate
//! them from the data alone — but the person who has both knows what makes
//! them different, and the filename is where they can say so.
//!
//! The `findopera-<id>` token is what is matched, not the brackets around it,
//! so a title containing brackets cannot confuse it and the delimiter can
//! change without invalidating anything already on disk.
//!
//! Nothing here opens a file. Deciding by content would mean reading every
//! `.txt` in the library to discover that almost none of them are markers —
//! on a 12,500-file tree that is three and a half times the work, nearly all
//! of it wasted, and far worse over a network mount where opening a file
//! costs so much more than listing one. It also makes the contents load
//! bearing: as it is, the file can be empty, and `touch '10655.txt'` is a
//! perfectly good marker.

use crate::FieldDoc;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct Marker {
    /// The `.txt` file the id was read from.
    pub marker_path: PathBuf,
    /// The directory holding it — what the id actually names.
    pub directory: PathBuf,
    pub id: String,
    /// Whatever the name carries after the id, for telling two rips apart.
    pub variant: Option<String>,
}

/// The one field that comes from the marker rather than the recording.
///
/// Nullable, and deliberately so: most markers carry no variant, which means a
/// template has to wrap it in a group — `[ ({{variant}})]` — and that group
/// disappears for every recording with only one copy.
pub const VARIANT: FieldDoc = FieldDoc::new(
    "variant",
    "Whatever the marker's filename carries after the id, e.g. `332 flac.txt`",
);

#[derive(Debug, Default)]
pub struct Report {
    pub markers: Vec<Marker>,
    /// Paths that could not be walked, with the reason.
    pub unreadable: Vec<(PathBuf, String)>,
}

/// Normalize a digit run to an id, or `None` if it is not one.
///
/// Leading zeros are trimmed so `075` and `75` are not two recordings.
fn normalize(digits: &str) -> Option<String> {
    if digits.is_empty() {
        return None;
    }
    let trimmed = digits.trim_start_matches('0');
    Some(if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    })
}

/// The id a file's name carries, if it follows the convention.
///
/// Accepts a stem that is nothing but the id, which is what `curl -O` and
/// `wget` produce from the URL, and a `findopera-<id>` token anywhere in the
/// stem, which is what the site's suggested filename carries.
fn id_from_name(path: &Path) -> Option<(String, Option<String>)> {
    let stem = path.file_stem()?.to_str()?;
    // A stem that opens with digits: the id, then anything else is a variant.
    let digits: String = stem.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        return normalize(&digits).map(|id| (id, variant_after(&stem[digits.len()..])));
    }
    const TOKEN: &str = "findopera-";
    let mut at = 0;
    while let Some(pos) = stem[at..].find(TOKEN) {
        let start = at + pos;
        at = start + TOKEN.len();
        // The token has to start here, not merely end here, or
        // `notfindopera-75.txt` reads as recording 75.
        let starts_token = stem[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        if !starts_token {
            continue;
        }
        let digits: String = stem[at..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Some(id) = normalize(&digits) {
            return Some((id, variant_after(&stem[at + digits.len()..])));
        }
    }
    None
}

/// What is left of the stem once the id has been read off it.
///
/// The delimiters around the id are not part of the variant, whatever they
/// are: `[findopera-332] flac`, `332 flac` and `332-flac` all give `flac`.
fn variant_after(rest: &str) -> Option<String> {
    let v = rest.trim_matches(|c: char| !c.is_alphanumeric());
    (!v.is_empty()).then(|| v.to_string())
}

/// Walk `roots` depth-first, collecting markers.
///
/// Symlinks are not followed by default: a library organised with symlinks
/// would otherwise report the same recording once per link, and a cycle would
/// not terminate.
pub fn scan(roots: &[PathBuf], follow_links: bool) -> Report {
    let mut report = Report::default();
    // The same directory can hold two files naming one recording — a marker
    // and a renamed copy. Report it once, unless they carry different
    // variants, which is the caller saying they are different rips.
    let mut seen: BTreeSet<(PathBuf, String, Option<String>)> = BTreeSet::new();

    for root in roots {
        for entry in WalkDir::new(root)
            .follow_links(follow_links)
            .sort_by_file_name()
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    let path = e.path().unwrap_or(Path::new("?")).to_path_buf();
                    report.unreadable.push((path, e.to_string()));
                    continue;
                }
            };
            let path = entry.path();
            if !entry.file_type().is_file()
                || !path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
            {
                continue;
            }
            let Some((id, variant)) = id_from_name(path) else {
                continue;
            };
            let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
            if seen.insert((dir.clone(), id.clone(), variant.clone())) {
                report.markers.push(Marker {
                    marker_path: path.to_path_buf(),
                    directory: dir,
                    id,
                    variant,
                });
            }
        }
    }
    report
}
