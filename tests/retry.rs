//! Waiting out a server that is asking for less traffic.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A server that refuses the first `refusals` requests, then relents.
///
/// Returns the endpoint and a counter of how many requests it actually saw,
/// which is the only way to tell a retry from a client that gave up quietly.
fn grudging(refusals: usize, retry_after: Option<&'static str>) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let endpoint = format!("http://{}/api/graphql", listener.local_addr().unwrap());
    let seen = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&seen);

    std::thread::spawn(move || {
        while let Ok((stream, _)) = listener.accept() {
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                return;
            }
            let mut length = 0usize;
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).is_err() {
                    return;
                }
                let header = header.trim_end().to_string();
                if header.is_empty() {
                    break;
                }
                if let Some(n) = header.to_lowercase().strip_prefix("content-length:") {
                    length = n.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);

            let nth = count.fetch_add(1, Ordering::SeqCst);
            let mut stream = &stream;
            if nth < refusals {
                let payload = r#"{"errors":[{"message":"Too many requests.","extensions":{"code":"RATE_LIMITED"}}]}"#;
                let after = retry_after
                    .map(|s| format!("Retry-After: {s}\r\n"))
                    .unwrap_or_default();
                let _ = write!(
                    stream,
                    "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\n{after}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
            } else {
                let payload = r#"{"data":{"ok":1}}"#;
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
            }
            let _ = stream.flush();
        }
    });
    (endpoint, seen)
}

#[test]
fn a_request_told_to_wait_is_sent_again() {
    let (endpoint, seen) = grudging(2, Some("1"));
    let api = findopera::api::Client::new(&endpoint, Some("t".into()));

    let payload = api
        .post("{ ok }", None)
        .expect("it should get there eventually");
    assert_eq!(payload["data"]["ok"], 1);
    assert_eq!(
        seen.load(Ordering::SeqCst),
        3,
        "two refusals, then the answer"
    );
}

#[test]
fn giving_up_says_what_to_do_about_it() {
    // Refuses forever. The message has to be about traffic rather than a bare
    // status, because the answer is to stop asking rather than to look for a
    // bug.
    let (endpoint, seen) = grudging(usize::MAX, Some("1"));
    let api = findopera::api::Client::new(&endpoint, Some("t".into()));

    let why = api.post("{ ok }", None).expect_err("it never relents");
    let why = why.to_string();
    assert!(why.contains("refusing"), "got: {why}");
    assert!(why.contains("wait"), "got: {why}");
    assert_eq!(seen.load(Ordering::SeqCst), 5, "one try and four retries");
}

#[test]
fn an_anonymous_caller_is_told_a_token_would_help() {
    // Anonymous callers share a much smaller budget, so for them the limit is
    // the likely cause rather than a coincidence — and there is something they
    // can actually do about it.
    let (endpoint, _) = grudging(usize::MAX, Some("1"));
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
    let (endpoint, seen) = grudging(1, None);
    let api = findopera::api::Client::new(&endpoint, Some("t".into()));

    let payload = api.post("{ ok }", None).expect("it relents after one");
    assert_eq!(payload["data"]["ok"], 1);
    assert_eq!(seen.load(Ordering::SeqCst), 2);
}
