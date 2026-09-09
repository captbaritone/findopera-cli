//! The generated model against real API responses.
//!
//! `tests/fixtures/recordings.json` is what findopera.com actually returned
//! for ids 10655, 75 and 1 — captured rather than invented, because the point
//! of these tests is the gap between what the schema promises and what the
//! wire carries. Between them the two real recordings spell "unknown" all
//! three ways: `month: 0`, `month: null`, and `librettist: ""`.
//!
//! What those spellings do to a folder name is a case under
//! `tests/snapshots/template/nullability/`. What is left here is the claim
//! that the captured bytes still deserialize at all — a guard on the schema
//! having moved under us, which no rendering can make.

use findopera::model::Recording;

fn recordings() -> Vec<Option<Recording>> {
    let raw = include_str!("fixtures/recordings.json");
    serde_json::from_str(raw).expect("the captured response deserializes")
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
