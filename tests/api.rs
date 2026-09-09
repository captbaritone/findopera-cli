//! What the client tells the server about itself.
//!
//! Offline: a listener on a loose port stands in for findopera.com, so this
//! tests the bytes that actually go out rather than the constant they are
//! built from.
//!
//! What the server *says* is not here. How a refusal reads is a case under
//! `tests/snapshots/refusals/`, judged on what a person sees rather than on
//! a substring of an error type. What is left is the request itself, and
//! where the schema is fetched from — neither of which any command reports.

mod support;

use support::server::{serve, Answer};

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
