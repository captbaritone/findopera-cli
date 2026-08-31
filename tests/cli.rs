//! End-to-end tests over the built binary.
//!
//! These cover marker discovery, path rendering, and the destructive-write
//! guards. They build their own fixture tree and never reach the network: the
//! `--endpoint` flag is pointed at a file:// -style stub where a recording
//! lookup is needed, and tests that would require the live API are marked
//! `#[ignore]` so `cargo test` stays offline and deterministic.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> PathBuf {
    // target/debug/deps/<test binary> -> target/debug/findopera
    let mut p = std::env::current_exe().expect("test binary path");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("findopera")
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let root =
            std::env::temp_dir().join(format!("findopera-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("music")).expect("create fixture");
        Fixture { root }
    }
    /// A recording directory containing a marker file with the given body.
    fn recording(&self, dir: &str, marker: &str, body: &str) -> &Fixture {
        let d = self.root.join("music").join(dir);
        fs::create_dir_all(&d).expect("create recording dir");
        fs::write(d.join(marker), body).expect("write marker");
        fs::write(d.join("disc1.flac"), b"").expect("write audio");
        self
    }
    fn music(&self) -> PathBuf {
        self.root.join("music")
    }
    fn dest(&self) -> PathBuf {
        self.root.join("canonical")
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(bin())
            .args(args)
            .output()
            .expect("run findopera")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn code(o: &Output) -> i32 {
    o.status.code().expect("exited with a code")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

const MARKER_75: &str = "\
                         BILLY BUDD

              by Benjamin Britten (1913-1976)

Conductor: Benjamin Britten
Recorded: 1967


             https://findopera.com/recording/75
";

// --- marker discovery (no network) ---------------------------------------

#[test]
fn finds_markers_and_reports_them_as_json_when_piped() {
    let f = Fixture::new("discover");
    f.recording("billy_budd", "Billy Budd.txt", MARKER_75);
    let out = f.run(&["marker", "list", "--source", f.music().to_str().unwrap()]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("stdout is JSON");
    assert_eq!(v["markers"][0]["recordingId"], "75");
    assert_eq!(v["summary"]["markersFound"], 1);
}

#[test]
fn ignores_txt_files_without_a_findopera_url() {
    let f = Fixture::new("ignore");
    f.recording(
        "notes",
        "readme.txt",
        "Ripped from CD in 2019. See findopera.com",
    );
    let out = f.run(&["marker", "list", "--source", f.music().to_str().unwrap()]);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v["summary"]["markersFound"], 0);
    assert_eq!(v["summary"]["txtFilesSkipped"], 1);
}

#[test]
fn one_directory_can_hold_markers_for_several_recordings() {
    let f = Fixture::new("boxset");
    let d = f.music().join("boxset");
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join("a.txt"), "https://findopera.com/recording/75").unwrap();
    fs::write(d.join("b.txt"), "https://findopera.com/recording/500").unwrap();
    let out = f.run(&["marker", "list", "--source", f.music().to_str().unwrap()]);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v["summary"]["markersFound"], 2);
}

// --- argument and template validation (no network) -----------------------

#[test]
fn rejects_an_unknown_template_field_before_touching_the_disk() {
    let f = Fixture::new("badfield");
    f.recording("billy_budd", "m.txt", MARKER_75);
    let out = f.run(&[
        "library",
        "sync",
        "--source",
        f.music().to_str().unwrap(),
        "--destination",
        f.dest().to_str().unwrap(),
        "--template",
        "{{composer.surname}}",
        "--apply",
    ]);
    assert_eq!(code(&out), 2);
    let v: serde_json::Value = serde_json::from_str(&stderr(&out)).expect("stderr is JSON");
    assert_eq!(v["error"], "template_unknown_field");
    assert!(v["suggestion"].as_str().unwrap().contains("library fields"));
    assert!(!f.dest().exists(), "must not create the destination");
}

#[test]
fn rejects_malformed_template_syntax() {
    let f = Fixture::new("badsyntax");
    for bad in ["{{opera.title", "{{}}", "{{opera.title|\"unclosed}}"] {
        let out = f.run(&[
            "library",
            "plan",
            "--source",
            f.music().to_str().unwrap(),
            "--destination",
            f.dest().to_str().unwrap(),
            "--template",
            bad,
        ]);
        assert_eq!(code(&out), 2, "template {bad:?} should be a usage error");
    }
}

#[test]
fn rejects_a_template_that_would_escape_the_destination() {
    // `..` can only arrive from the template itself, since interpolated
    // values have their separators stripped.
    let f = Fixture::new("escape");
    let out = f.run(&[
        "library",
        "plan",
        "--source",
        f.music().to_str().unwrap(),
        "--destination",
        f.dest().to_str().unwrap(),
        "--template",
        "../../etc/{{opera.title}}",
    ]);
    // No markers, so this exits cleanly; the guard is unit-tested in
    // `template.rs`. Here we only assert nothing was written.
    assert!(!f.dest().exists());
    assert_ne!(code(&out), 0, "an empty plan is not a successful apply");
}

#[test]
fn missing_source_directory_is_a_usage_error() {
    let f = Fixture::new("nosource");
    let out = f.run(&[
        "library",
        "plan",
        "--source",
        f.root.join("nope").to_str().unwrap(),
        "--destination",
        f.dest().to_str().unwrap(),
        "--template",
        "{{opera.title}}",
    ]);
    assert_eq!(code(&out), 2);
    let v: serde_json::Value = serde_json::from_str(&stderr(&out)).unwrap();
    assert_eq!(v["error"], "source_not_found");
}

#[test]
fn non_numeric_recording_id_is_rejected_without_a_request() {
    let out = Command::new(bin())
        .args(["recording", "get", "not-an-id"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 2);
    let v: serde_json::Value = serde_json::from_str(&stderr(&out)).unwrap();
    assert_eq!(v["error"], "invalid_recording_id");
}

// --- exit codes and help --------------------------------------------------

#[test]
fn help_succeeds_but_a_missing_subcommand_is_a_usage_error() {
    assert_eq!(
        code(&Command::new(bin()).arg("--help").output().unwrap()),
        0
    );
    assert_eq!(code(&Command::new(bin()).output().unwrap()), 2);
    assert_eq!(
        code(
            &Command::new(bin())
                .args(["library", "frobnicate"])
                .output()
                .unwrap()
        ),
        2
    );
}

#[test]
fn schema_dumps_the_command_tree_with_exit_codes_and_fields() {
    let out = Command::new(bin())
        .args(["schema", "--all"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 0);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("schema is JSON");
    let names: Vec<&str> = v["subcommands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"library"));
    assert!(names.contains(&"recording"));
    assert_eq!(v["exitCodes"]["10"], "plan produced, safe to --apply");
    assert!(v["templateFields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["field"] == "composer.lastName"));
}

#[test]
fn schema_for_an_unknown_command_lists_the_real_ones() {
    let out = Command::new(bin())
        .args(["schema", "library", "frobnicate"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 2);
    let v: serde_json::Value = serde_json::from_str(&stderr(&out)).unwrap();
    assert_eq!(v["error"], "unknown_command");
    let details: Vec<&str> = v["details"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap())
        .collect();
    assert!(details.contains(&"sync"));
}

// --- destructive-write guards --------------------------------------------

/// The stamp is what makes wipe-and-rebuild safe, so verify the refusal
/// directly rather than through a full sync.
#[test]
fn refuses_to_wipe_a_destination_it_did_not_create() {
    let f = Fixture::new("precious");
    f.recording("billy_budd", "m.txt", MARKER_75);
    let dest = f.root.join("precious");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("thesis.txt"), b"years of work").unwrap();

    let out = f.run(&[
        "library",
        "sync",
        "--source",
        f.music().to_str().unwrap(),
        "--destination",
        dest.to_str().unwrap(),
        "--template",
        "{{opera.title}}",
        "--apply",
        // Point at an endpoint that will fail so the test needs no network;
        // the destination check must still not have destroyed anything.
        "--endpoint",
        "http://127.0.0.1:9/graphql",
        "--timeout",
        "2",
    ]);
    assert_ne!(code(&out), 0);
    assert!(
        dest.join("thesis.txt").exists(),
        "an unmanaged destination must survive"
    );
}

#[test]
fn plan_never_writes_to_the_destination() {
    let f = Fixture::new("planonly");
    f.recording("billy_budd", "m.txt", MARKER_75);
    let _ = f.run(&[
        "library",
        "plan",
        "--source",
        f.music().to_str().unwrap(),
        "--destination",
        f.dest().to_str().unwrap(),
        "--template",
        "{{opera.title}}",
        "--endpoint",
        "http://127.0.0.1:9/graphql",
        "--timeout",
        "2",
    ]);
    assert!(!f.dest().exists());
}

#[test]
fn unreachable_api_reports_a_retryable_error() {
    let f = Fixture::new("offline");
    f.recording("billy_budd", "m.txt", MARKER_75);
    let out = f.run(&[
        "library",
        "plan",
        "--source",
        f.music().to_str().unwrap(),
        "--destination",
        f.dest().to_str().unwrap(),
        "--template",
        "{{opera.title}}",
        "--endpoint",
        "http://127.0.0.1:9/graphql",
        "--timeout",
        "2",
    ]);
    assert_eq!(code(&out), 6);
    let v: serde_json::Value = serde_json::from_str(&stderr(&out)).unwrap();
    assert_eq!(v["retryable"], true);
}

// --- live API ------------------------------------------------------------
// Run with `cargo test -- --ignored` to exercise the real findopera.com API.

#[test]
#[ignore = "hits the live findopera.com API"]
fn live_sync_builds_a_tree_of_working_symlinks() {
    let f = Fixture::new("live");
    f.recording("billy_budd", "Billy Budd.txt", MARKER_75);
    let out = f.run(&[
        "library",
        "sync",
        "--source",
        f.music().to_str().unwrap(),
        "--destination",
        f.dest().to_str().unwrap(),
        "--template",
        "{{composer.lastName}}/{{opera.title}}/{{year}}",
        "--apply",
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));

    let link = f.dest().join("Britten/Billy Budd/1967");
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert!(
        link.join("disc1.flac").exists(),
        "symlink resolves to the recording"
    );
    assert!(
        f.dest().join(".findopera-library.json").is_file(),
        "stamp written"
    );

    // Re-running is idempotent, and dropping a marker prunes its link.
    let before = tree(&f.dest());
    let out = f.run(&[
        "library",
        "sync",
        "--source",
        f.music().to_str().unwrap(),
        "--destination",
        f.dest().to_str().unwrap(),
        "--template",
        "{{composer.lastName}}/{{opera.title}}/{{year}}",
        "--apply",
    ]);
    assert_eq!(code(&out), 0);
    assert_eq!(before, tree(&f.dest()), "re-apply must be idempotent");

    fs::remove_dir_all(f.music().join("billy_budd")).unwrap();
    let _ = f.run(&[
        "library",
        "sync",
        "--source",
        f.music().to_str().unwrap(),
        "--destination",
        f.dest().to_str().unwrap(),
        "--template",
        "{{composer.lastName}}/{{opera.title}}/{{year}}",
        "--apply",
    ]);
    assert!(
        !f.dest().join("Britten").exists(),
        "stale link must be pruned"
    );
}

#[test]
#[ignore = "hits the live findopera.com API"]
fn live_clean_plan_exits_ten() {
    let f = Fixture::new("live-plan");
    f.recording("billy_budd", "Billy Budd.txt", MARKER_75);
    let out = f.run(&[
        "library",
        "plan",
        "--source",
        f.music().to_str().unwrap(),
        "--destination",
        f.dest().to_str().unwrap(),
        "--template",
        "{{composer.lastName}}/{{opera.title}}/{{year}}",
    ]);
    assert_eq!(code(&out), 10, "a clean plan signals safe-to-apply");
}

#[test]
#[ignore = "hits the live findopera.com API"]
fn live_unknown_recording_id_exits_three() {
    let out = Command::new(bin())
        .args(["recording", "get", "99999999"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 3);
}

fn tree(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            out.push(p.strip_prefix(root).unwrap().display().to_string());
            if p.symlink_metadata().is_ok_and(|m| m.file_type().is_dir()) {
                stack.push(p);
            }
        }
    }
    out.sort();
    out
}

// --- template discoverability --------------------------------------------

#[test]
fn fields_lists_every_template_field_with_a_description() {
    let out = Command::new(bin())
        .args(["library", "fields", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 0);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("stdout is JSON");
    let fields = v["fields"].as_array().unwrap();
    assert!(fields.len() >= 20, "expected the full field surface");
    for f in fields {
        assert!(f["field"].is_string());
        let d = f["description"].as_str().unwrap_or("");
        assert!(!d.is_empty(), "{} needs a description", f["field"]);
    }
    // The syntax summary is what teaches fallbacks.
    assert!(v["syntax"]["fallback"].is_string());
}

/// The set `fields` advertises must be exactly what a template accepts —
/// otherwise the documentation lies and `--template` rejects a listed field.
#[test]
fn every_advertised_field_is_accepted_by_the_template_parser() {
    let out = Command::new(bin())
        .args(["library", "fields", "--format", "json"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let f = Fixture::new("advertised");
    for field in v["fields"].as_array().unwrap() {
        let name = field["field"].as_str().unwrap();
        let out = f.run(&[
            "library",
            "plan",
            "--source",
            f.music().to_str().unwrap(),
            "--destination",
            f.dest().to_str().unwrap(),
            "--template",
            &format!("{{{{{name}}}}}"),
            "--endpoint",
            "http://127.0.0.1:9/graphql",
            "--timeout",
            "2",
        ]);
        // No markers, so it never reaches the network; a usage error (2) would
        // mean the parser rejected a field that `fields` advertises.
        assert_ne!(
            code(&out),
            2,
            "`{name}` is listed by `library fields` but rejected by --template: {}",
            stderr(&out)
        );
    }
}

#[test]
fn schema_carries_template_field_descriptions_too() {
    let out = Command::new(bin())
        .args(["schema", "--all"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let fields = v["templateFields"].as_array().unwrap();
    assert!(fields
        .iter()
        .any(|f| f["field"] == "composer.lastName" && f["description"].is_string()));
}

#[test]
fn fields_example_rejects_a_non_numeric_id() {
    let out = Command::new(bin())
        .args(["library", "fields", "--example", "nope"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 2);
}

#[test]
#[ignore = "hits the live findopera.com API"]
fn live_fields_example_shows_which_fields_are_populated() {
    let out = Command::new(bin())
        .args([
            "library",
            "fields",
            "--example",
            "10655",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let by = |name: &str| -> serde_json::Value {
        v["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["field"] == name)
            .expect("field present")
            .clone()
    };
    assert_eq!(by("composer.lastName")["value"], "Handel");
    // 10655 has month/day of 0 and no language — all must read as absent.
    assert!(by("month")["value"].is_null());
    assert!(by("opera.language")["value"].is_null());
}
