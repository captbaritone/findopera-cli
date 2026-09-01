//! End-to-end tests driven by the fixture files in `tests/cases/`.
//!
//! One file is one case: a template, some data, and the result — the engine's
//! whole contract, exercised through the public API only. Nothing here reaches
//! inside for tokens or AST nodes, so the fixtures stay readable as a
//! specification of the language rather than of its implementation. The file
//! name is the case name, so `ls tests/cases/*` reads as the spec's contents.
//!
//! # The format
//!
//! ```text
//! Any lines before the first `--- ` are a note, for whatever the file name
//! could not say on its own. Most cases need none.
//!
//! --- template
//! {{opera.englishTitle|opera.title}}
//! --- data
//! opera.title = Salome
//! --- expect
//! Salome
//! ```
//!
//! A section runs until the next line beginning with `--- `. Each case names
//! a `--- template`, optionally some `--- data`, and an outcome:
//!
//! `--- expect` is the outcome: either the rendered path, segments joined with
//! `/`, or a diagnostic beginning `error[code]:` — for a parse error followed
//! by the template with the offending span underlined and a `help:` line.
//! Values are sanitized so a segment can never itself contain `/`, so a
//! rendered path round-trips.
//!
//! A `template_*` code is a parse error, settled against the schema with no
//! record in hand. A `path_*` code is the one thing still decided per record:
//! rendering itself is total, so the string always exists and only its
//! suitability as a path is in question.
//!
//! A `--- data` line is `path = value`. The value is trimmed; to write one
//! with significant whitespace or a control character, quote it and use
//! `\n \t \r \0 \\ \" \u{7}` escapes. A path absent from the data is absent
//! for the record, which is what drives group omission.
//!
//! Subdirectories group cases by topic and carry a `README.md` saying what the
//! topic covers. Only `.txt` files are cases; the numeric filename prefixes
//! keep each directory in reading order rather than alphabetical order.
//!
//! # Updating expectations
//!
//! ```text
//! UPDATE_EXPECT=1 cargo test
//! ```
//!
//! rewrites every `--- expect` block in place. That is the intended way to
//! write a new case: state the template and data, leave the outcome empty,
//! bless, and
//! read the diff. Blessing is only ever as good as the review of that diff,
//! so a run that rewrote anything *fails*, listing the files it touched — a
//! green test always means the expectations on disk are the ones that ran.

use findopera::{to_path, FieldDoc, Fields, Template};
use std::collections::BTreeMap;
use std::path::Path;

/// The field surface the fixtures are written against.
///
/// Deliberately a stand-in rather than a real model: it mirrors the shape of
/// FindOpera's recording graph — nested namespaces, an indexed series, a
/// couple of near-miss names for the suggestion tests — without the engine
/// depending on any of it. A generated model will supply a table like this.
///
/// Which fields are `non_null` follows what the real database actually holds:
/// `opera.title`, `composer.lastName` and `conductor.lastName` are populated
/// for every recording, while `opera.englishTitle` is there for fewer than one
/// in five. That split is what the parse-time checks reason about, so the
/// fixtures only mean anything if it stays realistic.
static SCHEMA: &[FieldDoc] = &[
    FieldDoc::non_null("id", "Record id"),
    FieldDoc::new("year", "Year recorded"),
    FieldDoc::new("month", "Month recorded, zero-padded"),
    FieldDoc::new("day", "Day recorded, zero-padded"),
    FieldDoc::new("orchestra", "Orchestra name"),
    FieldDoc::new("chorus", "Chorus name"),
    FieldDoc::non_null("opera.title", "Title in the original language"),
    FieldDoc::new("opera.englishTitle", "English title, if the opera has one"),
    FieldDoc::new("opera.librettist", "Librettist"),
    FieldDoc::new("opera.language", "Language sung"),
    FieldDoc::new("composer.fullName", "Composer, full name"),
    FieldDoc::new("composer.firstName", "Composer given name(s)"),
    FieldDoc::non_null("composer.lastName", "Composer surname"),
    FieldDoc::new("composer.born", "Composer year of birth"),
    FieldDoc::new("composer.died", "Composer year of death"),
    FieldDoc::new("conductor.fullName", "Conductor, full name"),
    FieldDoc::non_null("conductor.lastName", "Conductor surname"),
    FieldDoc::new("singer1", "First noted singer"),
    FieldDoc::new("singer2", "Second noted singer"),
    FieldDoc::new("singer3", "Third noted singer"),
];

/// A record built straight from a case's `--- data` block.
///
/// `required` hands back the empty string for a field the case left out. A
/// resolver has to return *something*, and this is what a real one built over
/// a `String` struct field would do when the record carries an empty one — so
/// the fixtures can still exercise what happens downstream.
struct Record(BTreeMap<String, String>);

impl Fields for Record {
    fn required(&self, path: &str) -> String {
        self.0.get(path).cloned().unwrap_or_default()
    }
    fn optional(&self, path: &str) -> Option<String> {
        self.0.get(path).cloned()
    }
}

// ---------------------------------------------------------------- the test

mod fixture;

#[test]
fn cases() {
    fixture::run(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases"),
        &["template"],
        &["data"],
        &["expect"],
        |name, input| {
            let outcome = run_case(name, &input["template"], input.get("data"));
            BTreeMap::from([("expect".to_string(), outcome)])
        },
    );
}

/// Parse and render one case, returning the outcome it should carry.
fn run_case(name: &str, template: &str, data: Option<&String>) -> String {
    let mut record = BTreeMap::new();
    for line in data.map(String::as_str).unwrap_or("").lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (path, raw) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("{name}: a data line reads `path = value`"));
        let path = path.trim().to_string();
        assert!(
            SCHEMA.iter().any(|f| f.path == path),
            "{name}: data names `{path}`, which is not in SCHEMA"
        );
        record.insert(path, unquote(raw.trim()));
    }

    let tmpl = match Template::parse(template, SCHEMA) {
        Ok(t) => t,
        Err(e) => {
            let mut out = format!("error[{}]: {}\n", e.code, e.message);
            out.push_str(&e.underline(template).join("\n"));
            if let Some(help) = &e.help {
                out.push_str(&format!("\nhelp: {help}"));
            }
            return out;
        }
    };

    // Rendering cannot fail; only judging the result as a path can.
    let rendered = tmpl.render(&Record(record));
    match to_path(&rendered) {
        Ok(segments) => segments.join("/"),
        Err(e) => format!("error[{}]: {e}", e.code()),
    }
}

/// A bare value is taken as written; a quoted one honors escapes, which is the
/// only way to put a control character or edge whitespace in a fixture.
fn unquote(raw: &str) -> String {
    let Some(inner) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) else {
        return raw.to_string();
    };
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('u') => {
                let hex: String = chars
                    .by_ref()
                    .skip_while(|c| *c == '{')
                    .take_while(|c| *c != '}')
                    .collect();
                let n = u32::from_str_radix(&hex, 16).expect("\\u{..} needs hex digits");
                out.push(char::from_u32(n).expect("\\u{..} needs a valid scalar value"));
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}
