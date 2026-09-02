//! What this program has built in a destination.
//!
//! The tree is derived, so keeping it right means removing folders as well as
//! making them — a recording leaves the library, a template changes, and what
//! was built for it no longer belongs. Working out which of those is ours by
//! looking at the filesystem does not survive contact with the modes: a
//! symlink names its own target and can be checked, a hard link knows only
//! that its inode has another name somewhere, and a copy is indistinguishable
//! from a file someone put there by hand.
//!
//! So it is written down instead. Everything removed was recorded here as
//! having been made, which is a fact rather than an inference, and the same
//! rule holds in all three modes.
//!
//! The file lives at the root of the destination, not inside each folder: in
//! symlink mode an entry *is* a link and has no inside, and one file at the
//! top also says "this tree is managed" — which is what makes it safe to
//! refuse a destination we know nothing about.

use crate::config::Link;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Hidden, so the tree it describes stays clean to look at.
pub const FILE_NAME: &str = ".findopera-state.json";

/// One folder this program made.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Built {
    /// Where it was built, relative to the destination, so that moving the
    /// whole tree does not invalidate the record.
    pub path: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// How it was made, which decides what "still ours" looks like later.
    pub link: Link,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    /// What wrote this, for a future version that has to read an older one.
    pub version: u32,
    pub entries: Vec<Built>,
}

const VERSION: u32 = 1;

#[derive(Debug)]
pub enum StateError {
    /// Nothing there. Not a problem on its own.
    Absent,
    Unreadable(String),
}

pub fn path(destination: &Path) -> PathBuf {
    destination.join(FILE_NAME)
}

/// Read what was built here before.
pub fn load(destination: &Path) -> Result<State, StateError> {
    let at = path(destination);
    let text = match std::fs::read_to_string(&at) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(StateError::Absent),
        Err(e) => {
            return Err(StateError::Unreadable(format!(
                "cannot read {}: {e}",
                at.display()
            )))
        }
    };
    serde_json::from_str(&text).map_err(|e| {
        // Refusing is the safe direction: a record we cannot read is a record
        // we cannot delete against.
        StateError::Unreadable(format!("{} is not readable as state: {e}", at.display()))
    })
}

/// Write down what is there now.
pub fn save(destination: &Path, entries: Vec<Built>) -> Result<(), String> {
    let at = path(destination);
    let state = State {
        version: VERSION,
        entries,
    };
    let text = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("cannot describe what was built: {e}"))?;
    std::fs::write(&at, text).map_err(|e| format!("cannot write {}: {e}", at.display()))
}

/// Whether a directory holds anything but our own state file.
///
/// A destination we have never written to must be empty, so that nothing
/// already there can ever be inside the set of things we might remove. After
/// the first run it is not empty any more, and the state file is what says we
/// are allowed to be there.
pub fn is_empty_but_ours(destination: &Path) -> std::io::Result<bool> {
    let mut entries = std::fs::read_dir(destination)?;
    Ok(entries.all(|e| e.map(|e| e.file_name() == FILE_NAME).unwrap_or(false)))
}

/// The entries recorded here that the plan no longer names.
pub fn orphans<'a>(state: &'a State, wanted: &BTreeMap<String, ()>) -> Vec<&'a Built> {
    state
        .entries
        .iter()
        .filter(|e| !wanted.contains_key(&e.path))
        .collect()
}
