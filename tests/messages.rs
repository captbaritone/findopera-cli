//! House style, checked against what the program actually says.
//!
//! The expectations under `tests/snapshots/` hold every message a case has
//! ever drawn out, already reviewed and checked in. That makes them a corpus:
//! rather than a written convention nobody rereads, the conventions are
//! asserted here against the real text.
//!
//! What it can see is what the cases exercise. A message no case reaches is
//! not policed — which is an argument for another case rather than against
//! the rule.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One message: the `findopera:` line, and the lines indented under it.
struct Message {
    first: String,
    rest: Vec<String>,
    case: String,
}

fn expectations(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            expectations(&path, into);
        } else if path.to_string_lossy().ends_with(".expected.md") {
            into.push(path);
        }
    }
}

/// Every message the cases have drawn out of the program.
fn messages() -> Vec<Message> {
    let mut files = Vec::new();
    expectations(Path::new("tests/snapshots"), &mut files);
    files.sort();
    assert!(!files.is_empty(), "no expectations to read");

    let mut out = Vec::new();
    for path in &files {
        let case = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let text = std::fs::read_to_string(path).expect("an expectation");
        let Some(after) = text.split_once("## stderr\n\n```\n") else {
            continue;
        };
        let Some((block, _)) = after.1.split_once("\n```") else {
            continue;
        };
        for line in block.lines() {
            if line.trim().is_empty() || line.starts_with("$ ") {
                continue;
            }
            match line.strip_prefix("findopera: ") {
                Some(first) => out.push(Message {
                    first: first.to_string(),
                    rest: Vec::new(),
                    case: case.clone(),
                }),
                None => {
                    if let Some(last) = out.last_mut() {
                        last.rest.push(line.to_string());
                    }
                }
            }
        }
    }
    out
}

#[test]
fn every_line_belongs_to_a_message() {
    // Something printed flush left that does not begin `findopera:` reads as
    // having escaped rather than been said — which is what the settings
    // parser's own complaint used to look like.
    let mut loose = BTreeSet::new();
    for message in messages() {
        for line in &message.rest {
            if !line.starts_with(' ') {
                loose.insert(format!("{}: {line}", message.case));
            }
        }
    }
    assert!(
        loose.is_empty(),
        "these lines are not indented under the message they belong to:\n{}",
        loose.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn a_continuation_does_not_open_with_a_pronoun() {
    // The thing it refers to is on another line, so the line cannot be read
    // where it is usually met: in a log, in a grep, in half of a CI page.
    const VAGUE: [&str; 6] = ["it ", "this ", "that ", "they ", "those ", "these "];
    let mut dangling = BTreeSet::new();
    for message in messages() {
        for line in &message.rest {
            let text = line.trim().to_lowercase();
            if VAGUE.iter().any(|p| text.starts_with(p)) {
                dangling.insert(format!("{}: {}", message.case, line.trim()));
            }
        }
    }
    assert!(
        dangling.is_empty(),
        "these lines begin with a word for something named elsewhere:\n{}",
        dangling.into_iter().collect::<Vec<_>>().join("\n")
    );
}

impl Message {
    /// A message whose first line ends in a colon or a dash introduces what
    /// follows — a command to run, an example, a list of paths. Those lines
    /// are not prose and are not punctuated like it.
    fn introduces(&self) -> bool {
        let first = self.first.trim_end();
        first.ends_with(':') || first.ends_with('—')
    }

    /// A diagnostic carries its own frame: the text objected to, an underline
    /// beneath it, and a `help:` line. None of that is a sentence.
    fn is_diagnostic(&self) -> bool {
        self.rest.iter().any(|l| {
            let t = l.trim();
            t.starts_with("help:")
                || t.starts_with("see `")
                || (!t.is_empty() && t.chars().all(|c| c == '^'))
        })
    }

    /// The lines of it that are prose, and so subject to prose's rules.
    fn prose(&self) -> Vec<&str> {
        let mut out = vec![self.first.trim_end()];
        if !self.introduces() && !self.is_diagnostic() {
            out.extend(
                self.rest
                    .iter()
                    .map(|l| l.trim())
                    .filter(|l| l.contains(' ')),
            );
        }
        out
    }
}

/// Does this run to more than one sentence?
fn several_sentences(text: &str) -> bool {
    text.char_indices()
        .any(|(i, c)| ".!?".contains(c) && text[i + 1..].starts_with(' '))
}

#[test]
fn a_line_of_several_sentences_ends_like_prose() {
    // One sentence stops without a full stop, as a label does. Two are prose,
    // and prose ends — otherwise the last sentence reads as cut off.
    let mut wrong = BTreeSet::new();
    for message in messages() {
        let first = message.first.trim_end();
        if message.introduces() || message.is_diagnostic() {
            continue;
        }
        if several_sentences(first) && !first.ends_with('.') {
            wrong.insert(format!(
                "{}: {}",
                message.case,
                &first[..first.len().min(90)]
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} message(s) run to several sentences and stop without a full stop:\n{}",
        wrong.len(),
        wrong.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn a_message_punctuates_all_of_itself_or_none() {
    // A message spread over lines is one piece of writing. Half of it ending
    // in full stops and half not makes the punctuation look like it means
    // something.
    let mut mixed = BTreeSet::new();
    for message in messages() {
        let prose = message.prose();
        if prose.len() < 2 {
            continue;
        }
        let stopped = prose.iter().filter(|l| l.ends_with('.')).count();
        if stopped != 0 && stopped != prose.len() {
            mixed.insert(format!(
                "{}: {}",
                message.case,
                &prose[0][..prose[0].len().min(80)]
            ));
        }
    }
    assert!(
        mixed.is_empty(),
        "{} message(s) punctuate some lines and not others:\n{}",
        mixed.len(),
        mixed.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn a_message_starts_in_lower_case() {
    // It follows `findopera: `, so it is the middle of a line rather than the
    // start of one. A capital there reads as a heading.
    let mut shouty = BTreeSet::new();
    for message in messages() {
        let first = message.first.trim_start();
        let opens_with_a_word = first
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() && c.is_uppercase());
        // A path, a name the program produced, or a word that is a proper
        // noun wherever it appears.
        let excused = ["TOML", "GitHub", "FindOpera", "Windows"]
            .iter()
            .any(|w| first.starts_with(w));
        if opens_with_a_word && !excused {
            shouty.insert(format!("{}: {first}", message.case));
        }
    }
    assert!(
        shouty.is_empty(),
        "these open with a capital, mid-line:\n{}",
        shouty.into_iter().collect::<Vec<_>>().join("\n")
    );
}
