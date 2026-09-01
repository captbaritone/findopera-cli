//! Fetching recordings from the FindOpera GraphQL API.

use crate::model::{Recording, QUERY};
use std::collections::BTreeMap;
use std::time::Duration;

pub const DEFAULT_ENDPOINT: &str = "https://findopera.com/api/graphql";

/// Ids per request. A scan of a real library turns up thousands, and asking
/// for them all in one query is a good way to be told no.
const BATCH: usize = 100;

/// How many times a request that was told "later" is sent again.
///
/// Small on purpose. A caller who has said who they are has a budget far
/// larger than this program will use, so meeting the limit at all means
/// something unusual, and the useful response to that is to wait a little and
/// then say so — not to keep asking until the server gives in.
const RETRIES: u32 = 4;

/// The longest this will sit waiting across one call, however long it is asked
/// to wait. Past this it is better to stop and let someone decide.
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(180);

/// What this program calls itself when it asks findopera.com for something.
///
/// Taken from the package version, so it cannot fall behind a release, and
/// sent on every request — someone reading their own logs should be able to
/// tell this apart from a browser, and tell one version of it from another.
pub const USER_AGENT: &str = concat!("findopera-cli/", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub struct ApiError(String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Where the schema lives, for a given API endpoint.
///
/// Derived from the endpoint rather than configured separately, so that
/// pointing this program at a development server moves both together and
/// there is no second setting to leave behind.
pub fn schema_url(endpoint: &str) -> String {
    let scheme = endpoint.find("://").map_or(0, |i| i + 3);
    let host = match endpoint[scheme..].find('/') {
        Some(i) => &endpoint[..scheme + i],
        None => endpoint.trim_end_matches('/'),
    };
    format!("{host}/schema.graphql")
}

/// How long the server asked us to wait, if it said.
///
/// `Retry-After` may also carry a date, which this does not read: the servers
/// this talks to send seconds, and guessing at a date badly would be worse
/// than falling back to a delay of our own.
fn retry_after(response: &ureq::http::Response<ureq::Body>) -> Option<Duration> {
    let value = response.headers().get("retry-after")?.to_str().ok()?;
    let seconds: u64 = value.trim().parse().ok()?;
    Some(Duration::from_secs(seconds))
}

/// How long to wait when the server did not say.
///
/// Doubling, from a second. Without a `Retry-After` there is nothing to go on,
/// and easing off is the only way to stop adding to whatever is wrong.
fn backoff(attempt: u32) -> Duration {
    Duration::from_secs(1u64 << attempt.min(6))
}

/// What the server said was wrong with the request, if it said anything.
///
/// A GraphQL response can carry both data and errors, and for this client an
/// error is always fatal: `@semanticNonNull` means a null in an annotated
/// position is explained by exactly one of these, so data alongside an error
/// cannot be trusted to be whole.
///
/// It is also how the server talks to whoever is running this — every request
/// says which version it is, and this is the channel for the answer "that one
/// is too old". So the words come through as they were written, one to a
/// line, rather than being summarised into a single line that a long
/// explanation would not survive.
pub fn refusal(payload: &serde_json::Value) -> Option<String> {
    let errors = payload.get("errors")?.as_array()?;
    if errors.is_empty() {
        return None;
    }
    let mut said = String::from("the server refused the request:");
    for error in errors {
        let message = error["message"].as_str().unwrap_or("(no message given)");
        // A code is where a server puts something a program can act on, so it
        // is worth showing beside the words meant for a person.
        match error["extensions"]["code"].as_str() {
            Some(code) => said.push_str(&format!("\n    [{code}] {message}")),
            None => said.push_str(&format!("\n    {message}")),
        }
    }
    Some(said)
}

/// A server, and who this program is when it talks to it.
///
/// Everything that reaches findopera.com goes through one of these, so there
/// is a single place deciding what it is told about the caller, and no way to
/// add a request that quietly says nothing.
pub struct Client {
    endpoint: String,
    token: Option<String>,
}

impl Client {
    pub fn new(endpoint: impl Into<String>, token: Option<String>) -> Self {
        Client {
            endpoint: endpoint.into(),
            token,
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Whether this client has a token at all.
    ///
    /// Only ever the fact, never the value — nothing outside this module has a
    /// reason to see the token, and the surest way to keep it out of a log is
    /// to keep it out of reach.
    pub fn is_identified(&self) -> bool {
        self.token.is_some()
    }

    /// Say who we are.
    ///
    /// The token goes on every request, not only the ones that change
    /// something. Reading is what this program mostly does — a library of
    /// three thousand markers is thirty requests — and a server that cannot
    /// tell those apart from a stranger's has to treat them like a stranger's.
    /// Identifying the reads is what earns them a limit of their own.
    fn identify<Any>(&self, request: ureq::RequestBuilder<Any>) -> ureq::RequestBuilder<Any> {
        let request = request.header("User-Agent", USER_AGENT);
        match &self.token {
            Some(token) => request.header("Authorization", &format!("Bearer {token}")),
            None => request,
        }
    }

    /// Send a GraphQL document, and hand back the whole response.
    ///
    /// Nothing here judges what came back: a GraphQL server answers a request
    /// it disliked with an ordinary 200 and an `errors` array, and deciding
    /// what that means is the caller's business — see [`refusal`]. This
    /// returns `Err` only when there is no response to judge.
    ///
    /// A request the server asked us to retry is retried here rather than
    /// reported, because the caller almost never has a better answer than
    /// waiting: `organize` is thirty requests in a row, and failing the
    /// twenty-ninth would throw away the twenty-eight before it.
    pub fn post(
        &self,
        query: &str,
        variables: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ApiError> {
        let mut body = serde_json::json!({ "query": query });
        if let Some(variables) = variables {
            body["variables"] = variables;
        }
        self.with_retries(&self.endpoint.clone(), |client| {
            // A failing status is not an error, because a GraphQL server puts
            // its reason in the body — including the one reason this client
            // most needs to hear, that its version is no longer welcome.
            // Letting ureq turn a 400 into `StatusCode` would throw that away
            // and report a number.
            let request = ureq::post(&client.endpoint)
                .config()
                .http_status_as_error(false)
                .build();
            client.identify(request).send_json(&body)
        })
        .and_then(|(status, mut response)| {
            response.body_mut().read_json().map_err(|e| {
                // A body that is not JSON is usually a proxy or an error page
                // rather than the API, and the status is the only clue as to
                // which.
                if status.is_success() {
                    ApiError(format!("the API returned something unreadable: {e}"))
                } else {
                    ApiError(format!(
                        "{} answered {status}, and not with JSON",
                        self.endpoint
                    ))
                }
            })
        })
    }

    /// Send something, waiting out any request to come back later.
    ///
    /// Waiting is announced. A run that has gone quiet for a minute looks
    /// exactly like one that has hung, and someone watching it has no way to
    /// tell the difference unless they are told.
    fn with_retries(
        &self,
        what: &str,
        send: impl Fn(&Self) -> Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<(ureq::http::StatusCode, ureq::http::Response<ureq::Body>), ApiError> {
        let mut spent = Duration::ZERO;

        for attempt in 0..=RETRIES {
            let response = send(self).map_err(|e| ApiError(format!("cannot reach {what}: {e}")))?;
            let status = response.status();

            if status.as_u16() != 429 {
                return Ok((status, response));
            }
            if attempt == RETRIES {
                return Err(ApiError(format!(
                    "{what} is still refusing after {} attempts. It is asking for less \
                     traffic, so the thing to do is wait rather than try again now.{}",
                    RETRIES + 1,
                    if self.is_identified() {
                        ""
                    } else {
                        "\n    These requests are anonymous, and anonymous callers share a \
                         much smaller budget.\n    `findopera login --new` gets a token."
                    }
                )));
            }

            let wait = retry_after(&response).unwrap_or_else(|| backoff(attempt));
            if spent + wait > PATIENCE {
                return Err(ApiError(format!(
                    "{what} asked to be left alone for another {}s, which is longer than this \
                     will wait. Try again later.",
                    wait.as_secs()
                )));
            }
            eprintln!(
                "findopera: {what} is asking for less traffic — waiting {}s ({} of {})",
                wait.as_secs(),
                attempt + 1,
                RETRIES
            );
            std::thread::sleep(wait);
            spent += wait;
        }
        unreachable!("the loop returns on its last pass")
    }

    /// Fetch the schema, as SDL.
    ///
    /// Fetched rather than built in: the server gains fields between releases
    /// of this program, and a schema that is quietly a version behind is worse
    /// than one that takes a moment to arrive.
    pub fn schema(&self) -> Result<String, ApiError> {
        let url = schema_url(&self.endpoint);
        let (status, mut response) = self.with_retries(&url, |client| {
            let request = ureq::get(&url).config().http_status_as_error(false).build();
            client.identify(request).call()
        })?;

        if !status.is_success() {
            return Err(ApiError(format!("{url} answered {status}")));
        }
        response
            .body_mut()
            .read_to_string()
            .map_err(|e| ApiError(format!("the schema could not be read: {e}")))
    }

    /// Fetch recordings by id.
    ///
    /// The API returns a list positionally aligned with the ids it was given,
    /// with `null` for any it does not know, so the returned map simply omits
    /// those and the caller reports them as not found.
    pub fn recordings(&self, ids: &[String]) -> Result<BTreeMap<String, Recording>, ApiError> {
        let mut out = BTreeMap::new();
        for chunk in ids.chunks(BATCH) {
            out.extend(self.fetch_batch(chunk)?);
        }
        Ok(out)
    }

    fn fetch_batch(&self, ids: &[String]) -> Result<BTreeMap<String, Recording>, ApiError> {
        let payload = self.post(QUERY, Some(serde_json::json!({ "ids": ids })))?;

        if let Some(said) = refusal(&payload) {
            return Err(ApiError(said));
        }

        let list = payload
            .get("data")
            .and_then(|d| d.get("getRecordingByIds"))
            .and_then(|r| r.as_array())
            .ok_or_else(|| ApiError("the API returned no recordings".into()))?;

        let mut out = BTreeMap::new();
        for (id, value) in ids.iter().zip(list) {
            if value.is_null() {
                continue; // an id the database does not know
            }
            let rec: Recording = serde_json::from_value(value.clone()).map_err(|e| {
                ApiError(format!(
                    "recording {id} did not match the generated model: {e}"
                ))
            })?;
            out.insert(id.clone(), rec);
        }
        Ok(out)
    }
}
