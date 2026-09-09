//! Waiting out a server that is asking for less traffic.
//!
//! On a real socket, because the subject is the HTTP: a `Retry-After` header
//! coming back off the wire, and the request going out again.

mod support;

use std::sync::mpsc;
use support::server::{serve, Answer};

const REFUSED: &str =
    r#"{"errors":[{"message":"Too many requests.","extensions":{"code":"RATE_LIMITED"}}]}"#;

/// A server that refuses the first `refusals` requests, then relents.
///
/// The receiver is how many requests it actually saw, which is the only way
/// to tell a retry from a client that gave up quietly.
fn grudging(
    refusals: usize,
    retry_after: Option<&'static str>,
) -> (String, mpsc::Receiver<support::server::Asked>) {
    serve(8, move |nth, _| {
        if nth < refusals {
            let answer = Answer::status(429, REFUSED);
            match retry_after {
                Some(after) => answer.header("Retry-After", after),
                None => answer,
            }
        } else {
            Answer::ok(r#"{"data":{"ok":1}}"#)
        }
    })
}

/// How many requests reached the server.
fn seen(rx: &mpsc::Receiver<support::server::Asked>) -> usize {
    rx.try_iter().count()
}

#[test]
fn a_request_told_to_wait_is_sent_again() {
    let (endpoint, asked) = grudging(2, Some("1"));
    let api = findopera::api::Client::new(&endpoint, Some("t".into()));

    let payload = api
        .post("{ ok }", None)
        .expect("it should get there eventually");
    assert_eq!(payload["data"]["ok"], 1);
    assert_eq!(seen(&asked), 3, "two refusals, then the answer");
}

#[test]
fn giving_up_says_what_to_do_about_it() {
    // Refuses forever. The message has to be about traffic rather than a bare
    // status, because the answer is to stop asking rather than to look for a
    // bug.
    let (endpoint, asked) = grudging(usize::MAX, Some("1"));
    let api = findopera::api::Client::new(&endpoint, Some("t".into()));

    let why = api.post("{ ok }", None).expect_err("it never relents");
    let why = why.to_string();
    assert!(why.contains("refusing"), "got: {why}");
    assert!(why.contains("wait"), "got: {why}");
    assert_eq!(seen(&asked), 5, "one try and four retries");
}

#[test]
fn an_anonymous_caller_is_told_a_token_would_help() {
    // Anonymous callers share a much smaller budget, so for them the limit is
    // the likely cause rather than a coincidence — and there is something they
    // can actually do about it.
    let (endpoint, _asked) = grudging(usize::MAX, Some("1"));
    let api = findopera::api::Client::new(&endpoint, None);

    let why = api
        .post("{ ok }", None)
        .expect_err("it never relents")
        .to_string();
    assert!(why.contains("anonymous"), "got: {why}");
    assert!(why.contains("login --new"), "got: {why}");
}

#[test]
fn a_refusal_with_no_retry_after_still_eases_off() {
    // Nothing to go on, so the client picks its own delay rather than
    // hammering. Starting at one second keeps this test quick while still
    // exercising the path.
    let (endpoint, asked) = grudging(1, None);
    let api = findopera::api::Client::new(&endpoint, Some("t".into()));

    let payload = api.post("{ ok }", None).expect("it relents after one");
    assert_eq!(payload["data"]["ok"], 1);
    assert_eq!(seen(&asked), 2);
}
