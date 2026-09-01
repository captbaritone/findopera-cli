//! The generated model against real API responses.
//!
//! `tests/fixtures/recordings.json` is what findopera.com actually returned
//! for ids 10655, 75 and 1 — captured rather than invented, because the point
//! of these tests is the gap between what the schema promises and what the
//! wire carries. Between them the two real recordings spell "unknown" all
//! three ways: `month: 0`, `month: null`, and `librettist: ""`.

use findopera::model::{Recording, FIELDS};
use findopera::{to_path, Template};

fn recordings() -> Vec<Option<Recording>> {
    let raw = include_str!("fixtures/recordings.json");
    serde_json::from_str(raw).expect("the captured response deserializes")
}

fn render(rec: &Recording, template: &str) -> String {
    let t = Template::parse(template, FIELDS).expect("template parses");
    t.render(rec)
}

#[test]
fn the_captured_response_deserializes() {
    let recs = recordings();
    assert_eq!(recs.len(), 3);
    // The API returns a positionally-aligned list with null for ids it does
    // not know, which is why the outer type is Option.
    assert!(recs[2].is_none(), "id 1 is not in the database");
}

#[test]
fn every_sentinel_for_unknown_collapses_to_none() {
    let recs = recordings();
    let sosarme = recs[0].as_ref().unwrap();
    let billy = recs[1].as_ref().unwrap();

    // Zero and null, for the same field on different records.
    assert_eq!(sosarme.month, None, "month: 0 means unknown");
    assert_eq!(billy.month, None, "month: null means unknown");
    // Empty string.
    assert_eq!(sosarme.opera.librettist, None, "librettist: \"\"");
    // And a value that is genuinely there survives all of it.
    assert_eq!(sosarme.year, Some(2026));
}

#[test]
fn a_non_null_field_needs_no_fallback() {
    // The schema marks Recording.opera and Opera.title @semanticNonNull, so
    // the generated FIELDS calls the path non_null and this parses bare.
    let recs = recordings();
    let rec = recs[0].as_ref().unwrap();
    assert_eq!(
        render(rec, "{{composer.lastName}}/{{opera.title}}"),
        "Handel/Sosarme, Re di Media"
    );
}

#[test]
fn a_nullable_field_is_rejected_without_one() {
    // `year` carries no @semanticNonNull, so parsing refuses it bare — before
    // any record is fetched, and regardless of whether this one has a year.
    let e = Template::parse("{{composer.lastName}}/{{year}}", FIELDS)
        .expect_err("a nullable field needs a fallback or a group");
    assert_eq!(e.code, "template_unresolvable");
}

#[test]
fn a_group_drops_around_an_absent_value() {
    let recs = recordings();
    let sosarme = recs[0].as_ref().unwrap();
    let billy = recs[1].as_ref().unwrap();
    let t = "{{composer.lastName}}/{{opera.title}}[ ({{year}})][ - {{chorus}}]";
    assert_eq!(
        render(sosarme, t),
        "Handel/Sosarme, Re di Media (2026)",
        "no chorus, so that group goes"
    );
    assert_eq!(
        render(billy, t),
        "Britten/Billy Budd (1967) - Ambrosian Opera Chorus"
    );
}

#[test]
fn a_path_through_an_absent_object_resolves_to_nothing() {
    // Language is nullable, so `opera.language` is nullable even though
    // Language.name is @semanticNonNull. One recording has it, one does not.
    let recs = recordings();
    // The brackets around the language are escaped: a group whose only
    // placeholder sits in a nested group would be rejected, since dropping the
    // inner one cannot drop the outer.
    let t = r"{{opera.title}}[ \[{{opera.language}}\]]";
    assert_eq!(render(recs[0].as_ref().unwrap(), t), "Sosarme, Re di Media");
    assert_eq!(render(recs[1].as_ref().unwrap(), t), "Billy Budd [English]");
}

#[test]
fn derived_fields_project_out_of_a_list() {
    let recs = recordings();
    let rec = recs[0].as_ref().unwrap();
    assert_eq!(
        render(rec, "{{opera.title}}[ - {{singers}}]"),
        "Sosarme, Re di Media - Rémy Brès-Feuillet, Sarah Charles, Éléonore Pancrazi"
    );
    assert_eq!(
        render(
            rec,
            "{{opera.title}}[ - {{singer1.lastName}}][, {{singer2.lastName}}]"
        ),
        "Sosarme, Re di Media - Brès-Feuillet, Charles"
    );
}

#[test]
fn rendering_a_real_recording_gives_a_usable_path() {
    let recs = recordings();
    let rec = recs[0].as_ref().unwrap();
    let rendered = render(rec, "{{composer.lastName}}/{{opera.title}}[/{{year}}]");
    assert_eq!(
        to_path(&rendered).expect("a usable relative path"),
        vec!["Handel", "Sosarme, Re di Media", "2026"]
    );
}
