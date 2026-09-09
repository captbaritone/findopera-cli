//! What a merge actually sends, and which types may be sent one.
//!
//! Merging is the one mutation that destroys a record without being called
//! `delete`, and the losing id is an argument rather than the subject of the
//! sentence. Getting `id` and `intoId` the wrong way round would merge the
//! survivor into the duplicate — a mistake nothing downstream could notice —
//! so the request itself is what these assert on.
//!
//! No socket: the seam under the client replaces only the transport, so these
//! still run the real document building, variables and refusal handling.

mod support;

use findopera::model::crud;
use support::scripted::Scripted;

fn kind(name: &str) -> &'static crud::Type {
    crud::TYPES
        .iter()
        .find(|t| t.name == name)
        .expect("a type by that name")
}

#[test]
fn the_loser_is_the_subject_and_the_survivor_is_into_id() {
    let script = Scripted::new().answers("Merge", 200, r#"{"data":{"mergeSinger":{"id":"456"}}}"#);
    let api = script.client("https://example.invalid/g", Some("a-token".into()));

    let survivor = api
        .merge(kind("singer"), "133", "456", "same person, two spellings")
        .expect("the merge is accepted");

    let sent = script.sent();
    let variables = sent[0].variables.as_ref().expect("variables");
    assert_eq!(variables["id"], "133", "the losing id");
    assert_eq!(variables["intoId"], "456", "the surviving id");
    assert_eq!(variables["justification"], "same person, two spellings");
    assert!(
        sent[0].query.as_ref().unwrap().contains("mergeSinger"),
        "got: {:?}",
        sent[0].query
    );

    // The survivor is what comes back, not the id that was passed in: an id
    // handed to another command has to be one that still resolves.
    assert_eq!(survivor, "456");
}

#[test]
fn each_type_is_merged_by_its_own_mutation() {
    // One mutation per type rather than a general one, so a table that drifted
    // would send `mergeSinger` for an opera and be refused in terms naming
    // neither.
    let script = Scripted::new().answers("Merge", 200, r#"{"data":{"mergeOpera":{"id":"34"}}}"#);
    let api = script.client("https://example.invalid/g", None);
    let _ = api.merge(kind("opera"), "12", "34", "https://...");

    assert!(
        script.sent()[0]
            .query
            .as_ref()
            .unwrap()
            .contains("mergeOpera"),
        "got: {:?}",
        script.sent()[0].query
    );
}

#[test]
fn a_type_the_server_cannot_merge_is_refused_without_asking() {
    // Nothing is scripted, so any request at all would fail the test by
    // itself: an unmergeable type must be settled before one is built, or the
    // reply is a GraphQL error about a field that does not exist rather than
    // an answer about the type in hand.
    let script = Scripted::new();
    let api = script.client("https://example.invalid/g", None);

    let error = api
        .merge(kind("upc"), "1", "2", "a reason")
        .expect_err("a upc cannot be merged");

    assert!(
        matches!(error, findopera::api::ApiError::Refused(_)),
        "a refusal rather than an unreachable server, got: {error}"
    );
    assert!(
        error.to_string().contains("cannot be merged"),
        "got: {error}"
    );
    assert!(script.sent().is_empty(), "nothing should have been sent");
}

#[test]
fn merge_is_offered_for_exactly_the_types_the_schema_merges() {
    // The list is derived from the schema by codegen, so this pins the shape
    // of what it derived: every merge names its own type, and the types with
    // one are the ones the server actually offers. A schema that grows another
    // makes this fail, which is the reminder to check the verb's help still
    // reads true.
    let mergeable: Vec<&str> = crud::TYPES
        .iter()
        .filter(|t| t.merge.is_some())
        .map(|t| t.name)
        .collect();
    assert_eq!(
        mergeable,
        [
            "character",
            "composer",
            "conductor",
            "language",
            "opera",
            "recording",
            "singer"
        ],
    );

    for t in crud::TYPES {
        if let Some(mutation) = t.merge {
            assert_eq!(
                mutation,
                format!("merge{}", t.graphql),
                "{} merges through the wrong mutation",
                t.name
            );
        }
    }
}
