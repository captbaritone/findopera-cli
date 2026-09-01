//! A small harness for fixture-driven snapshot tests.
//!
//! One file is one case. Sections are introduced by a line beginning `--- `
//! and run to the next such line; anything before the first is a note. Some
//! sections are inputs, written by hand; the rest are snapshots, rewritten by
//! `UPDATE_EXPECT=1`.
//!
//! A blessing run that changed anything *fails*, listing the files it touched.
//! Cargo swallows the output of a passing test, and a rewritten expectation is
//! exactly what wants reading — so a green test always means the expectations
//! on disk are the ones that ran.
//!
//! An empty snapshot is written `(empty)`, because a section with nothing
//! under it is too easy to mistake for an unfinished case.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub type Sections = BTreeMap<String, String>;

/// Named for the format docs in the suites that use this.
pub struct Section;

const EMPTY: &str = "(empty)";

/// A path written the way the fixtures write them, whatever the platform uses.
///
/// The library hands back native paths, which is right for someone reading its
/// output — but a snapshot that changed shape on Windows would test the
/// platform rather than the program.
pub fn slashes(s: &str) -> String {
    s.replace(std::path::MAIN_SEPARATOR, "/")
}

/// Run every `.txt` case under `dir`.
///
/// `inputs` and `outputs` name the sections, in the order they are written
/// back. `run_case` is given the case name and its input sections, and returns
/// the snapshots.
///
/// An input in `optional` may be left out of a case, and arrives empty; every
/// other input must be present and non-empty, since a case missing one is
/// almost always unfinished rather than deliberate.
pub fn run(
    dir: PathBuf,
    inputs: &[&str],
    optional: &[&str],
    outputs: &[&str],
    run_case: impl Fn(&str, &Sections) -> Sections,
) {
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
        let case = Case::parse(&original, &name, inputs, optional, outputs);

        let got = run_case(&name, &case.sections);
        let mut after = case.sections.clone();
        for key in outputs {
            // Trailing newlines are trimmed off a parsed section, so they have
            // to come off a produced one too — otherwise anything ending in a
            // newline can never match what was written for it, and blessing
            // rewrites the same bytes for ever.
            let value = got.get(*key).cloned().unwrap_or_default();
            after.insert((*key).to_string(), value.trim_end_matches('\n').to_string());
        }
        if after == case.sections {
            continue;
        }

        let rendered = Case {
            note: case.note.clone(),
            sections: after.clone(),
        }
        .render(inputs, optional, outputs);
        if bless {
            std::fs::write(path, rendered).expect("fixture is writable");
            blessed.push(name);
        } else {
            failures.push(report(&name, outputs, &case.sections, &after));
        }
    }

    if bless {
        assert!(
            blessed.is_empty(),
            "UPDATE_EXPECT rewrote {} case(s):\n{}\n\n\
             Read the diff, then re-run without UPDATE_EXPECT to confirm.",
            blessed.len(),
            blessed
                .iter()
                .map(|b| format!("  {}/{b}", dir.file_name().unwrap().to_string_lossy()))
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

fn report(file: &str, outputs: &[&str], before: &Sections, after: &Sections) -> String {
    let block = |s: &Sections| -> String {
        outputs
            .iter()
            .map(|k| {
                let body = s.get(*k).map(String::as_str).unwrap_or("");
                let body = if body.is_empty() {
                    format!("    {EMPTY}")
                } else {
                    body.lines()
                        .map(|l| format!("    {l}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                format!("  --- {k}\n{body}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "{file}\nexpected:\n{}\nactual:\n{}\n",
        block(before),
        block(after)
    )
}

#[derive(Clone)]
struct Case {
    note: Vec<String>,
    sections: Sections,
}

impl Case {
    fn parse(text: &str, name: &str, inputs: &[&str], optional: &[&str], outputs: &[&str]) -> Case {
        let known: Vec<&str> = inputs
            .iter()
            .chain(optional)
            .chain(outputs)
            .copied()
            .collect();
        let mut note = Vec::new();
        let mut sections: Sections = BTreeMap::new();
        let mut current: Option<String> = None;

        for (n, line) in text.lines().enumerate() {
            if let Some(rest) = line.strip_prefix("---") {
                let head = rest.trim().to_string();
                assert!(
                    known.contains(&head.as_str()),
                    "{name}:{}: unknown section `--- {head}`",
                    n + 1
                );
                sections.entry(head.clone()).or_default();
                current = Some(head);
                continue;
            }
            match &current {
                None => note.push(line.to_string()),
                Some(k) => {
                    let body = sections.get_mut(k).expect("inserted above");
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }

        for k in &known {
            let body = sections.entry((*k).to_string()).or_default();
            *body = body.trim_end_matches('\n').to_string();
            if body == EMPTY {
                body.clear();
            }
        }
        while note.last().is_some_and(|l| l.trim().is_empty()) {
            note.pop();
        }
        for k in inputs {
            assert!(
                !sections[*k].is_empty(),
                "{name}: `--- {k}` is an input and cannot be empty"
            );
        }
        // An optional input that was never written stays out of the file.
        for k in optional {
            if sections[*k].is_empty() {
                sections.remove(*k);
            }
        }
        Case { note, sections }
    }

    fn render(&self, inputs: &[&str], optional: &[&str], outputs: &[&str]) -> String {
        let mut out = String::new();
        for line in &self.note {
            out.push_str(line);
            out.push('\n');
        }
        if !self.note.is_empty() {
            out.push('\n');
        }
        for k in inputs.iter().chain(optional).chain(outputs) {
            let body = self.sections.get(*k).map(String::as_str).unwrap_or("");
            // An optional section that was left out stays left out.
            if body.is_empty() && optional.contains(k) {
                continue;
            }
            out.push_str(&format!("--- {k}\n"));
            out.push_str(if body.is_empty() { EMPTY } else { body });
            out.push('\n');
        }
        out
    }
}
