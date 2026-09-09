//! Running a case, and blessing what it found.
//!
//! One case is one whole command: a library on disk, canned answers from
//! findopera.com, and an argv. What it captures is everything that command
//! did — both streams, the exit code, the tree it left behind, and the
//! requests it made — because the interesting failures live *between* those.
//! A summary line claiming a folder was built, above a destination that does
//! not contain it, is only a bug if you can see both at once.

use super::markdown::{self, Section};
use super::scripted::Scripted;
use findopera::cli::{Cli, Session};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// A throwaway library and destination, cleaned up afterwards.
pub struct Sandbox {
    pub root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let root = std::env::temp_dir().join(format!(
            "findopera-case-{}-{}",
            name.replace(['/', ' ', '.'], "-"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("library")).expect("a library");
        std::fs::create_dir_all(root.join("named")).expect("a destination");
        Sandbox { root }
    }

    pub fn library(&self) -> PathBuf {
        self.root.join("library")
    }

    pub fn destination(&self) -> PathBuf {
        self.root.join("named")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Lay out a source library from a `tree` block.
///
/// One path per line. A line ending in `/` is a directory; anything else is a
/// file, whose contents are its own name so that a copy can be told from a
/// link later.
fn build_tree(at: &Path, spec: &str) {
    for line in spec.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let path = at.join(line.trim_end_matches('/'));
        if line.ends_with('/') {
            std::fs::create_dir_all(&path).expect("a directory");
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("a parent");
            }
            std::fs::write(&path, line).expect("a file");
        }
    }
}

/// What is on disk, as a case shows it.
///
/// Sorted, relative, and with a symlink written as where it points, so that
/// the link mode is legible rather than erased.
fn describe_tree(at: &Path) -> String {
    fn walk(at: &Path, base: &Path, into: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(at) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            let shown = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            // The record this program keeps is its own business, not the
            // case's; it would only ever restate the tree beside it.
            if shown.contains(".findopera-state") {
                continue;
            }
            let meta = entry.path().symlink_metadata().expect("metadata");
            if meta.is_symlink() {
                let target = std::fs::read_link(&path).unwrap_or_default();
                let target = target.to_string_lossy();
                // Only the tail: the rest is a temporary directory whose name
                // changes every run.
                let tail = target.rsplit('/').next().unwrap_or(&target).to_string();
                into.push(format!("{shown} -> …/{tail}"));
            } else if meta.is_dir() {
                into.push(format!("{shown}/"));
                walk(&path, base, into);
            } else {
                into.push(shown);
            }
        }
    }
    let mut lines = Vec::new();
    walk(at, at, &mut lines);
    lines.join("\n")
}

/// Split a command line the way a shell would, so a template can be quoted.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quote: Option<char> = None;
    let mut any = false;
    for c in line.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => word.push(c),
            None if c == '\'' || c == '"' => {
                quote = Some(c);
                any = true;
            }
            None if c.is_whitespace() => {
                if !word.is_empty() || any {
                    out.push(std::mem::take(&mut word));
                    any = false;
                }
            }
            None => word.push(c),
        }
    }
    if !word.is_empty() || any {
        out.push(word);
    }
    out
}

/// Replace anything that changes between runs.
fn normalize(text: &str, sandbox: &Sandbox) -> String {
    let library = sandbox.library().to_string_lossy().to_string();
    let destination = sandbox.destination().to_string_lossy().to_string();
    let root = sandbox.root.to_string_lossy().to_string();
    text.replace(&library, "./library")
        .replace(&destination, "./named")
        .replace(&root, ".")
        .replace(env!("CARGO_PKG_VERSION"), "<version>")
}

/// Run one case file and return the sections it produced.
fn outputs_for(path: &Path, text: &str) -> Vec<Section> {
    let case = markdown::parse(text);
    let name = path.file_stem().unwrap_or_default().to_string_lossy();
    let sandbox = Sandbox::new(&name);

    if let Some(tree) = case.body("Library") {
        build_tree(&sandbox.library(), tree);
    }

    // Canned answers, keyed by the operation each one belongs to.
    let mut script = Scripted::new();

    // The ordinary case takes its recordings from the ones captured off the
    // real API, rather than writing JSON by hand — which is how a fixture ends
    // up describing a server that never existed. Ids not in the corpus come
    // back null, which is what a missing recording looks like.
    if case.section("Recordings").is_some() || case.section("Library").is_some() {
        let raw = include_str!("../fixtures/plan-recordings.json");
        let all: Vec<serde_json::Value> = serde_json::from_str(raw).expect("the captured corpus");
        script = script.serving_corpus(all);
    }

    for section in &case.inputs {
        if let Some(operation) = section.name.strip_prefix("Answer ") {
            script = script.answers(operation.trim(), 200, section.body.trim());
        }
    }
    if let Some(body) = case.body("Answer") {
        script = script.otherwise(200, body.trim());
    }

    // The config names the destination, which only exists at run time.
    // The destination only exists at run time, and a config must name a
    // template even where the case overrides it per run with -t.
    let config = case
        .body("Config")
        .map(|c| {
            let mut config = c.trim().to_string();
            if !config.contains("template") {
                config.insert_str(0, "template = \"{{opera.title}}\"\n");
            }
            format!(
                "{config}\ndestination = \"{}\"\n",
                sandbox.destination().display()
            )
        })
        .unwrap_or_default();
    let config_path = sandbox.root.join("findopera.toml");
    if case.section("Config").is_some() {
        std::fs::write(&config_path, &config).expect("a config");
    }

    // Every `$ …` line is a command, run in order against the same library
    // and destination. A second run is how a case says "and then the template
    // changed", which is where the interesting failures are.
    let commands: Vec<String> = case
        .body("Run")
        .unwrap_or("$ findopera --help")
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('$'))
        .map(|l| l.trim_start_matches('$').trim().to_string())
        .collect();

    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut codes = Vec::new();
    for command in &commands {
        let mut argv = words(command);
        for word in argv.iter_mut() {
            if word == "./library" {
                *word = sandbox.library().to_string_lossy().to_string();
            }
        }
        if case.section("Config").is_some() && !argv.iter().any(|a| a == "--config") {
            argv.push("--config".to_string());
            argv.push(config_path.to_string_lossy().to_string());
        }

        if commands.len() > 1 {
            let _ = writeln!(out, "$ {command}");
            let _ = writeln!(err, "$ {command}");
        }
        let for_client = script.clone();
        let mut ui = Session::new(&mut out, &mut err)
            .served_by(move |endpoint, token| for_client.client(endpoint, token));
        let code = match Cli::try_parse_from_argv(&argv) {
            Ok(cli) => findopera::cli::dispatch(&mut ui, cli),
            Err(rendered) => {
                ui.note(format_args!("{rendered}"));
                2
            }
        };
        codes.push(code.to_string());
    }

    let mut sections = Vec::new();
    let mut push = |name: &str, language: &str, body: String| {
        sections.push(Section {
            name: name.to_string(),
            language: language.to_string(),
            body,
        });
    };

    push(
        "stdout",
        "",
        normalize(&String::from_utf8_lossy(&out), &sandbox),
    );
    push(
        "stderr",
        "",
        normalize(&String::from_utf8_lossy(&err), &sandbox),
    );
    push("Destination", "tree", describe_tree(&sandbox.destination()));

    let mut asked = String::new();
    for sent in script.sent() {
        if let Some(operation) = &sent.operation {
            let _ = writeln!(asked, "{operation}");
            if let Some(variables) = &sent.variables {
                let _ = writeln!(asked, "  {variables}");
            }
        } else if sent.url.starts_with("(notice)") {
            let _ = writeln!(asked, "{}", sent.url);
        }
    }
    push("Requests", "", normalize(asked.trim(), &sandbox));
    push("Exit", "", codes.join("\n"));
    sections
}

/// Run every case under `dir`, blessing when asked.
pub fn run_all(dir: &Path) {
    let mut cases: Vec<PathBuf> = Vec::new();
    collect(dir, &mut cases);
    cases.sort();
    assert!(!cases.is_empty(), "no cases under {}", dir.display());

    let blessing = std::env::var("UPDATE_EXPECT").is_ok();
    let mut rewritten = Vec::new();
    let mut wrong = Vec::new();

    for path in &cases {
        let text = std::fs::read_to_string(path).expect("a case");
        let produced = markdown::render(&text, &outputs_for(path, &text));
        if produced == text {
            continue;
        }
        if blessing {
            std::fs::write(path, &produced).expect("blessing");
            rewritten.push(path.clone());
        } else {
            wrong.push((path.clone(), text, produced));
        }
    }

    if !wrong.is_empty() {
        let mut report = String::new();
        for (path, was, now) in &wrong {
            let _ = writeln!(report, "\n{} changed:", path.display());
            let _ = writeln!(report, "{}", difference(was, now));
        }
        let _ = writeln!(
            report,
            "\n{} case(s) do not match. UPDATE_EXPECT=1 cargo test rewrites them; \
             read the diff before believing it.",
            wrong.len()
        );
        panic!("{report}");
    }

    // A blessing run that changed anything fails on purpose: a green test has
    // to mean the expectations on disk are the ones that ran.
    assert!(
        rewritten.is_empty(),
        "rewrote {} case(s):\n{}\nRun again to confirm, after reading the diff.",
        rewritten.len(),
        rewritten
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|e| e == "md") {
            into.push(path);
        }
    }
}

/// A line-by-line difference, enough to read in a test failure.
fn difference(was: &str, now: &str) -> String {
    let old: Vec<&str> = was.lines().collect();
    let new: Vec<&str> = now.lines().collect();
    let mut out = String::new();
    for i in 0..old.len().max(new.len()) {
        match (old.get(i), new.get(i)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => {
                let _ = writeln!(out, "-{a}\n+{b}");
            }
            (Some(a), None) => {
                let _ = writeln!(out, "-{a}");
            }
            (None, Some(b)) => {
                let _ = writeln!(out, "+{b}");
            }
            (None, None) => {}
        }
    }
    out
}
