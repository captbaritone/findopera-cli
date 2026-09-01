//! What findopera.com is actually told about the caller.
//!
//! These assert on the bytes on the wire rather than on the shape of the code,
//! because the property worth keeping is that *every* request carries identity
//! — including the query built into the binary, which no caller passes in and
//! which is therefore the easiest one to forget.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

/// Everything one request said about itself.
struct Asked {
    line: String,
    headers: Vec<String>,
    body: String,
}

impl Asked {
    fn header(&self, name: &str) -> Option<&str> {
        let prefix = format!("{}:", name.to_lowercase());
        self.headers
            .iter()
            .find(|h| h.to_lowercase().starts_with(&prefix))
            .map(|h| h[prefix.len()..].trim())
    }
}

/// A server that answers `count` requests and reports what they asked.
fn serve(count: usize) -> (String, mpsc::Receiver<Asked>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let endpoint = format!("http://{}/api/graphql", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        for _ in 0..count {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();

            let mut headers = Vec::new();
            let mut length = 0usize;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let header = header.trim_end().to_string();
                if header.is_empty() {
                    break;
                }
                if let Some(n) = header.to_lowercase().strip_prefix("content-length:") {
                    length = n.trim().parse().unwrap_or(0);
                }
                headers.push(header);
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();

            // Enough of an answer that the client carries on to the next one.
            let payload = if line.starts_with("GET") {
                "type Query {\n  ok: Int\n}".to_string()
            } else {
                r#"{"data":{"getRecordingByIds":[null]}}"#.to_string()
            };
            let mut stream = &stream;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            )
            .unwrap();
            let _ = stream.flush();

            tx.send(Asked {
                line: line.trim_end().to_string(),
                headers,
                body: String::from_utf8_lossy(&body).into_owned(),
            })
            .unwrap();
        }
    });
    (endpoint, rx)
}

const TOKEN: &str = "a-token-and-nothing-like-a-real-one";

#[test]
fn the_built_in_query_carries_identity() {
    // `organize` never passes a query in — it uses the one generated into the
    // binary. That is the request most easily left anonymous, so it is the one
    // most worth pinning down.
    let (endpoint, asked) = serve(1);
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
    let (endpoint, asked) = serve(1);
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
    let (endpoint, asked) = serve(1);
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
    let (endpoint, asked) = serve(1);
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
    let (endpoint, asked) = serve(1);
    let api = findopera::api::Client::new(&endpoint, None);
    let _ = api.post("{ ok }", None);

    let request = asked.recv().expect("the server was asked something");
    assert_eq!(request.header("authorization"), None);
}
