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
//! a `--- template`, optionally some `--- data`, and exactly one outcome:
//!
//! - `--- expect` — the rendered path, segments joined with `/`. Values are
//!   sanitized so a segment can never itself contain `/`, so this round-trips.
//! - `--- error` — the diagnostic: `error[code]: message`, then for a parse
//!   error the template with the offending span underlined, then `help:`.
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
//! rewrites every `--- expect` / `--- error` block in place, including
//! switching a case between the two. That is the intended way to write a new
//! case: state the template and data, leave the outcome empty, bless, and
//! read the diff. Blessing is only ever as good as the review of that diff,
//! so a run that rewrote anything *fails*, listing the files it touched — a
//! green test always means the expectations on disk are the ones that ran.

use findopera::{to_path, FieldDoc, Fields, Template};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

#[test]
fn cases() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases");
    let bless = std::env::var_os("UPDATE_EXPECT").is_some();

    let mut files = Vec::new();
    collect(&dir, &mut files);
    files.sort();
    assert!(!files.is_empty(), "no fixtures under {}", dir.display());

    let mut failures = Vec::new();
    let mut blessed = Vec::new();

    for path in &files {
        let name = path
            .strip_prefix(&dir)
            .expect("collected from dir")
            .to_string_lossy()
            .into_owned();
        let original = std::fs::read_to_string(path).expect("fixture is readable");
        let before = Case::parse(&original, &name);

        let (outcome, expected) = run(&before);
        if before.outcome == outcome && before.expected == expected {
            continue;
        }
        let after = Case {
            outcome,
            expected,
            ..before.clone()
        };

        if bless {
            std::fs::write(path, after.render()).expect("fixture is writable");
            blessed.push(name);
        } else {
            failures.push(report(&name, &before, &after));
        }
    }

    if bless {
        // Fail on a run that changed something, so a bless can never be
        // mistaken for a green test — cargo swallows the output of a passing
        // one, and a rewritten expectation is exactly what wants reading.
        assert!(
            blessed.is_empty(),
            "UPDATE_EXPECT rewrote {} case(s):\n{}\n\n\
             Read the diff, then re-run without UPDATE_EXPECT to confirm.",
            blessed.len(),
            blessed
                .iter()
                .map(|b| format!("  tests/cases/{b}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        return;
    }

    assert!(
        failures.is_empty(),
        "{} case(s) did not match:\n\n{}\n\
         Re-run with UPDATE_EXPECT=1 to rewrite the expectations, then read the diff.",
        failures.len(),
        failures.join("\n")
    );
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("readable dir entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|x| x == "txt") {
            out.push(path);
        }
    }
}

/// Parse and render one case, returning the outcome section it should carry.
fn run(case: &Case) -> (Outcome, String) {
    let mut data = BTreeMap::new();
    for (path, value) in &case.data {
        assert!(
            SCHEMA.iter().any(|f| f.path == *path),
            "tests/cases/{}: data names `{path}`, which is not in SCHEMA",
            case.name
        );
        data.insert(path.clone(), value.clone());
    }

    let tmpl = match Template::parse(&case.template, SCHEMA) {
        Ok(t) => t,
        Err(e) => {
            let mut out = format!("error[{}]: {}\n", e.code, e.message);
            out.push_str(&e.underline(&case.template).join("\n"));
            if let Some(help) = &e.help {
                out.push_str(&format!("\nhelp: {help}"));
            }
            return (Outcome::Error, out);
        }
    };

    // Rendering cannot fail; only judging the result as a path can.
    let rendered = tmpl.render(&Record(data));
    match to_path(&rendered) {
        Ok(segments) => (Outcome::Expect, segments.join("/")),
        Err(e) => (Outcome::Error, format!("error[{}]: {e}", e.code())),
    }
}

fn report(file: &str, before: &Case, after: &Case) -> String {
    let block = |c: &Case| {
        let body = if c.expected.is_empty() {
            "    (empty)".to_string()
        } else {
            c.expected
                .lines()
                .map(|l| format!("    {l}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        format!("  --- {}\n{body}", c.outcome.keyword())
    };
    format!(
        "tests/cases/{file}\n  --- template\n    {}\nexpected:\n{}\nactual:\n{}\n",
        before.template.replace('\n', "\n    "),
        block(before),
        block(after),
    )
}

// -------------------------------------------------------- the fixture format

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Outcome {
    Expect,
    Error,
}

impl Outcome {
    fn keyword(self) -> &'static str {
        match self {
            Outcome::Expect => "expect",
            Outcome::Error => "error",
        }
    }
}

#[derive(Clone)]
struct Case {
    /// Path relative to `tests/cases`, used to name the case in failures.
    name: String,
    /// Free text before the first section, kept verbatim.
    note: Vec<String>,
    template: String,
    /// Verbatim `--- data` lines, so blessing round-trips them untouched.
    data_lines: Vec<String>,
    /// The same lines, parsed.
    data: Vec<(String, String)>,
    outcome: Outcome,
    expected: String,
}

impl Case {
    fn parse(text: &str, name: &str) -> Case {
        let mut case = Case {
            name: name.to_string(),
            note: Vec::new(),
            template: String::new(),
            data_lines: Vec::new(),
            data: Vec::new(),
            outcome: Outcome::Expect,
            expected: String::new(),
        };
        // Which section body the following lines belong to, if any.
        let mut section: Option<&str> = None;
        let mut seen_outcome = false;

        for (n, line) in text.lines().enumerate() {
            let at = |msg: &str| format!("tests/cases/{name}:{}: {msg}", n + 1);

            if let Some(rest) = line.strip_prefix("---") {
                let head = rest.trim();
                match head {
                    "template" | "data" => {}
                    "expect" | "error" => {
                        assert!(!seen_outcome, "{}", at("a case has one outcome section"));
                        seen_outcome = true;
                        case.outcome = if head == "error" {
                            Outcome::Error
                        } else {
                            Outcome::Expect
                        };
                    }
                    other => panic!("{}", at(&format!("unknown section `--- {other}`"))),
                }
                section = Some(if head == "expect" || head == "error" {
                    "outcome"
                } else {
                    head
                });
                continue;
            }

            match section {
                None => case.note.push(line.to_string()),
                Some("template") => push_line(&mut case.template, line),
                Some("outcome") => push_line(&mut case.expected, line),
                Some("data") => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let (path, raw) = line
                        .split_once('=')
                        .unwrap_or_else(|| panic!("{}", at("a data line reads `path = value`")));
                    case.data_lines.push(line.to_string());
                    case.data
                        .push((path.trim().to_string(), unquote(raw.trim())));
                }
                Some(_) => unreachable!(),
            }
        }

        // A body picks up the blank line that ends the file; that is layout,
        // not content.
        case.template = case.template.trim_end_matches('\n').to_string();
        case.expected = case.expected.trim_end_matches('\n').to_string();
        while case.note.last().is_some_and(|l| l.trim().is_empty()) {
            case.note.pop();
        }
        assert!(
            !case.template.is_empty(),
            "tests/cases/{name}: no `--- template` section"
        );
        assert!(
            seen_outcome,
            "tests/cases/{name}: no `--- expect` or `--- error` section"
        );
        case
    }

    fn render(&self) -> String {
        let mut out = String::new();
        for line in &self.note {
            out.push_str(line);
            out.push('\n');
        }
        if !self.note.is_empty() {
            out.push('\n');
        }
        out.push_str("--- template\n");
        out.push_str(&self.template);
        out.push('\n');
        if !self.data_lines.is_empty() {
            out.push_str("--- data\n");
            for line in &self.data_lines {
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push_str(&format!("--- {}\n", self.outcome.keyword()));
        if !self.expected.is_empty() {
            out.push_str(&self.expected);
            out.push('\n');
        }
        out
    }
}

fn push_line(buf: &mut String, line: &str) {
    buf.push_str(line);
    buf.push('\n');
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
