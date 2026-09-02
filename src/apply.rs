//! Building the named tree.
//!
//! **Nothing in the library is ever touched, and nothing in the destination
//! that this program did not put there.** What it built, it records; what it
//! records, it may remove again when the plan stops naming it. Everything else
//! is somebody else's and is left where it is.
//!
//! Within a folder it built, files are still only added,
//! so a second run after adding one recording does one thing, and a run that
//! meets something unexpected describes it rather than deciding on your
//! behalf.
//!
//! The modes differ in what they can do about a name that is already taken,
//! and the difference is forced. [`Link::Symlink`] puts one link where a
//! folder would go, so anything already at that name is in the way and it
//! stops. The other two build the folder and fill it, so they merge into one
//! that is already there — which is exactly what makes a second run cheap —
//! while still refusing to write over any file inside it.
//!
//! Most of what can go wrong is knowable before a single link is made, and
//! [`preflight`] is where that happens: a plan that could not be acted on, a
//! destination inside its own source, a hard link asked to cross a disk. Half
//! a tree is worse than none, so those are refused up front rather than
//! discovered a thousand entries in.

use crate::config::Link;
use crate::plan::Plan;
use crate::state;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Outcome {
    /// Made, or would be made.
    Created,
    /// Already exactly this. Left alone.
    Skipped,
    /// Something else is there. Left alone.
    Conflict(String),
    /// The system refused.
    Failed(String),
}

#[derive(Debug)]
pub struct Entry {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub outcome: Outcome,
}

#[derive(Debug, Default)]
pub struct Applied {
    pub entries: Vec<Entry>,
}

impl Applied {
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut c = (0, 0, 0);
        for e in &self.entries {
            match e.outcome {
                Outcome::Created => c.0 += 1,
                Outcome::Skipped => c.1 += 1,
                Outcome::Conflict(_) | Outcome::Failed(_) => c.2 += 1,
            }
        }
        c
    }

    /// Did anything go wrong that the caller should hear about?
    pub fn troubled(&self) -> bool {
        self.entries
            .iter()
            .any(|e| matches!(e.outcome, Outcome::Conflict(_) | Outcome::Failed(_)))
    }
}

/// Everything that can be known before writing anything.
pub fn preflight(
    plan: &Plan,
    source: &Path,
    destination: &Path,
    link: Link,
    strict: bool,
) -> Result<(), String> {
    if plan.blocked(strict) {
        return Err(
            "the plan has problems, and half a tree is worse than none — fix them first"
                .to_string(),
        );
    }
    if plan.rows.is_empty() {
        return Err("there is nothing to build".to_string());
    }

    let src = source
        .canonicalize()
        .map_err(|e| format!("cannot read {}: {e}", source.display()))?;
    // The destination need not exist yet, so it cannot simply be canonicalized
    // — and resolving only as far as its nearest existing ancestor would make
    // "inside the library" indistinguishable from "the library itself".
    let dst = absolutize(destination).ok_or_else(|| {
        format!(
            "nowhere to write: no part of {} exists",
            destination.display()
        )
    })?;

    // The same folder is a real thing to want — a library that reorganises
    // itself in place — but it needs moving rather than linking, and moving
    // is not here yet. Refuse it plainly rather than building a tree of links
    // inside the folder they point at.
    if src == dst {
        return Err(format!(
            "the destination and the library are both {} — organising a folder in \
             place is not supported yet, since it needs moving rather than linking",
            src.display()
        ));
    }

    // One inside the other means building the tree changes what the next scan
    // sees, which is a mess nobody wants to unpick.
    if dst.starts_with(&src) {
        return Err(format!(
            "the destination {} is inside the library being scanned ({}) — \
             building there would put the new tree in the way of the next scan",
            dst.display(),
            src.display()
        ));
    }
    if src.starts_with(&dst) {
        return Err(format!(
            "the library {} is inside the destination {} — building there would \
             write over what it is reading",
            src.display(),
            dst.display()
        ));
    }

    if link == Link::Hardlink {
        // Only something that exists has a disk to be on, so the question goes
        // to the nearest part of the destination that is already there.
        let anchor = nearest_existing(&dst).unwrap_or(dst.clone());
        if let Some(why) = crossing_filesystems(&src, &anchor) {
            return Err(why);
        }
    }
    // A destination we have never written to has to be empty, and one we have
    // is identified by its own record. Between them there is no third case
    // where something already here might be mistaken for something we made,
    // which is what lets removal be safe at all.
    match state::load(destination) {
        Ok(_) => {}
        Err(state::StateError::Unreadable(why)) => return Err(why),
        Err(state::StateError::Absent) => {
            let occupied = destination
                .exists()
                .then(|| state::is_empty_but_ours(destination).map(|empty| !empty))
                .transpose()
                .map_err(|e| format!("cannot read {}: {e}", destination.display()))?
                .unwrap_or(false);
            if occupied {
                return Err(format!(
                    "{} already has things in it, and no record of this program having \
                     put them there.\n    An empty folder, or one built by an earlier run, \
                     is what this can work with — otherwise there is no way to tell what \
                     would be safe to remove later.",
                    destination.display()
                ));
            }
        }
    }

    Ok(())
}

/// `path` made absolute and normalized, whether or not it exists yet.
///
/// The part that exists is resolved properly — following any symlink, so a
/// destination reached through one is still recognised as being inside the
/// library — and the part that does not is appended as written.
fn absolutize(path: &Path) -> Option<PathBuf> {
    let anchor = nearest_existing(path)?;
    let rest = path.strip_prefix(&anchor).ok()?;
    Some(anchor.canonicalize().ok()?.join(rest))
}

/// The nearest ancestor of `path` that exists, `path` itself included.
fn nearest_existing(path: &Path) -> Option<PathBuf> {
    let mut p = path;
    loop {
        if p.exists() {
            return Some(p.to_path_buf());
        }
        p = p.parent()?;
    }
}

/// Are these on different filesystems? Hard links cannot span them.
///
/// Worth answering before writing rather than after: the failure otherwise
/// arrives as `Invalid cross-device link` on the first file, with a tree half
/// built and nothing saying which two disks were meant.
#[cfg(unix)]
fn crossing_filesystems(src: &Path, dst: &Path) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    let a = std::fs::metadata(src).ok()?.dev();
    let b = std::fs::metadata(dst).ok()?.dev();
    (a != b).then(|| {
        format!(
            "`link = \"hardlink\"` cannot reach from {} to {}: they are on different \
             disks, and a hard link is a second name for a file on the disk it \
             already lives on. Use `link = \"symlink\"` to point at it instead, or \
             `link = \"copy\"` to have two of it.",
            src.display(),
            dst.display()
        )
    })
}

#[cfg(not(unix))]
fn crossing_filesystems(_src: &Path, _dst: &Path) -> Option<String> {
    // No cheap device id here; the per-file failure has to carry it instead.
    None
}

#[cfg(unix)]
fn symlink_to(target: &Path, at: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

#[cfg(windows)]
fn symlink_to(target: &Path, at: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, at)
}

/// Build the tree, or work out what building it would do.
/// A link on the way down to `at`, if there is one.
///
/// Every path operation here follows links — `create_dir_all` makes
/// directories through one, and `symlink_metadata` on the far end reports on
/// whatever it lands in — so a link sitting in the destination silently
/// redirects a build into whatever it points at. For this program that is
/// nearly always the library being read, which is the one place nothing may be
/// written.
///
/// [`crate::plan`] refuses a plan whose own folders nest, but it can only see
/// this run. A link left by an earlier one, under a different template, is
/// still here — so this is checked again at the point of writing.
fn passes_through_link(destination: &Path, at: &Path) -> Option<PathBuf> {
    let rest = at.strip_prefix(destination).ok()?;
    let mut components: Vec<_> = rest.components().collect();
    // The last is the entry itself; a link there is an ordinary conflict, and
    // is already reported as one.
    components.pop();

    let mut here = destination.to_path_buf();
    for component in components {
        here.push(component);
        match std::fs::symlink_metadata(&here) {
            Ok(meta) if meta.is_symlink() => return Some(here),
            Ok(_) => {}
            // Not there yet, so nothing below it can be either.
            Err(_) => return None,
        }
    }
    None
}

pub fn apply(plan: &Plan, destination: &Path, link: Link, dry_run: bool) -> Applied {
    let mut out = Applied::default();

    for row in &plan.rows {
        let mut at = destination.to_path_buf();
        for segment in &row.segments {
            at.push(segment);
        }
        let source = row.marker.directory.clone();
        let outcome = match passes_through_link(destination, &at) {
            Some(link_at) => Outcome::Conflict(format!(
                "{} is a link to somewhere else, so building this inside it would write \
                 outside {} — most likely into the library itself",
                link_at.display(),
                destination.display()
            )),
            None => match link {
                Link::Symlink => one_symlink(&source, &at, dry_run),
                Link::Hardlink | Link::Copy => mirror(&source, &at, link, dry_run),
            },
        };
        out.entries.push(Entry {
            source,
            destination: at,
            outcome,
        });
    }
    out
}

fn one_symlink(source: &Path, at: &Path, dry_run: bool) -> Outcome {
    let target = match source.canonicalize() {
        Ok(t) => t,
        Err(e) => return Outcome::Failed(format!("cannot read {}: {e}", source.display())),
    };
    // `exists` follows a link, which would report a broken one as absent.
    match std::fs::symlink_metadata(at) {
        Ok(meta) if meta.is_symlink() => {
            return match std::fs::read_link(at) {
                Ok(existing) if existing == target => Outcome::Skipped,
                Ok(existing) => Outcome::Conflict(format!(
                    "a link is already there, pointing at {}",
                    existing.display()
                )),
                Err(e) => Outcome::Failed(format!("cannot read the link that is there: {e}")),
            }
        }
        Ok(meta) => {
            return Outcome::Conflict(format!(
                "{} is already there",
                if meta.is_dir() { "a folder" } else { "a file" }
            ))
        }
        Err(_) => {}
    }
    if dry_run {
        return Outcome::Created;
    }
    if let Some(parent) = at.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Outcome::Failed(format!("cannot make {}: {e}", parent.display()));
        }
    }
    match symlink_to(&target, at) {
        Ok(()) => Outcome::Created,
        Err(e) => Outcome::Failed(explain_io(&e)),
    }
}

/// Recreate the folder and treat every file in it separately.
fn mirror(source: &Path, at: &Path, link: Link, dry_run: bool) -> Outcome {
    let mut made = 0usize;
    let mut skipped = 0usize;
    for entry in ignore::WalkBuilder::new(source)
        .standard_filters(false)
        .sort_by_file_name(|a, b| a.cmp(b))
        .build()
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => return Outcome::Failed(format!("cannot read: {e}")),
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let relative = match entry.path().strip_prefix(source) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let target = at.join(relative);
        if target.exists() {
            skipped += 1;
            continue;
        }
        if dry_run {
            made += 1;
            continue;
        }
        if let Some(parent) = target.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Outcome::Failed(format!("cannot make {}: {e}", parent.display()));
            }
        }
        let result = match link {
            Link::Hardlink => std::fs::hard_link(entry.path(), &target),
            _ => std::fs::copy(entry.path(), &target).map(|_| ()),
        };
        if let Err(e) = result {
            return Outcome::Failed(format!("{}: {}", relative.display(), explain_io(&e)));
        }
        made += 1;
    }
    match (made, skipped) {
        (0, 0) => Outcome::Skipped,
        (0, _) => Outcome::Skipped,
        _ => Outcome::Created,
    }
}

/// Say what the system's own wording will not.
fn explain_io(e: &std::io::Error) -> String {
    let text = e.to_string();
    if text.contains("cross-device") {
        return format!(
            "{text} — a hard link cannot span two disks. Use `link = \"symlink\"` or \
             `link = \"copy\"`."
        );
    }
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        return format!("{text} — check you can write there");
    }
    text
}

/// What was removed, and what was left alone.
#[derive(Debug, Default)]
pub struct Pruned {
    pub removed: Vec<PathBuf>,
    /// Recorded as ours, but no longer looking like it. Left where it is.
    pub changed: Vec<(PathBuf, String)>,
    pub failed: Vec<(PathBuf, String)>,
}

/// Remove what we built that the plan no longer names.
///
/// Only entries in the record are ever candidates, so nothing anyone else put
/// in the destination can be removed however the plan changes. Each is checked
/// against what it was recorded as before it goes: a folder that has become
/// something else is somebody's doing, and the safe answer is to leave it and
/// say so.
pub fn prune(state: &state::State, plan: &Plan, destination: &Path, dry_run: bool) -> Pruned {
    let wanted: std::collections::BTreeSet<&str> =
        plan.rows.iter().map(|r| r.path.as_str()).collect();
    let mut out = Pruned::default();

    for built in &state.entries {
        if wanted.contains(built.path.as_str()) {
            continue;
        }
        let at = destination.join(&built.path);
        if let Err(why) = still_ours(&at, built) {
            // Missing is not a complaint: someone removing it by hand is the
            // same outcome we wanted.
            if at.symlink_metadata().is_ok() {
                out.changed.push((at, why));
            }
            continue;
        }
        if dry_run {
            out.removed.push(at);
            continue;
        }
        let result = if at
            .symlink_metadata()
            .map(|m| m.is_symlink())
            .unwrap_or(false)
        {
            std::fs::remove_file(&at)
        } else {
            std::fs::remove_dir_all(&at)
        };
        match result {
            Ok(()) => {
                sweep_empty_parents(&at, destination);
                out.removed.push(at);
            }
            Err(e) => out.failed.push((at, e.to_string())),
        }
    }
    out
}

/// Whether what is there is still what we recorded making.
fn still_ours(at: &Path, built: &state::Built) -> Result<(), String> {
    let meta = at
        .symlink_metadata()
        .map_err(|_| "it is not there any more".to_string())?;
    match built.link {
        Link::Symlink => {
            if !meta.is_symlink() {
                return Err("it was a link and is now a folder".to_string());
            }
            Ok(())
        }
        Link::Hardlink | Link::Copy => {
            if meta.is_symlink() {
                return Err("it was a folder and is now a link".to_string());
            }
            if !meta.is_dir() {
                return Err("it was a folder and is now a file".to_string());
            }
            Ok(())
        }
    }
}

/// Take away the folders an entry leaves behind, up to but not including the
/// destination itself, so a tree does not fill with empty composers.
fn sweep_empty_parents(from: &Path, destination: &Path) {
    let mut at = from.parent().map(Path::to_path_buf);
    while let Some(dir) = at {
        if dir == destination || !dir.starts_with(destination) {
            return;
        }
        match std::fs::read_dir(&dir).map(|mut d| d.next().is_none()) {
            Ok(true) => {
                if std::fs::remove_dir(&dir).is_err() {
                    return;
                }
            }
            _ => return,
        }
        at = dir.parent().map(Path::to_path_buf);
    }
}

/// What a run should record as having built.
pub fn built(plan: &Plan, done: &Applied, link: Link) -> Vec<state::Built> {
    plan.rows
        .iter()
        .zip(&done.entries)
        .filter(|(_, e)| matches!(e.outcome, Outcome::Created | Outcome::Skipped))
        .map(|(row, _)| state::Built {
            path: row.path.clone(),
            id: row.marker.id.clone(),
            variant: row.marker.variant.clone(),
            link,
        })
        .collect()
}
