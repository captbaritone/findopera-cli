//! Which types the schema says may be merged.
//!
//! What a merge sends, and what it says, are cases under
//! `tests/snapshots/merge/`. This is the one part no command reports: the
//! table codegen derived, checked against the schema it was derived from.

use findopera::model::crud;

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
