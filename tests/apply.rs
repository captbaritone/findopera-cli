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

        // Where it lands, not how it is spelled. It is written relative so
        // that a share reachable by two names reads correctly from both.
        assert_eq!(
            at.canonicalize().expect("it resolves"),
            f.source.join("rip-a").canonicalize().expect("real source")
        );
        assert!(
            fs::read_link(&at).expect("target").is_relative(),
            "an absolute target can only name one of a share's names"
        );
    }

    #[test]
    fn a_symlinked_tree_survives_being_moved() {
        // The other thing relative targets buy: the whole share can be
        // renamed, or mounted somewhere else, and the links still land.
        let f = Fixture::new("moved");
        let parts = f.parts(T);
        let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
        apply::apply(&plan, &f.destination, Link::Symlink, false);

        let moved = f.root.join("elsewhere");
        fs::create_dir_all(&moved).expect("somewhere to move to");
        fs::rename(f.root.join("library"), moved.join("library")).expect("move the library");
        fs::rename(&f.destination, moved.join("named")).expect("move the tree with it");

        let at = moved.join("named/Britten/Billy Budd [75]");
        assert!(at.is_dir(), "the link still resolves after the move");
        assert!(
            at.canonicalize()
                .expect("resolves")
                .starts_with(moved.canonicalize().expect("moved")),
            "and lands inside the new location rather than the old one"
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

    #[test]
    fn a_clone_is_its_own_file_with_the_same_contents() {
        // The point of reflinking, and what separates it from both neighbours:
        // a hard link is one file under two names, a copy is two files and two
        // lots of disk, and a clone is two real files that happen to share
        // their extents until one of them is written to. So the inode must
        // differ -- otherwise it linked rather than cloned -- while the bytes
        // must not.
        use std::os::unix::fs::MetadataExt;
        let f = Fixture::new("clone");
        let parts = f.parts(T);
        let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
        apply::apply(&plan, &f.destination, Link::Reflink, false);

        let built = f.destination.join("Britten/Billy Budd [75]/01.mp3");
        if !built.exists() {
            // Cloning is the one mode a filesystem may simply not offer. On
            // ext4 or tmpfs there is nothing to assert and nothing wrong.
            eprintln!("skipping: this filesystem cannot clone a file");
            return;
        }

        let a = fs::metadata(f.source.join("rip-a/01.mp3")).expect("source");
        let b = fs::metadata(&built).expect("built");
        assert_ne!(
            a.ino(),
            b.ino(),
            "a clone is its own file, not a second name"
        );
        assert_eq!(b.nlink(), 1, "and nothing else points at it");
        assert_eq!(
            fs::read_to_string(f.source.join("rip-a/01.mp3")).expect("source bytes"),
            fs::read_to_string(&built).expect("built bytes"),
            "sharing extents means sharing contents"
        );
    }

    #[test]
    fn writing_to_a_clone_leaves_the_original_alone() {
        // The half of copy-on-write that matters for a library: the whole
        // point of not paying for the second copy is that it is still a second
        // copy. If editing one changed the other this would be a hard link
        // wearing a different name.
        let f = Fixture::new("clone-cow");
        let parts = f.parts(T);
        let plan = plan::plan(&parts.0.markers, &parts.1, &parts.2);
        apply::apply(&plan, &f.destination, Link::Reflink, false);

        let built = f.destination.join("Britten/Billy Budd [75]/01.mp3");
        if !built.exists() {
            eprintln!("skipping: this filesystem cannot clone a file");
            return;
        }

        let source = f.source.join("rip-a/01.mp3");
        let before = fs::read_to_string(&source).expect("source before");
        fs::write(&built, "rewritten").expect("write to the clone");
        assert_eq!(
            fs::read_to_string(&source).expect("source after"),
            before,
            "the original is untouched"
        );
        assert_eq!(
            fs::read_to_string(&built).expect("clone after"),
            "rewritten",
            "and the clone kept the write"
        );
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

// ---- what a run records, and what it may take away --------------------------

/// Build once, so the destination has a record and a tree.
fn establish(f: &Fixture, template: &str, link: Link) -> usize {
    let (report, recs, tmpl) = f.parts(template);
    let p = plan::plan(&report.markers, &recs, &tmpl);
    let done = apply::apply(&p, &f.destination, link, false);
    let built = apply::built(&p, &done, link);
    findopera::state::save(&f.destination, built).expect("state");
    p.rows.len()
}

#[test]
fn a_destination_with_things_in_it_and_no_record_is_refused() {
    // Whatever is already there cannot be told from something we made, so
    // there would be no safe answer later about what may be removed. An empty
    // folder, or one we wrote the record for, are the only two cases.
    let f = Fixture::new("occupied");
    fs::create_dir_all(&f.destination).expect("destination");
    fs::write(f.destination.join("someone-elses.flac"), "theirs").expect("their file");

    let (report, recs, tmpl) = f.parts(T);
    let p = plan::plan(&report.markers, &recs, &tmpl);
    let why = apply::preflight(&p, &f.source, &f.destination, Link::Symlink, false)
        .expect_err("an unknown destination is refused");
    assert!(why.contains("already has things in it"), "got: {why}");

    // And the file is still there, since nothing ran.
    assert!(f.destination.join("someone-elses.flac").exists());
}

#[test]
fn a_destination_we_built_before_is_allowed_back_into() {
    let f = Fixture::new("returning");
    establish(&f, T, Link::Symlink);
    let (report, recs, tmpl) = f.parts(T);
    let p = plan::plan(&report.markers, &recs, &tmpl);
    apply::preflight(&p, &f.source, &f.destination, Link::Symlink, false)
        .expect("our own record is what lets us back in");
}

#[test]
fn what_the_plan_no_longer_names_is_removed() {
    let f = Fixture::new("orphan");
    establish(&f, T, Link::Symlink);
    let state = findopera::state::load(&f.destination).expect("state");

    // A template that names one of them something else leaves the other
    // orphaned, which is what removing a recording from the library looks
    // like from here.
    let (report, recs, tmpl) = f.parts(r"{{opera.title}}");
    let p = plan::plan(&report.markers, &recs, &tmpl);
    let gone = apply::prune(&state, &p, &f.destination, false);

    assert_eq!(
        gone.removed.len(),
        state.entries.len(),
        "every old path moved"
    );
    assert!(gone.failed.is_empty(), "{:?}", gone.failed);
}

#[test]
fn nothing_outside_the_record_is_ever_removed() {
    // The whole safety argument. A copied folder is indistinguishable from
    // one somebody made by hand, so this must not depend on being able to
    // tell them apart — only on what was written down.
    let f = Fixture::new("theirs");
    establish(&f, T, Link::Copy);
    let state = findopera::state::load(&f.destination).expect("state");

    let mine = f.destination.join("My Own Mixes");
    fs::create_dir_all(&mine).expect("their folder");
    fs::write(mine.join("track.flac"), "theirs").expect("their file");

    // A plan naming nothing: every recorded entry is an orphan.
    let (report, recs, tmpl) = f.parts(r"{{opera.title}} \[{{id}}\] elsewhere");
    let p = plan::plan(&report.markers, &recs, &tmpl);
    apply::prune(&state, &p, &f.destination, false);

    assert!(mine.join("track.flac").exists(), "their file was removed");
}

#[test]
fn an_entry_that_stopped_looking_like_ours_is_left_alone() {
    let f = Fixture::new("changed");
    establish(&f, T, Link::Copy);
    let state = findopera::state::load(&f.destination).expect("state");

    // Recorded as a copied folder, and now a link. Someone did that, so it is
    // not ours to remove any more.
    let entry = f.destination.join(&state.entries[0].path);
    fs::remove_dir_all(&entry).expect("clear it");
    #[cfg(unix)]
    std::os::unix::fs::symlink("/tmp", &entry).expect("their link");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir("C:\\", &entry).expect("their link");

    let (report, recs, tmpl) = f.parts(r"{{opera.title}} \[{{id}}\] elsewhere");
    let p = plan::plan(&report.markers, &recs, &tmpl);
    let gone = apply::prune(&state, &p, &f.destination, false);

    assert!(entry.symlink_metadata().is_ok(), "it was removed anyway");
    assert!(
        gone.changed.iter().any(|(p, _)| *p == entry),
        "it should be reported: {:?}",
        gone.changed
    );
}

#[test]
fn a_dry_run_removes_nothing() {
    let f = Fixture::new("dry-prune");
    establish(&f, T, Link::Symlink);
    let state = findopera::state::load(&f.destination).expect("state");
    let entry = f.destination.join(&state.entries[0].path);

    let (report, recs, tmpl) = f.parts(r"{{opera.title}} \[{{id}}\] elsewhere");
    let p = plan::plan(&report.markers, &recs, &tmpl);
    let gone = apply::prune(&state, &p, &f.destination, true);

    assert!(!gone.removed.is_empty(), "it should say what it would take");
    assert!(entry.symlink_metadata().is_ok(), "but not take it");
}
