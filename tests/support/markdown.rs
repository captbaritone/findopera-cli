//! Reading and writing a case file.
//!
//! A case is one markdown document. `##` headings are its sections; a fenced
//! block under a heading is that section's content. Markdown is the wrapper
//! rather than a format of our own because it buys three things at once: the
//! prose above a section says *why* the case exists, each block keeps the
//! syntax highlighting of whatever it holds, and nothing else in the toolchain
//! tries to lint or format the contents.
//!
//! Inputs and outputs are separate files: `some-case.md` is written by hand
//! and never touched, `some-case.expected.md` is generated whole. Splicing
//! generated content into the file that describes the case meant a marker
//! line whose loss would silently turn old output into input, and it made a
//! clean regeneration of everything impossible.

use std::fmt::Write as _;

/// Said at the top of every generated file, so nobody edits one by hand.
pub const GENERATED: &str = "<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->";

/// One `##` section of a case.
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    /// The language on the fence, which says how to read the body.
    pub language: String,
    pub body: String,
}

/// A parsed case file.
#[derive(Debug, Clone)]
pub struct Case {
    /// The `#` title, and any prose before the first section.
    pub preamble: String,
    pub inputs: Vec<Section>,
}

impl Case {
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.inputs
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    pub fn body(&self, name: &str) -> Option<&str> {
        self.section(name).map(|s| s.body.as_str())
    }
}

/// Read a case: its title, its prose, and its sections.
pub fn parse(text: &str) -> Case {
    let inputs_text = text;

    let mut preamble = String::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<Section> = None;
    let mut fence: Option<String> = None;

    for line in inputs_text.lines() {
        // A fence closes only on a fence of at least its own length, so a
        // block holding backticks can be wrapped in more of them.
        if let Some(open) = &fence {
            if line.trim_start().starts_with(open.as_str()) && line.trim().chars().all(|c| c == '`')
            {
                fence = None;
                continue;
            }
            if let Some(s) = current.as_mut() {
                s.body.push_str(line);
                s.body.push('\n');
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("## ") {
            if let Some(done) = current.take() {
                sections.push(done);
            }
            current = Some(Section {
                name: rest.trim().to_string(),
                language: String::new(),
                body: String::new(),
            });
            continue;
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            let ticks: String = trimmed.chars().take_while(|c| *c == '`').collect();
            let language = trimmed[ticks.len()..].trim().to_string();
            if let Some(s) = current.as_mut() {
                s.language = language;
            }
            fence = Some(ticks);
            continue;
        }

        if current.is_none() {
            preamble.push_str(line);
            preamble.push('\n');
        }
    }
    if let Some(done) = current.take() {
        sections.push(done);
    }

    Case {
        preamble: preamble.trim_end().to_string(),
        inputs: sections,
    }
}

/// The case's title, for the head of the generated file.
///
/// Repeated there so that a diff of expectations alone still says which case
/// it belongs to — a reviewer reading a behaviour change should not have to
/// open a second file to learn what was supposed to happen.
pub fn title(text: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or("A case")
        .trim()
        .to_string()
}

/// Write what a case found, as a document of its own.
pub fn render(title: &str, source: &str, outputs: &[Section]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# {title}\n");
    let _ = writeln!(out, "<!-- From {source}. -->");
    let _ = writeln!(out, "{GENERATED}");

    for section in outputs {
        let _ = write!(out, "\n## {}\n\n", section.name);
        // Enough backticks to hold whatever is inside.
        let longest = section
            .body
            .lines()
            .filter(|l| l.trim_start().starts_with("```"))
            .map(|l| l.trim_start().chars().take_while(|c| *c == '`').count())
            .max()
            .unwrap_or(0);
        let fence = "`".repeat(longest.max(2) + 1);
        let _ = writeln!(out, "{fence}{}", section.language);
        if !section.body.is_empty() {
            out.push_str(section.body.trim_end());
            out.push('\n');
        }
        let _ = writeln!(out, "{fence}");
    }
    out
}
