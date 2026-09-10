//! The release check, against a stand-in for GitHub.
//!
//! Offline, like the rest: a listener on a loose port answers the releases
//! API, so what is exercised is the request that actually goes out and the
//! reading of what comes back — not a mock of this program's own idea of
//! either.

mod support;

use findopera::release::{self, How};
use std::path::Path;
use support::server::{serve, Answer};

/// A stand-in for the releases API, answering once.
fn releases(
    status: u16,
    body: &'static str,
) -> (String, std::sync::mpsc::Receiver<support::server::Asked>) {
    serve(1, move |_, _| Answer::status(status, body))
}

#[test]
fn a_newer_release_is_reported_as_newer() {
    let (url, _asked) = releases(200, r#"{"tag_name":"v99.0.0"}"#);
    let check = release::check(
        &findopera::api::Http,
        &url,
        Some(Path::new("/usr/local/bin/findopera")),
    )
    .expect("the release was read");

    assert!(check.newer_available());
    assert_eq!(check.latest.to_string(), "99.0.0");
    assert_eq!(check.current.to_string(), release::CURRENT);
}

#[test]
fn the_release_this_binary_already_is_is_not_newer() {
    // The version is stamped in at build time, so the case worth pinning is
    // this one: the server naming exactly what is already installed.
    let body: &'static str =
        Box::leak(format!(r#"{{"tag_name":"v{}"}}"#, release::CURRENT).into_boxed_str());
    let (url, _asked) = releases(200, body);
    let check = release::check(&findopera::api::Http, &url, None).expect("the release was read");

    assert!(
        !check.newer_available(),
        "{} should not be newer than itself",
        release::CURRENT
    );
}

#[test]
fn an_older_release_is_not_newer() {
    // A build from a checkout is ahead of what is published, and must not be
    // told to downgrade itself.
    let (url, _asked) = releases(200, r#"{"tag_name":"v0.0.1"}"#);
    let check = release::check(&findopera::api::Http, &url, None).expect("the release was read");
    assert!(!check.newer_available());
}

#[test]
fn the_request_says_who_is_asking_and_which_api_it_wants() {
    // GitHub refuses a request with no user-agent outright, and the Accept
    // header is what pins the response shape this parses.
    let (url, asked) = releases(200, r#"{"tag_name":"v99.0.0"}"#);
    let _ = release::check(&findopera::api::Http, &url, None);

    let request = asked.recv().expect("the server was asked something");
    assert_eq!(
        request.header("user-agent"),
        Some(findopera::api::USER_AGENT)
    );
    assert_eq!(
        request.header("accept"),
        Some("application/vnd.github+json")
    );
}

#[test]
fn being_rate_limited_says_so_rather_than_saying_forbidden() {
    // Anonymous callers meet this often enough that "403" on its own would
    // send someone looking for a permission they do not need.
    let (url, _asked) = releases(403, r#"{"message":"rate limit exceeded"}"#);
    let error =
        release::check(&findopera::api::Http, &url, None).expect_err("403 is not an answer");
    let said = error.to_string();
    assert!(said.contains("rate limit"), "got: {said}");
}

#[test]
fn a_tag_that_is_not_a_version_is_an_error() {
    let (url, _asked) = releases(200, r#"{"tag_name":"nightly"}"#);
    let error =
        release::check(&findopera::api::Http, &url, None).expect_err("`nightly` is not a version");
    assert!(
        matches!(error, release::Error::Unreadable(_)),
        "got: {error}"
    );
}

#[test]
fn where_the_binary_sits_decides_what_it_is_told_to_run() {
    let (url, _asked) = releases(200, r#"{"tag_name":"v99.0.0"}"#);
    let check = release::check(
        &findopera::api::Http,
        &url,
        Some(Path::new("/home/me/.cargo/bin/findopera")),
    )
    .expect("the release was read");

    assert_eq!(check.how, How::Cargo);
    assert!(
        check.how.command().starts_with("cargo install"),
        "got: {}",
        check.how.command()
    );
}
