//! What a swapped transport still runs.
//!
//! The seam replaces the socket and nothing else, so these go through the
//! real retry loop, the real status handling, the real JSON parsing and the
//! real GraphQL refusal check. If any of those moved below the seam these
//! tests would stop meaning anything, which is the property worth pinning.

use findopera::api::{ApiError, Client, Method, Reply, Request, Transport};
use std::sync::Mutex;
use std::time::Duration;

/// What one request said, as a test wants to read it back.
type Asked = (Method, String, Vec<(String, String)>, Option<String>);

struct Script {
    replies: Mutex<Vec<Reply>>,
    asked: Mutex<Vec<Asked>>,
}

/// A transport that answers from a script and remembers what it was asked.
///
/// Cloned rather than shared by reference so that the test keeps a handle on
/// what was sent after the client has taken ownership of the transport.
#[derive(Clone)]
struct Scripted(std::sync::Arc<Script>);

impl Scripted {
    fn new(replies: Vec<Reply>) -> Scripted {
        Scripted(std::sync::Arc::new(Script {
            replies: Mutex::new(replies.into_iter().rev().collect()),
            asked: Mutex::new(Vec::new()),
        }))
    }

    fn asked(&self) -> std::sync::MutexGuard<'_, Vec<Asked>> {
        self.0.asked.lock().unwrap()
    }
}

impl Transport for Scripted {
    fn round_trip(&self, request: &Request) -> Result<Reply, String> {
        self.asked().push((
            request.method,
            request.url.clone(),
            request
                .headers
                .iter()
                .map(|(n, v)| (n.to_string(), v.clone()))
                .collect(),
            request.body.as_ref().map(|b| b.to_string()),
        ));
        self.0
            .replies
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| "the script ran out of replies".to_string())
    }
}

fn reply(status: u16, body: &str) -> Reply {
    Reply {
        status,
        retry_after: None,
        disposition: None,
        body: body.to_string(),
    }
}

#[test]
fn the_client_still_chooses_what_to_say_about_itself() {
    // The headers are built above the seam, so swapping the socket does not
    // quietly make requests anonymous.
    let script = Scripted::new(vec![reply(200, r#"{"data":{"ok":1}}"#)]);
    let api = Client::with_transport(
        "https://example.invalid/api/graphql",
        Some("a-token".to_string()),
        Box::new(script.clone()),
    );
    let _ = api.post("{ ok }", None);

    let asked = script.asked();
    let (method, url, headers, body) = &asked[0];
    assert_eq!(*method, Method::Post);
    assert_eq!(url, "https://example.invalid/api/graphql");
    let header = |name: &str| {
        headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(header("Authorization"), Some("Bearer a-token"));
    assert_eq!(header("User-Agent"), Some(findopera::api::USER_AGENT));
    assert!(body.as_ref().unwrap().contains("{ ok }"));
}

#[test]
fn without_a_token_nothing_is_claimed() {
    let script = Scripted::new(vec![reply(200, r#"{"data":{"ok":1}}"#)]);
    let api = Client::with_transport("https://example.invalid/g", None, Box::new(script.clone()));
    let _ = api.post("{ ok }", None);

    let asked = script.asked();
    assert!(
        !asked[0].2.iter().any(|(n, _)| n == "Authorization"),
        "an empty or absent token must send no header at all"
    );
}

#[test]
fn a_graphql_refusal_is_still_read_out_of_a_200() {
    // The refusal check lives above the seam. A GraphQL server reports its
    // complaint in the body of a perfectly successful response, so a client
    // that only looked at the status would call this a success.
    let script = Scripted::new(vec![reply(
        200,
        r#"{"errors":[{"message":"no such singer","extensions":{"code":"NOT_FOUND"}}]}"#,
    )]);
    let api = Client::with_transport("https://example.invalid/g", None, Box::new(script));

    let error = api
        .query_named("query Q { x }", "Q", serde_json::json!({}))
        .expect_err("errors in the body are a refusal");
    assert!(matches!(error, ApiError::Refused(_)), "got: {error}");
    assert!(error.to_string().contains("no such singer"), "got: {error}");
}

#[test]
fn a_body_that_is_not_json_is_reported_against_the_status() {
    // A proxy or an error page, which is what this usually is.
    let script = Scripted::new(vec![reply(502, "<html>Bad Gateway</html>")]);
    let api = Client::with_transport("https://example.invalid/g", None, Box::new(script));

    let error = api.post("{ ok }", None).expect_err("html is not an answer");
    let said = error.to_string();
    assert!(said.contains("502"), "the status is the clue, got: {said}");
}

#[test]
fn a_429_is_waited_out_and_retried() {
    // The retry loop is above the seam, so this exercises the real one — with
    // a Retry-After small enough not to make the suite slow.
    let mut first = reply(429, "slow down");
    first.retry_after = Some(Duration::from_secs(0));
    let script = Scripted::new(vec![first, reply(200, r#"{"data":{"ok":1}}"#)]);
    let api = Client::with_transport("https://example.invalid/g", None, Box::new(script.clone()));

    let answer = api.post("{ ok }", None).expect("the retry succeeds");
    assert_eq!(answer["data"]["ok"], 1);
    assert_eq!(
        script.asked().len(),
        2,
        "the first attempt was refused and the second was made"
    );
}

#[test]
fn the_notes_filename_comes_from_the_header() {
    let mut r = reply(200, "the notes themselves");
    r.disposition = Some("attachment; filename=\"Tosca [findopera-264].txt\"".to_string());
    let script = Scripted::new(vec![r]);
    let api = Client::with_transport(
        "https://findopera.com/api/graphql",
        None,
        Box::new(script.clone()),
    );

    let notes = api.notes("264").expect("the notes arrive");
    assert_eq!(notes.filename, "Tosca [findopera-264].txt");
    assert_eq!(notes.body, "the notes themselves");
    assert_eq!(script.asked()[0].0, Method::Get);
}

#[test]
fn a_missing_recording_is_named_rather_than_reported_as_a_status() {
    let script = Scripted::new(vec![reply(404, "not found")]);
    let api = Client::with_transport("https://findopera.com/api/graphql", None, Box::new(script));

    let error = api.notes("999999").expect_err("there is no such recording");
    assert!(
        error.to_string().contains("no recording 999999"),
        "got: {error}"
    );
}
