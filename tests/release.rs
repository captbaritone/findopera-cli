//! The release check, against a stand-in for GitHub.
//!
//! Offline, like the rest: a listener on a loose port answers the releases
//! API, so what is exercised is the request that actually goes out and the
//! reading of what comes back — not a mock of this program's own idea of
//! either.

use findopera::release::{self, How};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::mpsc;

/// A server that answers one request with `status` and `body`, and reports
/// the request line and headers it was sent.
fn serve(status: u16, body: &'static str) -> (String, mpsc::Receiver<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("http://{}/releases/latest", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        let mut reader = BufReader::new(&stream);
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            let line = line.trim_end().to_string();
            if line.is_empty() {
                break;
            }
            lines.push(line);
        }

        let reason = if status == 200 { "OK" } else { "Error" };
        let mut stream = &stream;
        write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .ok();
        let _ = stream.flush();
        let _ = tx.send(lines);
    });
    (url, rx)
}

#[test]
fn a_newer_release_is_reported_as_newer() {
    let (url, _asked) = serve(200, r#"{"tag_name":"v99.0.0"}"#);
    let check = release::check(&url, Some(Path::new("/usr/local/bin/findopera")))
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
    let (url, _asked) = serve(200, body);
    let check = release::check(&url, None).expect("the release was read");

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
    let (url, _asked) = serve(200, r#"{"tag_name":"v0.0.1"}"#);
    let check = release::check(&url, None).expect("the release was read");
    assert!(!check.newer_available());
}

#[test]
fn the_request_says_who_is_asking_and_which_api_it_wants() {
    // GitHub refuses a request with no user-agent outright, and the Accept
    // header is what pins the response shape this parses.
    let (url, asked) = serve(200, r#"{"tag_name":"v99.0.0"}"#);
    let _ = release::check(&url, None);

    let headers = asked.recv().expect("the server was asked something");
    let header = |name: &str| {
        headers
            .iter()
            .find(|h| h.to_lowercase().starts_with(&format!("{name}:")))
            .map(|h| h[name.len() + 1..].trim().to_string())
    };
    assert_eq!(
        header("user-agent").as_deref(),
        Some(findopera::api::USER_AGENT)
    );
    assert_eq!(
        header("accept").as_deref(),
        Some("application/vnd.github+json")
    );
}

#[test]
fn being_rate_limited_says_so_rather_than_saying_forbidden() {
    // Anonymous callers meet this often enough that "403" on its own would
    // send someone looking for a permission they do not need.
    let (url, _asked) = serve(403, r#"{"message":"rate limit exceeded"}"#);
    let error = release::check(&url, None).expect_err("403 is not an answer");
    let said = error.to_string();
    assert!(said.contains("rate limit"), "got: {said}");
}

#[test]
fn a_tag_that_is_not_a_version_is_an_error() {
    let (url, _asked) = serve(200, r#"{"tag_name":"nightly"}"#);
    let error = release::check(&url, None).expect_err("`nightly` is not a version");
    assert!(
        matches!(error, release::Error::Unreadable(_)),
        "got: {error}"
    );
}

#[test]
fn where_the_binary_sits_decides_what_it_is_told_to_run() {
    let (url, _asked) = serve(200, r#"{"tag_name":"v99.0.0"}"#);
    let check = release::check(&url, Some(Path::new("/home/me/.cargo/bin/findopera")))
        .expect("the release was read");

    assert_eq!(check.how, How::Cargo);
    assert!(
        check.how.command().starts_with("cargo install"),
        "got: {}",
        check.how.command()
    );
}
