//! What the client tells the server about itself.
//!
//! Offline: a listener on a loose port stands in for findopera.com, so this
//! tests the bytes that actually go out rather than the constant they are
//! built from.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

/// Take one request, hand back an empty GraphQL response, and report the
/// headers that arrived.
fn one_request(listener: TcpListener) -> Vec<String> {
    let (stream, _) = listener.accept().expect("a connection");
    let mut reader = BufReader::new(stream);
    let mut headers = Vec::new();
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).expect("a header line") == 0 {
            break;
        }
        let line = line.trim_end().to_string();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        }
        headers.push(line);
    }
    // Read the body so the client is not left writing into a closed pipe.
    let mut body = vec![0u8; length];
    std::io::Read::read_exact(&mut reader, &mut body).ok();

    let payload = br#"{"data":{"getRecordingByIds":[null]}}"#;
    let mut stream = reader.into_inner();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        payload.len()
    )
    .expect("respond");
    stream.write_all(payload).expect("respond");
    stream.flush().ok();
    headers
}

#[test]
fn every_request_says_which_program_and_version_it_is() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let endpoint = format!("http://{}", listener.local_addr().expect("an address"));
    let server = std::thread::spawn(move || one_request(listener));

    let ids = vec!["75".to_string()];
    let _ = findopera::api::recordings(&endpoint, &ids);

    let headers = server.join().expect("the server thread");
    let agent = headers
        .iter()
        .find_map(|h| {
            let (name, value) = h.split_once(':')?;
            name.eq_ignore_ascii_case("user-agent")
                .then(|| value.trim().to_string())
        })
        .unwrap_or_else(|| panic!("no User-Agent among {headers:?}"));

    assert_eq!(agent, findopera::api::USER_AGENT);
    assert_eq!(
        agent,
        format!("findopera-cli/{}", env!("CARGO_PKG_VERSION")),
        "the version has to come from the package, not a copy of it"
    );
}

/// Reply to one request with the given body, and report nothing.
fn respond_with(listener: TcpListener, payload: &'static [u8]) {
    let (stream, _) = listener.accept().expect("a connection");
    let mut reader = BufReader::new(stream);
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).expect("a header line") == 0 {
            break;
        }
        if line.trim_end().is_empty() {
            break;
        }
        if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; length];
    std::io::Read::read_exact(&mut reader, &mut body).ok();
    let mut stream = reader.into_inner();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        payload.len()
    )
    .expect("respond");
    stream.write_all(payload).expect("respond");
    stream.flush().ok();
}

fn ask(payload: &'static [u8]) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let endpoint = format!("http://{}", listener.local_addr().expect("an address"));
    let server = std::thread::spawn(move || respond_with(listener, payload));
    let result = findopera::api::recordings(&endpoint, &["75".to_string()]);
    server.join().expect("the server thread");
    result.map(|_| ()).map_err(|e| e.to_string())
}

#[test]
fn a_top_level_error_is_fatal_and_reaches_the_reader_intact() {
    // The channel the server uses to say a client is too old. The words are
    // the whole point, so they must not be summarised away.
    let why = ask(
        br#"{"errors":[{"message":"findopera-cli 0.1.0 is no longer supported; upgrade to 0.3 or later","extensions":{"code":"CLIENT_TOO_OLD"}}]}"#,
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
        ask(br#"{"errors":[{"message":"first"},{"message":"second"}]}"#).expect_err("still fatal");
    assert!(why.contains("\n    first"), "got: {why}");
    assert!(why.contains("\n    second"), "got: {why}");
}

#[test]
fn an_error_is_fatal_even_when_data_came_with_it() {
    // Partial data cannot be trusted: a null in a @semanticNonNull position is
    // explained by exactly one of these errors.
    let why = ask(br#"{"data":{"getRecordingByIds":[null]},"errors":[{"message":"partial"}]}"#)
        .expect_err("data alongside an error is not enough");
    assert!(why.contains("partial"), "got: {why}");
}

#[test]
fn an_error_with_no_message_still_says_something() {
    let why = ask(br#"{"errors":[{"extensions":{"code":"WEIRD"}}]}"#).expect_err("fatal");
    assert!(why.contains("WEIRD"), "got: {why}");
    assert!(why.contains("no message given"), "got: {why}");
}

#[test]
fn an_empty_errors_list_is_not_an_error() {
    // The spec says the field is absent unless non-empty, but a server that
    // sends `"errors": []` should not be read as refusing.
    ask(br#"{"errors":[],"data":{"getRecordingByIds":[null]}}"#).expect("nothing was reported");
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
