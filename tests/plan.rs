//! End-to-end tests for planning, driven by the fixture files in
//! `tests/plan-cases/`.
//!
//! One file is one case: a template, a directory tree, and what the plan says
//! about it. Everything is exercised through the public API — `scan::scan`
//! over a real temporary tree, then `plan::plan` against canned recordings —
//! so the only thing missing compared with running the binary is the network
//! and the argument parsing.
//!
//! # The format
//!
//! ```text
//! Any lines before the first `--- ` are a note.
//!
//! --- template
//! {{opera.title}} \[{{id}}\][ ({{variant}})]
//! --- tree
//! rips/flac/332 flac.txt
//! rips/mp3/332 mp3.txt
//! --- listing
//! rips/flac  Don Giovanni [332] (flac)
//! rips/mp3   Don Giovanni [332] (mp3)
//! --- report
//! (empty)
//! ```
//!
//! `--- strict` is optional; present with any content, the case runs the way
//! `--require-variants` would.
//!
//! `--- tree` is a list of files to create, one per line, relative to the
//! case's own temporary directory. They are all created empty: a marker is
//! identified by its name, so there is nothing to put inside one.
//!
//! `--- listing` is what would go to stdout and `--- report` what would go to
//! stderr, with paths relative to the tree root so the snapshots do not carry
//! a temporary directory in them. An empty report is written `(empty)`, since
//! a section with nothing in it is hard to tell from a mistake.
//!
//! The recordings come from `tests/fixtures/plan-recordings.json`, captured
//! from the real API. It deliberately includes 332 and 5876 — two separate
//! FindOpera recordings of Don Giovanni with the same year, conductor and
//! singers, which is the collision this whole area exists to handle.
//!
//! # Updating expectations
//!
//! ```text
//! UPDATE_EXPECT=1 cargo test
//! ```
//!
//! rewrites the `--- listing` and `--- report` blocks in place, then fails on
//! purpose listing what it touched — a green test always means the
//! expectations on disk are the ones that ran.

use findopera::model::{Recording, FIELDS};
use findopera::{plan, scan, FieldDoc, Template};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod fixture;

fn recordings() -> BTreeMap<String, Recording> {
    let raw = include_str!("fixtures/plan-recordings.json");
    let list: Vec<Recording> = serde_json::from_str(raw).expect("the captured response");
    list.into_iter().map(|r| (r.id.to_string(), r)).collect()
}

#[test]
fn cases() {
    fixture::run(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/plan-cases"),
        &["template", "tree"],
        &["strict"],
        &["listing", "report"],
        |name, input| {
            let tree = Tree::new(name, input["tree"].lines());
            let report = scan::scan(std::slice::from_ref(&tree.0), false);

            let mut schema: Vec<FieldDoc> = FIELDS.to_vec();
            schema.push(scan::VARIANT);
            let tmpl = match Template::parse(&input["template"], &schema) {
                Ok(t) => t,
                Err(e) => {
                    return BTreeMap::from([
                        ("listing".to_string(), String::new()),
                        (
                            "report".to_string(),
                            format!("error[{}]: {}", e.code, e.message),
                        ),
                    ])
                }
            };

            // `--- strict` present, with any content, runs the case the way
            // `--require-variants` would.
            let strict = input.contains_key("strict");
            let plan = plan::plan(&report.markers, &recordings(), &tmpl);
            let strip = |s: String| s.replace(&format!("{}/", tree.0.display()), "");
            BTreeMap::from([
                (
                    "listing".to_string(),
                    plan.listing(true)
                        .into_iter()
                        .map(strip)
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
                (
                    "report".to_string(),
                    plan.report(strict)
                        .into_iter()
                        .map(strip)
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
            ])
        },
    );
}

/// A throwaway directory tree, removed when the case ends.
struct Tree(PathBuf);

impl Tree {
    fn new<'a>(name: &str, files: impl Iterator<Item = &'a str>) -> Tree {
        let slug: String = name.chars().filter(|c| c.is_alphanumeric()).collect();
        let dir =
            std::env::temp_dir().join(format!("findopera-plan-{slug}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        for f in files {
            let f = f.trim();
            if f.is_empty() {
                continue;
            }
            let full = dir.join(f);
            fs::create_dir_all(full.parent().expect("a parent")).expect("parent dirs");
            fs::write(full, "").expect("write");
        }
        Tree(dir)
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
