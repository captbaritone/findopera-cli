//! Finding marker files on disk.
//!
//! Offline, and nothing here writes file contents: a marker is identified by
//! its name, so the files are all empty.

use findopera::scan;
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
        scan::scan(std::slice::from_ref(&self.0), false)
            .markers
            .iter()
            .map(|m| {
                let dir = m.directory.strip_prefix(&self.0).unwrap_or(&m.directory);
                (dir.display().to_string(), m.id.clone())
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
    t.touch("Britten/Billy Budd/75.txt");
    assert_eq!(t.scan(), vec![("Britten/Billy Budd".into(), "75".into())]);
}

#[test]
fn the_walk_goes_arbitrarily_deep() {
    let t = Tree::new("deep");
    t.touch("a/b/c/d/e/10655.txt");
    assert_eq!(t.scan(), vec![("a/b/c/d/e".into(), "10655".into())]);
}

#[test]
fn a_box_set_directory_stands_for_every_recording_in_it() {
    let t = Tree::new("boxset");
    t.touch("Box/1721.txt");
    t.touch("Box/5000.txt");
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
    t.touch("a/075.txt");
    assert_eq!(t.scan(), vec![("a".into(), "75".into())]);
}

#[test]
fn one_directory_naming_a_recording_twice_is_reported_once() {
    let t = Tree::new("dupe");
    t.touch("a/75.txt");
    t.touch("a/Billy Budd [findopera-75].txt");
    assert_eq!(t.scan(), vec![("a".into(), "75".into())]);
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
    t.touch("a/10655.jpg");
    assert!(t.scan().is_empty());
}

#[test]
fn a_marker_may_be_empty() {
    // `touch 10655.txt` is a perfectly good marker; every file in these tests
    // is empty, and this one says so on purpose.
    let t = Tree::new("empty");
    t.touch("a/10655.txt");
    assert_eq!(t.scan().len(), 1);
}

// ---- variants -------------------------------------------------------------

/// Every (directory, id, variant) found.
fn scan_variants(t: &Tree) -> Vec<(String, String, Option<String>)> {
    scan::scan(std::slice::from_ref(&t.0), false)
        .markers
        .iter()
        .map(|m| {
            let dir = m.directory.strip_prefix(&t.0).unwrap_or(&m.directory);
            (dir.display().to_string(), m.id.clone(), m.variant.clone())
        })
        .collect()
}

#[test]
fn whatever_follows_the_id_is_a_variant() {
    let t = Tree::new("variant");
    t.touch("a/332 flac.txt");
    t.touch("b/332 mp3.txt");
    let found = scan_variants(&t);
    assert!(found.contains(&("a".into(), "332".into(), Some("flac".into()))));
    assert!(found.contains(&("b".into(), "332".into(), Some("mp3".into()))));
}

#[test]
fn a_bare_id_carries_no_variant() {
    let t = Tree::new("novariant");
    t.touch("a/332.txt");
    assert_eq!(scan_variants(&t), vec![("a".into(), "332".into(), None)]);
}

#[test]
fn the_delimiters_around_the_id_are_not_part_of_the_variant() {
    // However the id is spelled, the variant is the word after it.
    let t = Tree::new("vdelims");
    t.touch("a/332-flac.txt");
    t.touch("b/Don Giovanni [findopera-332] SACD.txt");
    t.touch("c/332 (LP).txt");
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
    t.touch("a/332 flac.txt");
    t.touch("a/332 mp3.txt");
    assert_eq!(scan_variants(&t).len(), 2);
}

#[test]
fn the_same_marker_twice_is_still_reported_once() {
    let t = Tree::new("samevariant");
    t.touch("a/332 flac.txt");
    t.touch("a/Don Giovanni [findopera-332] flac.txt");
    assert_eq!(scan_variants(&t).len(), 1);
}
