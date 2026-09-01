//! Finding marker files on disk.
//!
//! Offline, and nothing here writes file contents: a marker is identified by
//! its name, so the files are all empty.

use findopera::scan;

mod fixture;
use std::fs;
use std::path::PathBuf;

/// A throwaway directory tree, removed when the test ends.
struct Tree(PathBuf);

impl Tree {
    fn new(name: &str) -> Tree {
        let dir = std::env::temp_dir().join(format!("findopera-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        Tree(dir)
    }
    /// Create an empty file — the name is the whole convention.
    fn touch(&self, path: &str) -> &Tree {
        let full = self.0.join(path);
        fs::create_dir_all(full.parent().unwrap()).expect("parent dirs");
        fs::write(full, "").expect("write");
        self
    }
    /// Every (directory, id) pair found, with paths relative to the tree root.
    fn scan(&self) -> Vec<(String, String)> {
        scan::scan(&self.0, false, &scan::Ignore::default())
            .markers
            .iter()
            .map(|m| {
                let dir = m.directory.strip_prefix(&self.0).unwrap_or(&m.directory);
                (fixture::slashes(&dir.display().to_string()), m.id.clone())
            })
            .collect()
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_marker_names_the_directory_that_holds_it() {
    let t = Tree::new("holds");
    t.touch("Britten/Billy Budd/findopera-75.txt");
    assert_eq!(t.scan(), vec![("Britten/Billy Budd".into(), "75".into())]);
}

#[test]
fn the_walk_goes_arbitrarily_deep() {
    let t = Tree::new("deep");
    t.touch("a/b/c/d/e/findopera-10655.txt");
    assert_eq!(t.scan(), vec![("a/b/c/d/e".into(), "10655".into())]);
}

#[test]
fn a_box_set_directory_stands_for_every_recording_in_it() {
    let t = Tree::new("boxset");
    t.touch("Box/findopera-1721.txt");
    t.touch("Box/findopera-5000.txt");
    let found = t.scan();
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|(d, _)| d == "Box"));
}

#[test]
fn the_sites_suggested_filename_carries_the_id() {
    let t = Tree::new("token");
    t.touch("a/Sosarme, Re di Media-2026 [findopera-10655].txt");
    assert_eq!(t.scan(), vec![("a".into(), "10655".into())]);
}

#[test]
fn the_token_is_matched_not_the_brackets_around_it() {
    // A title may carry brackets of its own, and the delimiter may change
    // later; neither should matter.
    let t = Tree::new("delims");
    t.touch("a/Salome [Live] (findopera-75).txt");
    t.touch("b/Aida.findopera-1721.txt");
    t.touch("c/[Remastered] findopera-5000 edition.txt");
    let found = t.scan();
    assert_eq!(found.len(), 3);
    assert!(found.contains(&("a".into(), "75".into())));
    assert!(found.contains(&("b".into(), "1721".into())));
    assert!(found.contains(&("c".into(), "5000".into())));
}

#[test]
fn a_name_that_merely_ends_in_the_token_is_not_a_marker() {
    let t = Tree::new("nametoken");
    t.touch("a/notfindopera-75.txt");
    assert!(t.scan().is_empty());
}

#[test]
fn leading_zeros_do_not_make_a_second_recording() {
    let t = Tree::new("zeros");
    t.touch("a/findopera-075.txt");
    assert_eq!(t.scan(), vec![("a".into(), "75".into())]);
}

#[test]
fn one_directory_naming_a_recording_twice_is_reported_once() {
    let t = Tree::new("dupe");
    t.touch("a/findopera-75.txt");
    t.touch("a/Billy Budd [findopera-75].txt");
    assert_eq!(t.scan(), vec![("a".into(), "75".into())]);
}

#[test]
fn a_bare_number_is_not_a_marker() {
    // A number and a `.txt` is what a track listing, a year or a disc number
    // looks like, and a library is full of them. The token is the part that
    // says the number means a recording rather than anything else.
    let t = Tree::new("bare");
    t.touch("a/10655.txt");
    t.touch("a/1967.txt");
    t.touch("a/01.txt");
    assert!(t.scan().is_empty());
}

#[test]
fn a_text_file_whose_name_carries_no_id_is_not_a_marker() {
    // Including one that talks about a recording: the name is the convention,
    // so nothing here is opened to find out.
    let t = Tree::new("skip");
    t.touch("a/liner notes.txt");
    t.touch("a/Sosarme [findopera].txt");
    assert!(t.scan().is_empty());
}

#[test]
fn only_txt_files_count() {
    let t = Tree::new("ext");
    t.touch("a/findopera-10655.jpg");
    assert!(t.scan().is_empty());
}

#[test]
fn a_marker_may_be_empty() {
    // `touch 10655.txt` is a perfectly good marker; every file in these tests
    // is empty, and this one says so on purpose.
    let t = Tree::new("empty");
    t.touch("a/findopera-10655.txt");
    assert_eq!(t.scan().len(), 1);
}

// ---- variants -------------------------------------------------------------

/// Every (directory, id, variant) found.
fn scan_variants(t: &Tree) -> Vec<(String, String, Option<String>)> {
    scan::scan(&t.0, false, &scan::Ignore::default())
        .markers
        .iter()
        .map(|m| {
            let dir = m.directory.strip_prefix(&t.0).unwrap_or(&m.directory);
            (
                fixture::slashes(&dir.display().to_string()),
                m.id.clone(),
                m.variant.clone(),
            )
        })
        .collect()
}

#[test]
fn whatever_follows_the_id_is_a_variant() {
    let t = Tree::new("variant");
    t.touch("a/findopera-332 flac.txt");
    t.touch("b/findopera-332 mp3.txt");
    let found = scan_variants(&t);
    assert!(found.contains(&("a".into(), "332".into(), Some("flac".into()))));
    assert!(found.contains(&("b".into(), "332".into(), Some("mp3".into()))));
}

#[test]
fn a_bare_id_carries_no_variant() {
    let t = Tree::new("novariant");
    t.touch("a/findopera-332.txt");
    assert_eq!(scan_variants(&t), vec![("a".into(), "332".into(), None)]);
}

#[test]
fn the_delimiters_around_the_id_are_not_part_of_the_variant() {
    // However the id is spelled, the variant is the word after it.
    let t = Tree::new("vdelims");
    t.touch("a/findopera-332-flac.txt");
    t.touch("b/Don Giovanni [findopera-332] SACD.txt");
    t.touch("c/findopera-332 (LP).txt");
    let found = scan_variants(&t);
    assert!(found.contains(&("a".into(), "332".into(), Some("flac".into()))));
    assert!(found.contains(&("b".into(), "332".into(), Some("SACD".into()))));
    assert!(found.contains(&("c".into(), "332".into(), Some("LP".into()))));
}

#[test]
fn two_variants_of_one_recording_in_one_directory_are_both_kept() {
    // The dedup is on (directory, id, variant): saying they are different
    // rips is exactly what the variant is for.
    let t = Tree::new("twovariants");
    t.touch("a/findopera-332 flac.txt");
    t.touch("a/findopera-332 mp3.txt");
    assert_eq!(scan_variants(&t).len(), 2);
}

#[test]
fn the_same_marker_twice_is_still_reported_once() {
    let t = Tree::new("samevariant");
    t.touch("a/findopera-332 flac.txt");
    t.touch("a/Don Giovanni [findopera-332] flac.txt");
    assert_eq!(scan_variants(&t).len(), 1);
}

// ---- folders left alone ---------------------------------------------------

fn scan_ignoring(t: &Tree, patterns: &[&str]) -> Vec<(String, String)> {
    let owned: Vec<String> = patterns.iter().map(|s| (*s).to_string()).collect();
    let ignore = scan::Ignore::new(&owned).expect("patterns compile");
    scan::scan(&t.0, false, &ignore)
        .markers
        .iter()
        .map(|m| {
            let dir = m.directory.strip_prefix(&t.0).unwrap_or(&m.directory);
            (fixture::slashes(&dir.display().to_string()), m.id.clone())
        })
        .collect()
}

#[test]
fn a_named_folder_is_skipped_wherever_it_turns_up() {
    // What a Synology leaves beside its media, at any depth.
    let t = Tree::new("eadir");
    t.touch("a/findopera-75.txt");
    t.touch("a/@eaDir/findopera-10655.txt");
    t.touch("@eaDir/findopera-1721.txt");
    assert_eq!(
        scan_ignoring(&t, &["@eaDir"]),
        vec![("a".into(), "75".into())]
    );
}

#[test]
fn a_path_pattern_skips_only_that_one() {
    let t = Tree::new("anchored");
    t.touch("Incomplete/findopera-75.txt");
    t.touch("Boxes/Incomplete/findopera-10655.txt");
    let found = scan_ignoring(&t, &["Incomplete/**"]);
    assert_eq!(
        found,
        vec![("Boxes/Incomplete".into(), "10655".into())],
        "the one at the top is skipped, the one further down is not"
    );
}

#[test]
fn a_skipped_folder_is_never_looked_inside() {
    // The point is pruning, not filtering: a folder that is skipped must cost
    // nothing, because on a network mount looking inside it is a round trip.
    // A directory with no permission to enter would raise an error if it were
    // descended into, and none if it is left alone.
    let t = Tree::new("pruned");
    t.touch("keep/findopera-75.txt");
    t.touch("skip/inner/findopera-10655.txt");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(t.0.join("skip"), fs::Permissions::from_mode(0o000))
            .expect("make it unreadable");
    }
    let ignore = scan::Ignore::new(&["skip".to_string()]).expect("compiles");
    let report = scan::scan(&t.0, false, &ignore);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(t.0.join("skip"), fs::Permissions::from_mode(0o755)).expect("restore");
    }

    assert_eq!(report.markers.len(), 1, "only the one that was kept");
    assert!(
        report.unreadable.is_empty(),
        "and nothing tried to read inside: {:?}",
        report.unreadable
    );
}

#[test]
fn no_patterns_leaves_everything_alone() {
    let t = Tree::new("nopatterns");
    t.touch("a/@eaDir/findopera-75.txt");
    assert_eq!(scan_ignoring(&t, &[]).len(), 1);
}
