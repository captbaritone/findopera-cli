//! A transport that answers from a script and remembers what it was asked.
//!
//! It replaces the socket and nothing else, so a test using it still runs the
//! real retry loop, status handling, JSON parsing and GraphQL refusal check.
//!
//! Answers are keyed by GraphQL operation name — `Recordings`, `Merge`, `Add`
//! — because that is what the request already calls itself, so a case says
//! which answer belongs to which question without inventing a second naming
//! scheme.

use findopera::api::{Client, Method, Reply, Request, Transport};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// One request, as a test wants to read it back.
#[derive(Clone, Debug)]
pub struct Sent {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    /// The GraphQL document, where this was a GraphQL request.
    pub query: Option<String>,
    /// The operation named in the request.
    pub operation: Option<String>,
    pub variables: Option<serde_json::Value>,
}

#[derive(Default)]
struct Inner {
    /// Recordings captured off the real API, answered by the ids asked for.
    corpus: Mutex<Option<Vec<serde_json::Value>>>,
    /// By operation name, and `None` for anything not named.
    replies: Mutex<BTreeMap<String, Vec<Reply>>>,
    fallback: Mutex<Vec<Reply>>,
    sent: Mutex<Vec<Sent>>,
}

/// The transport, and the handle a test keeps on it.
#[derive(Clone, Default)]
pub struct Scripted(Arc<Inner>);

impl Scripted {
    pub fn new() -> Scripted {
        Scripted::default()
    }

    /// Answer this operation with this body, on the next request naming it.
    pub fn answers(self, operation: &str, status: u16, body: &str) -> Scripted {
        self.0
            .replies
            .lock()
            .unwrap()
            .entry(operation.to_string())
            .or_default()
            .push(reply(status, body));
        self
    }

    /// Answer `Recordings` from the captured corpus, by whatever ids are
    /// asked for.
    ///
    /// Built from the request rather than written out in advance, so a case
    /// never has to know the order the CLI happens to ask in — and so an id
    /// the corpus does not hold comes back `null` in the right position, which
    /// is what a missing recording looks like.
    pub fn serving_corpus(self, records: Vec<serde_json::Value>) -> Scripted {
        *self.0.corpus.lock().unwrap() = Some(records);
        self
    }

    /// Answer anything not otherwise scripted.
    pub fn otherwise(self, status: u16, body: &str) -> Scripted {
        self.0.fallback.lock().unwrap().push(reply(status, body));
        self
    }

    /// Answer with a whole `Reply`, for the cases that care about a header.
    pub fn replies_with(self, reply: Reply) -> Scripted {
        self.0.fallback.lock().unwrap().push(reply);
        self
    }

    /// Everything asked so far, in order.
    pub fn sent(&self) -> Vec<Sent> {
        self.0.sent.lock().unwrap().clone()
    }

    /// A client that sends through this, saying nothing to the process.
    pub fn client(&self, endpoint: &str, token: Option<String>) -> Client {
        let notices = self.clone();
        Client::with_transport(endpoint, token, Box::new(self.clone()))
            .on_notice(move |said| notices.note(said))
    }

    fn note(&self, said: &str) {
        // Waiting notices are part of what a case shows, so they are kept
        // rather than printed at whoever is running the tests.
        self.0.sent.lock().unwrap().push(Sent {
            method: Method::Get,
            url: format!("(notice) {said}"),
            headers: Vec::new(),
            query: None,
            operation: None,
            variables: None,
        });
    }
}

pub fn reply(status: u16, body: &str) -> Reply {
    Reply {
        status,
        retry_after: None,
        disposition: None,
        body: body.to_string(),
    }
}

impl Transport for Scripted {
    fn round_trip(&self, request: &Request) -> Result<Reply, String> {
        let body = request.body.as_ref();
        let operation = body
            .and_then(|b| b.get("operationName"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let query = body
            .and_then(|b| b.get("query"))
            .and_then(|v| v.as_str())
            .map(str::to_string);

        // Where the document names its operation but the request does not, the
        // name is still what a case would call it by. Read from the operation
        // definition rather than by searching the text: these documents open
        // with a comment, and the word "query" appears in it.
        let named = operation.clone().or_else(|| {
            query.as_ref().and_then(|q| {
                q.lines()
                    .map(str::trim)
                    .filter(|l| !l.starts_with('#'))
                    .find_map(|l| {
                        let rest = l
                            .strip_prefix("query ")
                            .or_else(|| l.strip_prefix("mutation "))?;
                        let name = rest.split(['(', '{', ' ']).next().unwrap_or(rest).trim();
                        (!name.is_empty()).then(|| name.to_string())
                    })
            })
        });

        self.0.sent.lock().unwrap().push(Sent {
            method: request.method,
            url: request.url.clone(),
            headers: request
                .headers
                .iter()
                .map(|(n, v)| (n.to_string(), v.clone()))
                .collect(),
            query,
            operation: named.clone(),
            variables: body.and_then(|b| b.get("variables")).cloned(),
        });

        if named.as_deref() == Some("Recordings") {
            if let Some(all) = self.0.corpus.lock().unwrap().as_ref() {
                let asked = body
                    .and_then(|b| b.pointer("/variables/ids"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let found: Vec<serde_json::Value> = asked
                    .iter()
                    .map(|id| {
                        let id = id.as_str().unwrap_or_default();
                        all.iter()
                            .find(|r| r["id"].to_string().trim_matches('"') == id)
                            .cloned()
                            .unwrap_or(serde_json::Value::Null)
                    })
                    .collect();
                return Ok(reply(
                    200,
                    &serde_json::json!({ "data": { "getRecordingByIds": found } }).to_string(),
                ));
            }
        }

        if let Some(name) = &named {
            let mut replies = self.0.replies.lock().unwrap();
            if let Some(queue) = replies.get_mut(name) {
                // The last answer stays: a server does not stop knowing
                // something because it was asked twice. A case that wants a
                // different second answer scripts two.
                if queue.len() > 1 {
                    return Ok(queue.remove(0));
                }
                if let Some(last) = queue.first() {
                    return Ok(last.clone());
                }
            }
        }
        let mut fallback = self.0.fallback.lock().unwrap();
        if fallback.is_empty() {
            return Err(format!(
                "nothing scripted for {}",
                named.unwrap_or_else(|| request.url.clone())
            ));
        }
        Ok(fallback.remove(0))
    }
}
