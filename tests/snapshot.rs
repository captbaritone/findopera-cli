//! Every case under `tests/snapshots/`.
//!
//! One file is one whole command — a library, canned answers, an argv — and
//! what it captures is everything that command did at once. See
//! `support/snapshot.rs` for why that is the point.
//!
//! ```text
//! UPDATE_EXPECT=1 cargo test    rewrite the expectations, then read the diff
//! ```

mod support;

#[test]
fn every_case_matches_what_it_records() {
    support::snapshot::run_all(std::path::Path::new("tests/snapshots"));
}
