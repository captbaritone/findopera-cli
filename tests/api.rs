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

/// What the server says about versions, and what this program does with it.
///
/// The split is deliberate: this program knows which queries are its own,
/// which the server cannot tell, and the server knows what the newest release
/// is, which this program would otherwise have to ask GitHub about mid-error.
/// Neither half guesses.
mod out_of_step {
    use super::support::server::{serve, Answer};

    /// A refusal to a query that ships with this program.
    const INVALID: &str = r#"{"errors":[{"message":"Cannot query field \"tracks\" on type \"SpotifyAlbum\".","extensions":{"code":"GRAPHQL_VALIDATION_FAILED"}}]}"#;

    fn refusing(latest: Option<&'static str>) -> String {
        let (endpoint, _asked) = serve(1, move |_, _| {
            let answer = Answer::ok(INVALID);
            match latest {
                Some(v) => answer.header("x-findopera-cli-latest", v),
                None => answer,
            }
        });
        findopera::api::Client::new(&endpoint, None)
            .query_named("query Ours { __typename }", "Ours", serde_json::json!({}))
            .expect_err("the server refused")
            .to_string()
    }

    #[test]
    fn a_newer_release_is_named_and_the_way_to_get_it_given() {
        let said = refusing(Some("99.0.0"));
        assert!(said.contains("no longer agree"), "{said}");
        assert!(said.contains("99.0.0"), "{said}");
        assert!(
            said.contains("install.sh") || said.contains("install.ps1"),
            "{said}"
        );
        // The server's own words survive: they name the field that went away,
        // which is the only clue to what actually changed.
        assert!(said.contains("Cannot query field"), "{said}");
    }

    #[test]
    fn being_on_the_newest_release_already_means_this_is_a_bug_not_an_upgrade() {
        // The branch that catches a release going out broken. Telling somebody
        // to upgrade to what they are already running sends them in a circle.
        let said = refusing(Some(findopera::release::CURRENT));
        assert!(said.contains("newest published version"), "{said}");
        assert!(said.contains("issues"), "{said}");
        assert!(!said.contains("To upgrade"), "{said}");
    }

    #[test]
    fn a_server_that_says_nothing_still_gets_the_likely_answer() {
        // Every server said nothing until the header shipped, and one pointed
        // at by `--endpoint` may never say anything.
        let said = refusing(None);
        assert!(said.contains("no longer agree"), "{said}");
        assert!(said.contains("Upgrading is usually the fix"), "{said}");
    }

    #[test]
    fn a_version_that_cannot_be_read_is_treated_as_no_answer() {
        // A header we cannot parse is not grounds for telling somebody to
        // reinstall against a version that may not exist.
        let said = refusing(Some("not-a-version"));
        assert!(said.contains("Upgrading is usually the fix"), "{said}");
        assert!(!said.contains("not-a-version"), "{said}");
    }

    #[test]
    fn a_refusal_that_is_not_about_validation_is_left_alone() {
        // Being told a record does not exist has nothing to do with versions.
        let (endpoint, _asked) = serve(1, |_, _| {
            Answer::ok(
                r#"{"errors":[{"message":"Unauthorized","extensions":{"code":"UNAUTHORIZED"}}]}"#,
            )
            .header("x-findopera-cli-latest", "99.0.0")
        });
        let said = findopera::api::Client::new(&endpoint, None)
            .query_named("query Ours { __typename }", "Ours", serde_json::json!({}))
            .expect_err("the server refused")
            .to_string();
        assert!(said.contains("the server refused the request"), "{said}");
        assert!(!said.contains("no longer agree"), "{said}");
    }

    #[test]
    fn a_query_the_user_wrote_is_never_blamed_on_the_version() {
        // `findopera graphql` sends whatever it is given, so an invalid query
        // there is the user's to fix and upgrading would not help. That path
        // reads the refusal directly rather than through the client, and this
        // is what holds it there.
        let payload: serde_json::Value = serde_json::from_str(INVALID).unwrap();
        let said = findopera::api::refusal(&payload)
            .expect("a refusal")
            .to_string();
        assert!(said.contains("Cannot query field"), "{said}");
        assert!(!said.contains("no longer agree"), "{said}");
        assert!(!said.contains("upgrade"), "{said}");
    }
}
