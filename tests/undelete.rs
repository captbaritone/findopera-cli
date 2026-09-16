//! Every type that can be removed can be put back.
//!
//! What an undelete sends, and what it says, are cases under
//! `tests/snapshots/`. This is the part no command reports: the table codegen
//! derived, checked against the rule it was derived under.

use findopera::model::crud;

#[test]
fn undelete_is_offered_for_every_type_delete_is() {
    // The server generates the pair together -- a delete writes a version with
    // the flag set, an undelete writes one with it cleared -- so a type that
    // could be removed and not restored would be a half-updated schema rather
    // than a decision. Codegen throws on that; this pins it from the Rust side
    // too, where anyone reading the table can see it.
    for t in crud::TYPES {
        assert_eq!(
            t.restore,
            format!("undelete{}", t.graphql),
            "{} restores through the wrong mutation",
            t.name
        );
        assert_eq!(
            t.remove,
            format!("delete{}", t.graphql),
            "{} removes through the wrong mutation",
            t.name
        );
    }
}

#[test]
fn restoring_is_offered_more_widely_than_merging() {
    // Worth stating, because the two verbs undo different things. Merge is for
    // the seven types whose ids this catalog mints and which something can
    // point at; undelete is for anything that can be deleted at all, which is
    // every one of them.
    let restorable = crud::TYPES.len();
    let mergeable = crud::TYPES.iter().filter(|t| t.merge.is_some()).count();
    assert!(restorable > mergeable);
    assert_eq!(mergeable, 7);
}
