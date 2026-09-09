//! What the client tells the server about itself.
//!
//! Offline: a listener on a loose port stands in for findopera.com, so this
//! tests the bytes that actually go out rather than the constant they are
//! built from.

mod support;

use support::server::{serve, Answer};

/// Send one payload to a client and report what it made of it.
fn ask(payload: &'static str) -> Result<(), String> {
    let (endpoint, _asked) = serve(1, move |_, _| Answer::ok(payload));
    findopera::api::Client::new(&endpoint, None)
        .recordings(&["75".to_string()])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn every_request_says_which_program_and_version_it_is() {
    let (endpoint, asked) = serve(1, |_, _| {
        Answer::ok(r#"{"data":{"getRecordingByIds":[null]}}"#)
    });
    let _ = findopera::api::Client::new(&endpoint, None).recordings(&["75".to_string()]);

    let request = asked.recv().expect("the server was asked something");
    let agent = request
        .header("user-agent")
        .unwrap_or_else(|| panic!("no User-Agent among {:?}", request.headers));

    assert_eq!(agent, findopera::api::USER_AGENT);
    assert_eq!(
        agent,
        format!("findopera-cli/{}", env!("CARGO_PKG_VERSION")),
        "the version has to come from the package, not a copy of it"
    );
}

#[test]
fn a_top_level_error_is_fatal_and_reaches_the_reader_intact() {
    // The channel the server uses to say a client is too old. The words are
    // the whole point, so they must not be summarised away.
    let why = ask(
        r#"{"errors":[{"message":"findopera-cli 0.1.0 is no longer supported; upgrade to 0.3 or later","extensions":{"code":"CLIENT_TOO_OLD"}}]}"#,
    )
    .expect_err("an error in the response is fatal");
    assert!(
        why.contains("no longer supported; upgrade to 0.3 or later"),
        "got: {why}"
    );
    assert!(
        why.contains("CLIENT_TOO_OLD"),
        "the code is worth showing: {why}"
    );
}

#[test]
fn several_errors_each_get_their_own_line() {
    let why =
        ask(r#"{"errors":[{"message":"first"},{"message":"second"}]}"#).expect_err("still fatal");
    assert!(why.contains("\n    first"), "got: {why}");
    assert!(why.contains("\n    second"), "got: {why}");
}

#[test]
fn an_error_is_fatal_even_when_data_came_with_it() {
    // Partial data cannot be trusted: a null in a @semanticNonNull position is
    // explained by exactly one of these errors.
    let why = ask(r#"{"data":{"getRecordingByIds":[null]},"errors":[{"message":"partial"}]}"#)
        .expect_err("data alongside an error is not enough");
    assert!(why.contains("partial"), "got: {why}");
}

#[test]
fn an_error_with_no_message_still_says_something() {
    let why = ask(r#"{"errors":[{"extensions":{"code":"WEIRD"}}]}"#).expect_err("fatal");
    assert!(why.contains("WEIRD"), "got: {why}");
    assert!(why.contains("no message given"), "got: {why}");
}

#[test]
fn an_empty_errors_list_is_not_an_error() {
    // The spec says the field is absent unless non-empty, but a server that
    // sends `"errors": []` should not be read as refusing.
    ask(r#"{"errors":[],"data":{"getRecordingByIds":[null]}}"#).expect("nothing was reported");
}

#[test]
fn the_schema_lives_beside_the_api_on_the_same_server() {
    // Derived from the endpoint so that pointing at a development server moves
    // both, rather than leaving a second setting behind on production.
    assert_eq!(
        findopera::api::schema_url("https://findopera.com/api/graphql"),
        "https://findopera.com/schema.graphql"
    );
    assert_eq!(
        findopera::api::schema_url("http://localhost:3333/api/graphql"),
        "http://localhost:3333/schema.graphql"
    );
    // An endpoint with no path of its own, with and without a trailing slash.
    assert_eq!(
        findopera::api::schema_url("https://example.test"),
        "https://example.test/schema.graphql"
    );
    assert_eq!(
        findopera::api::schema_url("https://example.test/"),
        "https://example.test/schema.graphql"
    );
}
