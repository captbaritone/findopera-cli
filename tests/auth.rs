//! What findopera.com is actually told about the caller.
//!
//! These assert on the bytes on the wire rather than on the shape of the code,
//! because the property worth keeping is that *every* request carries identity
//! — including the query built into the binary, which no caller passes in and
//! which is therefore the easiest one to forget.

mod support;

use support::server::serve;

const TOKEN: &str = "a-token-and-nothing-like-a-real-one";

#[test]
fn the_built_in_query_carries_identity() {
    // `organize` never passes a query in — it uses the one generated into the
    // binary. That is the request most easily left anonymous, so it is the one
    // most worth pinning down.
    let (endpoint, asked) = serve(1, |_, asked| {
        if asked.is_get() {
            support::server::Answer::ok("type Query {\n  ok: Int\n}")
        } else {
            support::server::Answer::ok(r#"{"data":{"getRecordingByIds":[null]}}"#)
        }
    });
    let api = findopera::api::Client::new(&endpoint, Some(TOKEN.to_string()));
    let _ = api.recordings(&["10655".to_string()]);

    let request = asked.recv().expect("the server was asked something");
    assert_eq!(
        request.header("authorization"),
        Some(&*format!("Bearer {TOKEN}"))
    );
    assert!(
        request.body.contains("getRecordingByIds"),
        "this should be the generated query, got: {}",
        request.body
    );
}

#[test]
fn an_arbitrary_query_carries_identity() {
    let (endpoint, asked) = serve(1, |_, asked| {
        if asked.is_get() {
            support::server::Answer::ok("type Query {\n  ok: Int\n}")
        } else {
            support::server::Answer::ok(r#"{"data":{"getRecordingByIds":[null]}}"#)
        }
    });
    let api = findopera::api::Client::new(&endpoint, Some(TOKEN.to_string()));
    let _ = api.post("{ ok }", None);

    let request = asked.recv().expect("the server was asked something");
    assert_eq!(
        request.header("authorization"),
        Some(&*format!("Bearer {TOKEN}"))
    );
}

#[test]
fn fetching_the_schema_carries_identity() {
    // Reads are the bulk of what this program does, and a server that cannot
    // tell them from a stranger's has to treat them like a stranger's.
    let (endpoint, asked) = serve(1, |_, asked| {
        if asked.is_get() {
            support::server::Answer::ok("type Query {\n  ok: Int\n}")
        } else {
            support::server::Answer::ok(r#"{"data":{"getRecordingByIds":[null]}}"#)
        }
    });
    let api = findopera::api::Client::new(&endpoint, Some(TOKEN.to_string()));
    let _ = api.schema();

    let request = asked.recv().expect("the server was asked something");
    assert!(request.line.starts_with("GET"), "got: {}", request.line);
    assert_eq!(
        request.header("authorization"),
        Some(&*format!("Bearer {TOKEN}"))
    );
}

#[test]
fn every_request_says_which_version_it_is() {
    let (endpoint, asked) = serve(1, |_, asked| {
        if asked.is_get() {
            support::server::Answer::ok("type Query {\n  ok: Int\n}")
        } else {
            support::server::Answer::ok(r#"{"data":{"getRecordingByIds":[null]}}"#)
        }
    });
    let api = findopera::api::Client::new(&endpoint, None);
    let _ = api.post("{ ok }", None);

    let request = asked.recv().expect("the server was asked something");
    assert_eq!(
        request.header("user-agent"),
        Some(findopera::api::USER_AGENT)
    );
}

#[test]
fn without_a_token_nothing_is_claimed() {
    // Anonymous has to stay genuinely anonymous: an empty or malformed
    // Authorization header is worse than none, since a server may read it as a
    // failed attempt rather than as no attempt.
    let (endpoint, asked) = serve(1, |_, asked| {
        if asked.is_get() {
            support::server::Answer::ok("type Query {\n  ok: Int\n}")
        } else {
            support::server::Answer::ok(r#"{"data":{"getRecordingByIds":[null]}}"#)
        }
    });
    let api = findopera::api::Client::new(&endpoint, None);
    let _ = api.post("{ ok }", None);

    let request = asked.recv().expect("the server was asked something");
    assert_eq!(request.header("authorization"), None);
}
