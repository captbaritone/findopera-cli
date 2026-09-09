//! One stand-in server on a real socket.
//!
//! For the tests whose subject is the HTTP itself: what this program says
//! about who it is, and what it does when told to come back later. Everything
//! else should reach for `scripted`, which needs no port and no thread.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

/// Everything one request said about itself.
pub struct Asked {
    pub line: String,
    pub headers: Vec<String>,
    pub body: String,
}

impl Asked {
    pub fn header(&self, name: &str) -> Option<&str> {
        let prefix = format!("{}:", name.to_lowercase());
        self.headers
            .iter()
            .find(|h| h.to_lowercase().starts_with(&prefix))
            .map(|h| h[prefix.len()..].trim())
    }

    pub fn is_get(&self) -> bool {
        self.line.starts_with("GET")
    }
}

/// What to answer with.
pub struct Answer {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Answer {
    pub fn ok(body: &str) -> Answer {
        Answer {
            status: 200,
            headers: Vec::new(),
            body: body.to_string(),
        }
    }

    pub fn status(status: u16, body: &str) -> Answer {
        Answer {
            status,
            headers: Vec::new(),
            body: body.to_string(),
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Answer {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

/// A server that answers `count` requests, and reports what each one asked.
///
/// The answer is chosen per request by `answer`, which is given the index, so
/// a test can refuse the first and accept the second without a script format
/// to learn.
pub fn serve(
    count: usize,
    answer: impl Fn(usize, &Asked) -> Answer + Send + 'static,
) -> (String, mpsc::Receiver<Asked>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let endpoint = format!("http://{}/api/graphql", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        for i in 0..count {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            let mut reader = BufReader::new(&stream);

            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            let mut headers = Vec::new();
            let mut length = 0usize;
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).unwrap_or(0) == 0 {
                    break;
                }
                let header = header.trim_end().to_string();
                if header.is_empty() {
                    break;
                }
                if let Some(v) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap_or(0);
                }
                headers.push(header);
            }
            // Read the body, so the client is never left writing into a pipe
            // nobody is reading.
            let mut body = vec![0u8; length];
            reader.read_exact(&mut body).ok();

            let asked = Asked {
                line: line.trim_end().to_string(),
                headers,
                body: String::from_utf8_lossy(&body).into_owned(),
            };
            let reply = answer(i, &asked);

            let mut stream = &stream;
            let mut head = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
                reply.status,
                if (200..300).contains(&reply.status) { "OK" } else { "Error" },
                reply.body.len()
            );
            for (name, value) in &reply.headers {
                head.push_str(&format!("{name}: {value}\r\n"));
            }
            head.push_str("\r\n");
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(reply.body.as_bytes());
            let _ = stream.flush();

            let _ = tx.send(asked);
        }
    });
    (endpoint, rx)
}

/// The common case: every request gets the same answer.
pub fn serving(count: usize, body: &'static str) -> (String, mpsc::Receiver<Asked>) {
    serve(count, move |_, asked| {
        // A GET is the schema fetch, which is SDL rather than JSON.
        if asked.is_get() {
            Answer::ok("type Query {\n  ok: Int\n}")
        } else {
            Answer::ok(body)
        }
    })
}
