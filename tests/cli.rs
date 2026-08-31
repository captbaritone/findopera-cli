//! End-to-end tests over the built binary.
//!
//! Everything that can be checked without the network is; tests that need the
//! real findopera.com API are marked `#[ignore]` so `cargo test` stays offline
//! and deterministic. Run those with `cargo test -- --ignored`.

use std::path::PathBuf;
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

fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("run findopera")
}

/// An endpoint that refuses instantly, so tests never touch the network.
const OFFLINE: [&str; 4] = ["--endpoint", "http://127.0.0.1:9/graphql", "--timeout", "2"];

fn offline(args: &[&str]) -> Output {
    let mut all: Vec<&str> = args.to_vec();
    all.extend_from_slice(&OFFLINE);
    run(&all)
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

// --- template validation (no network) ------------------------------------

#[test]
fn rejects_an_unknown_template_field_before_making_a_request() {
    let out = offline(&["render", "10655", "-t", "{{composer.surname}}"]);
    assert_eq!(code(&out), 2, "stderr: {}", stderr(&out));
    let v: serde_json::Value = serde_json::from_str(&stderr(&out)).expect("stderr is JSON");
    assert_eq!(v["error"], "template_unknown_field");
    assert!(v["suggestion"]
        .as_str()
        .unwrap()
        .contains("findopera fields"));
}

#[test]
fn rejects_malformed_template_syntax() {
    for bad in ["{{opera.title", "{{}}", "{{opera.title|\"unclosed}}"] {
        let out = offline(&["render", "10655", "-t", bad]);
        assert_eq!(code(&out), 2, "template {bad:?} should be a usage error");
    }
}

/// A bad template must be caught by parsing, not by a failed lookup — the
/// offline endpoint would give exit 6 if the request were attempted first.
#[test]
fn template_errors_take_priority_over_network_errors() {
    let out = offline(&["render", "10655", "-t", "{{nope}}"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn non_numeric_recording_id_is_rejected_without_a_request() {
    let out = offline(&["render", "not-an-id", "-t", "{{opera.title}}"]);
    assert_eq!(code(&out), 2);
    let v: serde_json::Value = serde_json::from_str(&stderr(&out)).unwrap();
    assert_eq!(v["error"], "invalid_recording_id");
}

#[test]
fn unreachable_api_reports_a_retryable_error() {
    let out = offline(&["render", "10655", "-t", "{{opera.title}}"]);
    assert_eq!(code(&out), 6);
    let v: serde_json::Value = serde_json::from_str(&stderr(&out)).unwrap();
    assert_eq!(v["retryable"], true);
}

// --- exit codes and help --------------------------------------------------

#[test]
fn help_succeeds_but_a_missing_subcommand_is_a_usage_error() {
    assert_eq!(code(&run(&["--help"])), 0);
    assert_eq!(code(&run(&[])), 2);
    assert_eq!(code(&run(&["frobnicate"])), 2);
}

// --- discoverability ------------------------------------------------------

#[test]
fn fields_lists_every_template_field_with_a_description() {
    let out = run(&["fields", "--format", "json"]);
    assert_eq!(code(&out), 0);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("stdout is JSON");
    let fields = v["fields"].as_array().unwrap();
    assert!(fields.len() >= 20, "expected the full field surface");
    for f in fields {
        assert!(f["field"].is_string());
        assert!(
            !f["description"].as_str().unwrap_or("").is_empty(),
            "{} needs a description",
            f["field"]
        );
    }
    assert!(v["syntax"]["fallback"].is_string());
}

/// The set `fields` advertises must be exactly what a template accepts, or the
/// documentation lies.
#[test]
fn every_advertised_field_is_accepted_by_the_template_parser() {
    let v: serde_json::Value =
        serde_json::from_str(&stdout(&run(&["fields", "--format", "json"]))).unwrap();
    for field in v["fields"].as_array().unwrap() {
        let name = field["field"].as_str().unwrap();
        let tmpl = format!("{{{{{name}}}}}");
        let out = offline(&["render", "10655", "-t", &tmpl]);
        // Exit 6 means it got past parsing and tried the network, which is
        // what we want; exit 2 would mean the parser rejected a listed field.
        assert_ne!(
            code(&out),
            2,
            "`{name}` is listed by `fields` but rejected by --template: {}",
            stderr(&out)
        );
    }
}

#[test]
fn schema_dumps_the_command_tree_with_exit_codes_and_fields() {
    let out = run(&["schema", "--all"]);
    assert_eq!(code(&out), 0);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("schema is JSON");
    let names: Vec<&str> = v["subcommands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"render"));
    assert!(names.contains(&"fields"));
    assert_eq!(v["exitCodes"]["3"], "recording not found in the database");
    assert!(v["templateFields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["field"] == "composer.lastName"));
}

#[test]
fn schema_for_an_unknown_command_lists_the_real_ones() {
    let out = run(&["schema", "frobnicate"]);
    assert_eq!(code(&out), 2);
    let v: serde_json::Value = serde_json::from_str(&stderr(&out)).unwrap();
    assert_eq!(v["error"], "unknown_command");
    let details: Vec<&str> = v["details"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap())
        .collect();
    assert!(details.contains(&"render"));
}

#[test]
fn fields_example_rejects_a_non_numeric_id() {
    assert_eq!(code(&offline(&["fields", "--example", "nope"])), 2);
}

// --- live API ------------------------------------------------------------

#[test]
#[ignore = "hits the live findopera.com API"]
fn live_render_produces_the_expected_string() {
    let out = run(&[
        "render",
        "10655",
        "-t",
        "{{composer.lastName}}/{{opera.title}}/{{year}}",
        "--format",
        "json",
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(
        v["results"][0]["rendered"],
        "Handel/Sosarme, Re di Media/2026"
    );
    assert_eq!(v["results"][0]["segments"][0], "Handel");
}

#[test]
#[ignore = "hits the live findopera.com API"]
fn live_renders_several_recordings_in_the_order_given() {
    // --format text is explicit here: captured output is not a TTY, so the
    // default would (correctly) be JSON.
    let out = run(&[
        "render",
        "75",
        "10655",
        "-t",
        "{{composer.lastName}}",
        "--format",
        "text",
    ]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "Britten\nHandel\n");
}

/// Recording 10655 has no `opera.englishTitle`, so the bare field must fail
/// loudly and the fallback form must succeed.
#[test]
#[ignore = "hits the live findopera.com API"]
fn live_absent_field_errors_unless_a_fallback_is_given() {
    let out = run(&[
        "render",
        "10655",
        "-t",
        "{{opera.englishTitle}}",
        "--format",
        "text",
    ]);
    assert_eq!(code(&out), 1, "an unresolved placeholder is an error");
    assert!(stderr(&out).contains("resolved to nothing"));

    // In JSON mode the same failure is data on stdout, not prose on stderr.
    let out = run(&[
        "render",
        "10655",
        "-t",
        "{{opera.englishTitle}}",
        "--format",
        "json",
    ]);
    assert_eq!(code(&out), 1);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v["problems"][0]["error"], "template_unresolved_field");

    let out = run(&[
        "render",
        "10655",
        "-t",
        "{{opera.englishTitle|opera.title}}",
        "--format",
        "text",
    ]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "Sosarme, Re di Media\n");

    let out = run(&[
        "render",
        "10655",
        "-t",
        "{{opera.englishTitle|\"Untitled\"}}",
        "--format",
        "text",
    ]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "Untitled\n");
}

#[test]
#[ignore = "hits the live findopera.com API"]
fn live_unknown_recording_id_exits_three() {
    let out = run(&["render", "99999999", "-t", "{{opera.title}}"]);
    assert_eq!(code(&out), 3);
}

#[test]
#[ignore = "hits the live findopera.com API"]
fn live_fields_example_shows_which_fields_are_populated() {
    let out = run(&["fields", "--example", "10655", "--format", "json"]);
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
