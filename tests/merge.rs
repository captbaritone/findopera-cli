//! What a merge actually sends, and which types may be sent one.
//!
//! Merging is the one mutation that destroys a record without being called
//! `delete`, and the losing id is an argument rather than the subject of the
//! sentence. Getting `id` and `intoId` the wrong way round would merge the
//! survivor into the duplicate — a mistake nothing downstream could notice —
//! so the bytes on the wire are pinned here rather than assumed.

use findopera::model::crud;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

/// A server that answers one mutation and reports the body it was sent.
fn serve(reply: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let endpoint = format!("http://{}/api/graphql", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        let mut length = 0usize;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            let header = header.trim_end().to_string();
            if header.is_empty() {
                break;
            }
            if let Some(v) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                length = v.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).ok();

        let mut stream = &stream;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
            reply.len()
        )
        .unwrap();
        let _ = stream.flush();

        tx.send(String::from_utf8_lossy(&body).into_owned())
            .unwrap();
    });
    (endpoint, rx)
}

fn kind(name: &str) -> &'static crud::Type {
    crud::TYPES
        .iter()
        .find(|t| t.name == name)
        .expect("a type by that name")
}

#[test]
fn the_loser_is_the_subject_and_the_survivor_is_into_id() {
    let (endpoint, asked) = serve(r#"{"data":{"mergeSinger":{"id":"456"}}}"#);
    let api = findopera::api::Client::new(&endpoint, Some("a-token".to_string()));

    let survivor = api
        .merge(kind("singer"), "133", "456", "same person, two spellings")
        .expect("the merge is accepted");

    let body: serde_json::Value =
        serde_json::from_str(&asked.recv().expect("the server was asked something"))
            .expect("the request body is JSON");

    assert_eq!(body["variables"]["id"], "133", "the losing id");
    assert_eq!(body["variables"]["intoId"], "456", "the surviving id");
    assert_eq!(
        body["variables"]["justification"],
        "same person, two spellings"
    );
    let query = body["query"].as_str().expect("a query");
    assert!(query.contains("mergeSinger"), "got: {query}");

    // The survivor is what comes back, not the id that was passed in: an id
    // handed to another command has to be one that still resolves.
    assert_eq!(survivor, "456");
}

#[test]
fn each_type_is_merged_by_its_own_mutation() {
    // One mutation per type rather than a general one, so a table that drifted
    // would send `mergeSinger` for an opera and be refused in terms naming
    // neither.
    let (endpoint, asked) = serve(r#"{"data":{"mergeOpera":{"id":"34"}}}"#);
    let api = findopera::api::Client::new(&endpoint, None);
    let _ = api.merge(kind("opera"), "12", "34", "https://...");

    let body = asked.recv().expect("the server was asked something");
    assert!(body.contains("mergeOpera"), "got: {body}");
}

#[test]
fn a_type_the_server_cannot_merge_is_refused_without_asking() {
    // No server here at all: an unmergeable type must be settled before a
    // request is built, or the reply is a GraphQL error about a field that
    // does not exist rather than an answer about the type in hand.
    let api = findopera::api::Client::new("http://127.0.0.1:1", None);
    let error = api
        .merge(kind("upc"), "1", "2", "a reason")
        .expect_err("a upc cannot be merged");

    assert!(
        matches!(error, findopera::api::ApiError::Refused(_)),
        "a refusal rather than an unreachable server, got: {error}"
    );
    assert!(
        error.to_string().contains("cannot be merged"),
        "got: {error}"
    );
}

#[test]
fn merge_is_offered_for_exactly_the_types_the_schema_merges() {
    // The list is derived from the schema by codegen, so this pins the shape
    // of what it derived: every merge names its own type, and the types with
    // one are the ones the server actually offers. A schema that grows another
    // makes this fail, which is the reminder to check the verb's help still
    // reads true.
    let mergeable: Vec<&str> = crud::TYPES
        .iter()
        .filter(|t| t.merge.is_some())
        .map(|t| t.name)
        .collect();
    assert_eq!(
        mergeable,
        [
            "character",
            "composer",
            "conductor",
            "language",
            "opera",
            "recording",
            "singer"
        ],
    );

    for t in crud::TYPES {
        if let Some(mutation) = t.merge {
            assert_eq!(
                mutation,
                format!("merge{}", t.graphql),
                "{} merges through the wrong mutation",
                t.name
            );
        }
    }
}
