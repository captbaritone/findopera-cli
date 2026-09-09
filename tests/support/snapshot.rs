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

/// Fold one JSON object into another, key by key.
fn merge(base: &mut serde_json::Value, over: &serde_json::Value) {
    match (base, over) {
        (serde_json::Value::Object(b), serde_json::Value::Object(o)) => {
            for (key, value) in o {
                merge(
                    b.entry(key.clone()).or_insert(serde_json::Value::Null),
                    value,
                );
            }
        }
        (b, o) => *b = o.clone(),
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

/// Where a step's path points, inside the sandbox.
fn at(sandbox: &Sandbox, path: &str) -> PathBuf {
    match path
        .strip_prefix("./library/")
        .or(path.strip_prefix("library/"))
    {
        Some(rest) => sandbox.library().join(rest),
        None => match path
            .strip_prefix("./named/")
            .or(path.strip_prefix("named/"))
        {
            Some(rest) => sandbox.destination().join(rest),
            None => sandbox.root.join(path.trim_start_matches("./")),
        },
    }
}

/// A step that is not a command: something done to the disk, or read off it.
///
/// A hard link, a clone and a copy all look the same in a listing, and their
/// inode numbers are not what anybody cares about. What differs is what
/// happens next — whether writing through one changes the original, whether a
/// track added later shows up. So a case can change a file and look at
/// another, and the difference is in what it reads back.
fn step(sandbox: &Sandbox, argv: &[String], out: &mut Vec<u8>) -> bool {
    let path = |i: usize| at(sandbox, &argv[i]);
    match argv[0].as_str() {
        "write" | "append" => {
            let file = path(1);
            if let Some(parent) = file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let text = argv[2..].join(" ");
            let mut existing = if argv[0] == "append" {
                std::fs::read_to_string(&file).unwrap_or_default()
            } else {
                String::new()
            };
            existing.push_str(&text);
            std::fs::write(&file, existing).expect("a file this case writes");
        }
        // A link somebody else left, or an earlier run under another
        // template. Only a case that declares `unix` may ask for one.
        "link" => {
            let target = path(1);
            let file = path(2);
            if let Some(parent) = file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &file).expect("a link this case makes");
            #[cfg(windows)]
            let _ = (&target, &file);
        }
        "rm" => {
            let file = path(1);
            if file.is_dir() && !file.is_symlink() {
                let _ = std::fs::remove_dir_all(&file);
            } else {
                let _ = std::fs::remove_file(&file);
            }
        }
        "show" => {
            let file = path(1);
            match std::fs::read_to_string(&file) {
                Ok(text) => {
                    let _ = writeln!(out, "{}", text.trim_end());
                }
                // Reading through a link whose target has gone is the whole
                // point of some of these, so it is an answer rather than a
                // failure.
                Err(e) => {
                    let _ = writeln!(out, "<cannot read {}: {}>", argv[1], e.kind());
                }
            }
        }
        _ => return false,
    }
    true
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

/// Forward slashes inside the path tokens, whatever this platform writes.
///
/// Only from the start of a token to the next space or tab: a template may
/// legitimately render a backslash into a folder name, and that is the name,
/// not a separator.
fn slashes_in_paths(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("./library").or_else(|| rest.find("./named")) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let end = tail
            .find(|c: char| c == ' ' || c == '\t' || c == '\n')
            .unwrap_or(tail.len());
        out.push_str(&tail[..end].replace('\\', "/"));
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// Replace anything that changes between runs.
fn normalize(text: &str, sandbox: &Sandbox) -> String {
    let library = sandbox.library().to_string_lossy().to_string();
    let destination = sandbox.destination().to_string_lossy().to_string();
    let root = sandbox.root.to_string_lossy().to_string();
    let normalized = text
        .replace(&library, "./library")
        .replace(&destination, "./named")
        .replace(&root, ".")
        .replace(env!("CARGO_PKG_VERSION"), "<version>");
    slashes_in_paths(&normalized)
}

/// Run one case file and return the sections it produced.
fn outputs_for(path: &Path, text: &str) -> Vec<Section> {
    command_outputs(path, &markdown::parse(text))
}

/// What a whole command did.
fn command_outputs(path: &Path, case: &markdown::Case) -> Vec<Section> {
    let name = path.file_stem().unwrap_or_default().to_string_lossy();
    let sandbox = Sandbox::new(&name);

    if let Some(tree) = case.body("Library") {
        build_tree(&sandbox.library(), tree);
    }

    // A destination that already holds something, with no record saying this
    // program put it there. That is the one starting state a prior run cannot
    // produce, and the whole reason the guard on an occupied destination
    // exists — so it is declared rather than built.
    //
    // A destination this program *did* build is not declared here. A case
    // gets one by running `organize --write` first, which is the only way to
    // be sure the tree and the record of it agree; writing the record by hand
    // would let a case assert against a state that could never occur.
    if let Some(tree) = case.body("Destination") {
        build_tree(&sandbox.destination(), tree);
    }

    // Canned answers, keyed by the operation each one belongs to.
    let mut script = Scripted::new();

    // The ordinary case takes its recordings from the ones captured off the
    // real API, rather than writing JSON by hand — which is how a fixture ends
    // up describing a server that never existed. Ids not in the corpus come
    // back null, which is what a missing recording looks like.
    if case.section("Recordings").is_some() || case.section("Library").is_some() {
        let raw = include_str!("../fixtures/plan-recordings.json");
        let mut all: Vec<serde_json::Value> =
            serde_json::from_str(raw).expect("the captured corpus");

        // `## Recording <id>` says a recording by how it differs from a real
        // one. Written out in full it would have to satisfy the whole
        // generated model — twenty fields to say that a title is "Salome" —
        // and a hand-written one that drifts from the schema is a fixture
        // describing a server that does not exist. Folded over a captured
        // record instead, a case says only what it is about.
        let base = all.first().cloned().expect("the corpus is not empty");
        let mut declared = Vec::new();
        for section in &case.inputs {
            let Some(id) = section.name.strip_prefix("Recording ") else {
                continue;
            };
            let over: serde_json::Value =
                serde_json::from_str(section.body.trim()).expect("a recording is JSON");
            let mut record = base.clone();
            merge(&mut record, &over);
            record["id"] = match id.trim().parse::<i64>() {
                Ok(n) => n.into(),
                Err(_) => id.trim().into(),
            };
            declared.push(record);
        }
        // Declared first, so one with the same id as a captured recording wins.
        declared.append(&mut all);
        script = script.serving_corpus(declared);
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
    // Two ways for a case to have settings. `## Toml` is written exactly as
    // given, for a case whose subject *is* the settings file — filling
    // anything in would be changing what is under test. `## Config` is the
    // convenience for every other case, which only wants somewhere to build.
    // A case that is *about* where the destination is has to be able to say
    // where, and only the run knows. These stand for the two directories.
    let fill = |text: &str| {
        // Into a TOML basic string, where a backslash starts an escape — and
        // a Windows path is mostly backslashes.
        let quoted = |p: std::path::PathBuf| p.to_string_lossy().replace('\\', "\\\\");
        text.replace("{library}", &quoted(sandbox.library()))
            .replace("{destination}", &quoted(sandbox.destination()))
            .replace("{root}", &quoted(sandbox.root.clone()))
    };
    let config = case
        .body("Toml")
        .map(&fill)
        .or_else(|| {
            case.body("Config").map(|c| {
                let mut config = c.trim().to_string();
                if !config.contains("template") {
                    config.insert_str(0, "template = \"{{opera.title}}\"\n");
                }
                format!(
                    "{config}\ndestination = \"{}\"\n",
                    sandbox.destination().display()
                )
            })
        })
        .unwrap_or_default();
    // A destination outside the sandbox makes the case depend on the machine
    // running it. One did, and passed here and failed in CI: `/Volumes/...`
    // is a separate disk on a Mac and a path that does not exist on Linux, so
    // the same settings drew a cross-device refusal in one place and a plan in
    // the other. Use `{destination}`.
    if let Some(line) = config
        .lines()
        .find(|l| l.trim_start().starts_with("destination"))
    {
        if let Some(named) = line.split('=').nth(1) {
            let named = named.trim().trim_matches('"').replace("\\\\", "\\");
            let named = named.as_str();
            assert!(
                !named.starts_with('/') || named.starts_with(&*sandbox.root.to_string_lossy()),
                "this case builds into {named}, which is outside the sandbox — what happens \
                 there depends on the machine running the case. Use {{destination}}."
            );
        }
    }

    let config_path = sandbox.root.join("findopera.toml");
    if case.section("Config").is_some() || case.section("Toml").is_some() {
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
        // The settings file belongs to the program, not to a step that only
        // touches the disk.
        if argv.first().is_some_and(|a| a == "findopera")
            && (case.section("Config").is_some() || case.section("Toml").is_some())
            && !argv.iter().any(|a| a == "--config")
        {
            argv.push("--config".to_string());
            argv.push(config_path.to_string_lossy().to_string());
        }

        if commands.len() > 1 {
            let _ = writeln!(out, "$ {command}");
        }
        // Anything that is not the program itself is something done to the
        // disk between runs.
        if argv.first().is_some_and(|a| a != "findopera") {
            assert!(
                step(&sandbox, &argv, &mut out),
                "no step called `{}`; the steps are write, append, rm and show",
                argv[0]
            );
            continue;
        }
        if commands.len() > 1 {
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

/// Where a case's expectations live.
fn expected_path(case: &Path) -> PathBuf {
    let stem = case.file_stem().unwrap_or_default().to_string_lossy();
    case.with_file_name(format!("{stem}.expected.md"))
}

/// Run every case under `dir`, blessing when asked.
pub fn run_all(dir: &Path) {
    let mut cases: Vec<PathBuf> = Vec::new();
    let mut expected: Vec<PathBuf> = Vec::new();
    collect(dir, &mut cases, &mut expected);
    cases.sort();
    assert!(!cases.is_empty(), "no cases under {}", dir.display());

    // An expectation whose case is gone would otherwise sit there for ever,
    // read as though something still produced it.
    let wanted: std::collections::BTreeSet<PathBuf> =
        cases.iter().map(|c| expected_path(c)).collect();
    let orphans: Vec<&PathBuf> = expected.iter().filter(|e| !wanted.contains(*e)).collect();
    assert!(
        orphans.is_empty(),
        "expectations with no case:\n{}",
        orphans
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let blessing = std::env::var("UPDATE_EXPECT").is_ok();
    let mut rewritten = Vec::new();
    let mut wrong = Vec::new();

    for path in &cases {
        let text = std::fs::read_to_string(path).expect("a case");

        // A case may need something this platform does not have. Symlinks are
        // the one so far: Windows asks for a privilege that CI does not grant,
        // which is why the older suite gated them the same way.
        if let Some(needs) = markdown::parse(&text).body("Requires") {
            let unmet = needs
                .split_whitespace()
                .any(|need| need == "unix" && !cfg!(unix));
            if unmet {
                continue;
            }
        }

        let source = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let produced =
            markdown::render(&markdown::title(&text), &source, &outputs_for(path, &text));

        let at = expected_path(path);
        let before = std::fs::read_to_string(&at).unwrap_or_default();
        if produced == before {
            continue;
        }
        if blessing {
            std::fs::write(&at, &produced).expect("blessing");
            rewritten.push(at);
        } else {
            wrong.push((at, before, produced));
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
        "rewrote {} expectation(s):\n{}\nRun again to confirm, after reading the diff.",
        rewritten.len(),
        rewritten
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Cases and expectations, told apart by name.
fn collect(dir: &Path, cases: &mut Vec<PathBuf>, expected: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, cases, expected);
        } else if path.to_string_lossy().ends_with(".expected.md") {
            expected.push(path);
        } else if path.extension().is_some_and(|e| e == "md")
            && !path
                .file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("README.md"))
        {
            // A directory says what its cases are for, the way the other
            // fixture directories already do. That note is not itself a case.
            cases.push(path);
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
