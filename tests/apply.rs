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

// Everything else `apply` does is a case under `tests/snapshots/building/`
// and `tests/snapshots/linking/`, judged on what a person would see. These
// two are what is left: a clone behaves exactly like a copy in every way
// anybody can observe — which is the point of it — so the only witness that
// it cloned rather than copied is the inode, and the only place to look at
// one is here.
#[cfg(unix)]
mod clones {
    use super::*;

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
