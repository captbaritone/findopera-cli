//! Building the tree, on a throwaway directory.
//!
//! Offline: the recordings come from the same captured responses the plan
//! tests use. Symlink cases are unix-only — Windows needs a privilege for
//! them that CI does not have — but the guards and the file-by-file modes are
//! exercised everywhere, and the guards are the part that matters most, since
//! they are what stops a half-built tree.

use findopera::config::Link;
use findopera::model::{Recording, FIELDS};
use findopera::{apply, plan, scan, FieldDoc, Template};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn recordings() -> BTreeMap<String, Recording> {
    let raw = include_str!("fixtures/plan-recordings.json");
    let list: Vec<Recording> = serde_json::from_str(raw).expect("the captured response");
    list.into_iter().map(|r| (r.id.to_string(), r)).collect()
}

/// A source library and a destination beside it, both thrown away at the end.
struct Fixture {
    root: PathBuf,
    source: PathBuf,
    destination: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let root =
            std::env::temp_dir().join(format!("findopera-apply-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let source = root.join("library");
        for (dir, id) in [("rip-a", "75"), ("rip-b", "10655")] {
            let d = source.join(dir);
            fs::create_dir_all(&d).expect("source dirs");
            fs::write(d.join(format!("findopera-{id}.txt")), "").expect("marker");
            fs::write(d.join("01.mp3"), "audio").expect("a track");
        }
        Fixture {
            destination: root.join("named"),
            source,
            root,
        }
    }

    /// Another folder for a recording already in the library.
    fn duplicate(&self, dir: &str, id: &str) {
        let d = self.source.join(dir);
        fs::create_dir_all(&d).expect("source dirs");
        fs::write(d.join(format!("findopera-{id}.txt")), "").expect("marker");
    }

    /// The pieces a plan borrows from, which have to outlive it.
    fn parts(&self, template: &str) -> (scan::Report, BTreeMap<String, Recording>, Template) {
        let mut schema: Vec<FieldDoc> = FIELDS.to_vec();
        schema.push(scan::VARIANT);
        let tmpl = Template::parse(template, &schema).expect("template parses");
        (
            scan::scan(&self.source, false, &scan::Ignore::default()),
            recordings(),
            tmpl,
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const T: &str = r"{{composer.lastName}}/{{opera.title}} \[{{id}}\]";

// ---- the guards, which run before anything is written ---------------------

#[test]
fn a_destination_that_is_the_library_is_refused() {
    let f = Fixture::new("same");
    let parts = f.parts(T);
    let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
    let why = apply::preflight(&plan, &f.source, &f.source, Link::Symlink, false)
        .expect_err("the same folder is not supported yet");
    assert!(why.contains("not supported yet"), "got: {why}");
}

#[test]
fn a_destination_inside_the_library_is_refused() {
    // It need not exist yet, which is exactly when this is easy to get wrong:
    // resolving only as far as what does exist would call it the library
    // itself.
    let f = Fixture::new("inside");
    let parts = f.parts(T);
    let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
    let inside = f.source.join("named");
    let why = apply::preflight(&plan, &f.source, &inside, Link::Symlink, false)
        .expect_err("inside the library is refused");
    assert!(why.contains("is inside the library"), "got: {why}");
}

#[test]
fn a_library_inside_the_destination_is_refused() {
    let f = Fixture::new("outside");
    let parts = f.parts(T);
    let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
    let why = apply::preflight(&plan, &f.source, &f.root, Link::Symlink, false)
        .expect_err("the library sits inside this");
    assert!(why.contains("is inside the destination"), "got: {why}");
}

#[test]
fn a_plan_two_folders_cannot_both_satisfy_is_refused() {
    // Two folders for one recording, and a template with no {{variant}} to
    // tell them apart. Building half of that is worse than building none.
    let f = Fixture::new("blocked");
    f.duplicate("rip-a-again", "75");
    let parts = f.parts(T);
    let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
    let why = apply::preflight(&plan, &f.source, &f.destination, Link::Symlink, false)
        .expect_err("a clashing plan cannot be built");
    assert!(why.contains("half a tree is worse than none"), "got: {why}");
}

#[test]
fn a_clean_plan_passes() {
    let f = Fixture::new("clean");
    let parts = f.parts(T);
    let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
    apply::preflight(&plan, &f.source, &f.destination, Link::Symlink, false)
        .expect("nothing wrong");
}

// ---- building -------------------------------------------------------------

#[test]
fn a_dry_run_writes_nothing() {
    let f = Fixture::new("dry");
    let parts = f.parts(T);
    let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
    let done = apply::apply(&plan, &f.destination, Link::Copy, true);
    assert_eq!(done.counts().0, 2, "two would be built");
    assert!(!f.destination.exists(), "and nothing was");
}

#[test]
fn copying_gives_the_destination_its_own_files() {
    let f = Fixture::new("copy");
    let parts = f.parts(T);
    let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
    let done = apply::apply(&plan, &f.destination, Link::Copy, false);
    assert_eq!(done.counts().0, 2);
    let track = f.destination.join("Britten/Billy Budd [75]/01.mp3");
    assert_eq!(fs::read_to_string(&track).expect("copied"), "audio");
}

#[test]
fn building_again_changes_nothing() {
    let f = Fixture::new("again");
    let parts = f.parts(T);
    let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
    apply::apply(&plan, &f.destination, Link::Copy, false);
    let second = apply::apply(&plan, &f.destination, Link::Copy, false);
    let (made, skipped, trouble) = second.counts();
    assert_eq!((made, skipped, trouble), (0, 2, 0), "all already there");
}

#[test]
fn copying_into_an_occupied_folder_adds_without_overwriting() {
    // The mirroring modes merge, which is what lets a second run fill in one
    // new recording. What they never do is write over a file that is there.
    let f = Fixture::new("inway");
    let parts = f.parts(T);
    let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
    let taken = f.destination.join("Britten/Billy Budd [75]");
    fs::create_dir_all(&taken).expect("something else");
    fs::write(taken.join("mine.txt"), "do not touch").expect("write");
    fs::write(taken.join("01.mp3"), "mine too").expect("write");

    apply::apply(&plan, &f.destination, Link::Copy, false);
    assert_eq!(
        fs::read_to_string(taken.join("mine.txt")).expect("still there"),
        "do not touch",
        "a file of theirs is left alone"
    );
    assert_eq!(
        fs::read_to_string(taken.join("01.mp3")).expect("still there"),
        "mine too",
        "and one whose name we would have used is not overwritten"
    );
}

#[cfg(unix)]
mod links {
    use super::*;

    #[test]
    fn a_symlink_will_not_displace_what_is_there() {
        // Unlike the mirroring modes, this one cannot merge: a link and a
        // folder cannot occupy one name, so it stops and says what is there.
        let f = Fixture::new("inway-link");
        let parts = f.parts(T);
        let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
        let taken = f.destination.join("Britten/Billy Budd [75]");
        fs::create_dir_all(&taken).expect("something else");

        let done = apply::apply(&plan, &f.destination, Link::Symlink, false);
        assert!(done.troubled(), "it should have said so");
        assert!(taken.is_dir(), "and left the folder alone");
    }

    #[test]
    fn a_symlink_points_at_the_folder_itself() {
        let f = Fixture::new("symlink");
        let parts = f.parts(T);
        let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
        let done = apply::apply(&plan, &f.destination, Link::Symlink, false);
        assert_eq!(done.counts().0, 2);
        let at = f.destination.join("Britten/Billy Budd [75]");
        assert!(at.symlink_metadata().expect("a link").is_symlink());
        assert_eq!(
            fs::read_link(&at).expect("target"),
            f.source.join("rip-a").canonicalize().expect("real source")
        );
    }

    #[test]
    fn a_track_added_later_appears_through_the_link() {
        // The reason to link the folder rather than mirror it.
        let f = Fixture::new("live");
        let parts = f.parts(T);
        let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
        apply::apply(&plan, &f.destination, Link::Symlink, false);
        fs::write(f.source.join("rip-a/02.mp3"), "more").expect("a new track");
        let at = f.destination.join("Britten/Billy Budd [75]/02.mp3");
        assert_eq!(fs::read_to_string(at).expect("through the link"), "more");
    }

    #[test]
    fn a_hard_link_shares_the_file() {
        use std::os::unix::fs::MetadataExt;
        let f = Fixture::new("hard");
        let parts = f.parts(T);
        let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
        apply::apply(&plan, &f.destination, Link::Hardlink, false);
        let a = fs::metadata(f.source.join("rip-a/01.mp3")).expect("source");
        let b = fs::metadata(f.destination.join("Britten/Billy Budd [75]/01.mp3")).expect("built");
        assert_eq!(a.ino(), b.ino(), "one file, two names");
    }
}

#[test]
fn a_link_left_by_an_earlier_run_does_not_become_a_way_into_the_library() {
    // The danger is specific: every path call here follows links, so a link
    // sitting in the destination redirects a build into whatever it points at
    // — and what it points at is the library. `plan` refuses folders that nest
    // within one run, but a link left by a previous run under a different
    // template is invisible to it.
    let f = Fixture::new("through-link");
    let (report, recs, tmpl) = f.parts(T);
    let p = plan::plan(&report.markers, &recs, &tmpl);
    let row = p.rows.first().expect("a row");

    // Stand in for that earlier run: the first path segment is already a link
    // pointing back into the library.
    let first = &row.segments[0];
    fs::create_dir_all(&f.destination).expect("destination");
    let into_library = f.source.join("rip-a");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&into_library, f.destination.join(first)).expect("the old link");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&into_library, f.destination.join(first))
        .expect("the old link");

    let done = apply::apply(&p, &f.destination, Link::Symlink, false);

    assert!(
        done.troubled(),
        "building through a link must not be a success"
    );
    let why = done
        .entries
        .iter()
        .find_map(|e| match &e.outcome {
            apply::Outcome::Conflict(why) => Some(why.clone()),
            _ => None,
        })
        .expect("a conflict naming the link");
    assert!(why.contains("link"), "got: {why}");

    // The point of all of it: the library is untouched.
    let leaked: Vec<_> = fs::read_dir(&into_library)
        .expect("the library folder")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n != "01.mp3" && !n.starts_with("findopera-"))
        .collect();
    assert!(leaked.is_empty(), "written into the library: {leaked:?}");
}
