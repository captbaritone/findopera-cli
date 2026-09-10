//! A list of the collection, left at the destination for someone to read.
//!
//! The tree can only be organised one way. A template chooses one hierarchy
//! and necessarily hides every other: a library filed by composer says
//! nothing about who sang, and no amount of browsing will tell you whether
//! there is a Tosca in it without knowing that Tosca is Puccini's.
//!
//! So a run leaves a file naming everything it built, which is the one place
//! the whole collection can be read at once. It is written and never read
//! back — unlike the state file, which is a record this program depends on,
//! this is only an answer to somebody's question and can be deleted, edited
//! or regenerated with no consequence at all.
//!
//! One line per recording rather than per folder. Two rips of one performance
//! are two folders in the tree, because they are two things on a disk, but
//! they are one recording to somebody asking whether you have it — so the
//! line is written once and names both.

use crate::model::{Recording, FIELDS};
use crate::plan::Plan;
use crate::{FieldDoc, Fields, Template, TemplateError};
use std::collections::BTreeMap;

/// Every variant of this recording the library holds, comma-joined.
pub const VARIANTS: FieldDoc = FieldDoc::new(
    "variants",
    "Every rip of this recording the library holds, e.g. `flac, mp3`",
);

/// Where it sits in the destination.
pub const PATH: FieldDoc = FieldDoc::non_null(
    "path",
    "Where the folder sits inside the destination, so a line says where to go",
);

/// What a line of the index may name.
///
/// The recording's own fields, and two the index adds: a folder has one
/// variant and a recording may have several, and only a built tree knows
/// where anything ended up.
pub fn schema() -> Vec<FieldDoc> {
    let mut fields: Vec<FieldDoc> = FIELDS.to_vec();
    fields.push(VARIANTS);
    fields.push(PATH);
    fields
}

/// Check a template against what a line may name, without a recording in hand.
pub fn parse(template: &str) -> Result<Template, TemplateError> {
    Template::parse(template, &schema())
}

/// How many recordings the list names.
pub const COUNT: FieldDoc = FieldDoc::non_null("count", "How many recordings the list names");

/// The day it was written, as `2026-09-10`.
pub const DATE: FieldDoc =
    FieldDoc::non_null("date", "The day the list was written, as `2026-09-10`");

/// What the note at the top of a list may name.
///
/// Two fields, and neither is a recording's: a header is written once, above
/// everything, and the only things it can say that a person could not write
/// by hand are how many there are and when. Those are also the two that go
/// stale silently, which is the whole reason to have them.
pub fn header_schema() -> Vec<FieldDoc> {
    vec![COUNT, DATE]
}

/// Check a header against what it may name.
pub fn parse_header(template: &str) -> Result<Template, TemplateError> {
    Template::parse(template, &header_schema())
}

/// What a header is rendered against.
struct Summary {
    count: usize,
    date: String,
}

impl Fields for Summary {
    fn required(&self, path: &str) -> String {
        self.optional(path).unwrap_or_default()
    }
    fn optional(&self, path: &str) -> Option<String> {
        match path {
            p if p == COUNT.path => Some(self.count.to_string()),
            p if p == DATE.path => Some(self.date.clone()),
            _ => None,
        }
    }
}

/// Today, as `2026-09-10`.
///
/// Worked out here rather than taken from a crate: it is one date, in one
/// format, and the algorithm for it is older than any of the crates that
/// would do it. Days since the epoch to a civil date, after Howard Hinnant.
fn today() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (seconds / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// One recording, however many folders it turned into.
struct Listed<'a> {
    recording: &'a Recording,
    variants: Option<String>,
    path: String,
}

impl Fields for Listed<'_> {
    fn required(&self, path: &str) -> String {
        if path == PATH.path {
            return self.path.clone();
        }
        self.recording.required(path)
    }

    fn optional(&self, path: &str) -> Option<String> {
        if path == VARIANTS.path {
            return self.variants.clone();
        }
        if path == PATH.path {
            return Some(self.path.clone());
        }
        self.recording.optional(path)
    }
}

/// How a line sorts, so that a name is where somebody would look for it.
///
/// Case and accents come off. A byte sort puts `d'Albert` after `Zimmermann`
/// and `Ädam` after everything, which is exactly where nobody looks — the
/// list this one is modelled on has that fault, three lines from its end.
pub fn sort_key(line: &str) -> String {
    line.chars()
        .flat_map(|c| {
            let plain = match c {
                'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => {
                    'a'
                }
                'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
                'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
                'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => {
                    'o'
                }
                'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
                'ç' | 'Ç' => 'c',
                'ñ' | 'Ñ' => 'n',
                'ý' | 'ÿ' | 'Ý' => 'y',
                'š' | 'Š' => 's',
                'ž' | 'Ž' => 'z',
                'č' | 'Č' => 'c',
                'ř' | 'Ř' => 'r',
                'ł' | 'Ł' => 'l',
                other => other,
            };
            plain.to_lowercase()
        })
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect()
}

/// The whole file, as it will be written.
///
/// Built from the plan rather than from what reached the disk, because a row
/// that failed to build is still a recording the library has — the index
/// describes the collection, and the tree is what may be incomplete.
pub fn render(
    plan: &Plan,
    recordings: &BTreeMap<String, Recording>,
    template: &Template,
    header: Option<&Template>,
) -> String {
    // By recording, so two rips make one line naming both.
    let mut gathered: BTreeMap<&str, (Vec<&str>, &str)> = BTreeMap::new();
    for row in &plan.rows {
        let entry = gathered
            .entry(row.marker.id.as_str())
            .or_insert_with(|| (Vec::new(), row.path.as_str()));
        if let Some(variant) = row.marker.variant.as_deref() {
            if !entry.0.contains(&variant) {
                entry.0.push(variant);
            }
        }
        // The shallowest path, so a line points at the folder rather than at
        // whichever rip happened to be walked first.
        if row.path.as_str() < entry.1 {
            entry.1 = row.path.as_str();
        }
    }

    let mut lines: Vec<String> = gathered
        .into_iter()
        .filter_map(|(id, (mut variants, path))| {
            let recording = recordings.get(id)?;
            variants.sort_unstable();
            Some(template.render_line(&Listed {
                recording,
                variants: (!variants.is_empty()).then(|| variants.join(", ")),
                path: path.to_string(),
            }))
        })
        .collect();

    lines.sort_by_key(|l| sort_key(l));

    let Some(header) = header else {
        return lines.join("\n");
    };
    // A blank line between the note and the list, so the note reads as a note
    // rather than as the first entry.
    let note = header.render_line(&Summary {
        count: lines.len(),
        date: today(),
    });
    format!("{}\n\n{}", note.trim_end(), lines.join("\n"))
}
