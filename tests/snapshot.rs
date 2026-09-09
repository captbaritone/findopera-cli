//! Every case under `tests/snapshots/`.
//!
//! A case is a pair. `some-case.md` says what to run — a library, canned
//! answers, an argv — and is only ever written by hand.
//! `some-case.expected.md` is what that produced, and is only ever generated.
//!
//! Kept apart so that a diff says which happened: a change under
//! `.expected.md` alone is the program behaving differently, and a change to
//! the case is somebody asking a different question. It also means every
//! expectation can be deleted and rebuilt, which is the only way to know none
//! of them is stale.
//!
//! ```text
//! UPDATE_EXPECT=1 cargo test              rewrite expectations, then read the diff
//! rm tests/snapshots/**/*.expected.md     and rebuild them all from nothing
//! ```
//!
//! See `support/snapshot.rs` for what a case captures, and why all of it.

mod support;

#[test]
fn every_case_matches_what_it_records() {
    support::snapshot::run_all(std::path::Path::new("tests/snapshots"));
}
