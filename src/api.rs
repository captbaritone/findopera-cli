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

/// Where a recording's notes live, for a given API endpoint.
pub fn notes_url(endpoint: &str, id: &str) -> String {
    let base = schema_url(endpoint);
    let host = base.trim_end_matches("/schema.graphql");
    format!("{host}/recording/{id}.txt")
}

/// A recording's notes, and what findopera.com calls the file.
pub struct Notes {
    /// The name the server gives it, which is the one that will be recognised
    /// again later. Taken from the server rather than assembled here so that
    /// there is one authority for the convention.
    pub filename: String,
    pub body: String,
}

/// Undo the percent-encoding in an RFC 6266 `filename*`.
fn percent_decode(encoded: &str) -> Option<String> {
    let bytes = encoded.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = encoded.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The filename out of a `Content-Disposition`.
///
/// Prefers the RFC 6266 `filename*`, which is UTF-8 and carries the accents;
/// `filename` is the ASCII fallback the same header sends for old clients, and
/// taking it would quietly strip every diacritic out of a composer's name.
fn disposition_filename(header: &str) -> Option<String> {
    if let Some(at) = header.find("filename*=") {
        let value = header[at + "filename*=".len()..].trim();
        let value = value.split(';').next()?.trim().trim_matches('"');
        // `UTF-8''<encoded>`; the middle field is the language, and is empty.
        if let Some(encoded) = value.strip_prefix("UTF-8''") {
            if let Some(decoded) = percent_decode(encoded) {
                return Some(decoded);
            }
        }
    }
    let at = header.find("filename=")?;
    let value = header[at + "filename=".len()..].trim();
    let value = value.split(';').next()?.trim().trim_matches('"');
    (!value.is_empty()).then(|| value.to_string())
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
        self.send(body)
    }

    /// Send one operation out of a document that holds several.
    ///
    /// The lookup queries live together in `schema/search.graphql` so that
    /// codegen validates them in one pass; `operationName` is how GraphQL says
    /// which of them this request means.
    pub fn query_named(
        &self,
        document: &str,
        operation: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        let payload = self.send(serde_json::json!({
            "query": document,
            "operationName": operation,
            "variables": variables,
        }))?;
        if let Some(said) = refusal(&payload) {
            return Err(ApiError(said));
        }
        Ok(payload)
    }

    fn send(&self, body: serde_json::Value) -> Result<serde_json::Value, ApiError> {
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

    /// Fetch a recording's notes, and the name to keep them under.
    ///
    /// Plain HTTP rather than GraphQL, because the notes are a document the
    /// site already serves, formatted for reading. Rendering them here from
    /// the API would be a second implementation to keep in step, and it would
    /// drift.
    pub fn notes(&self, id: &str) -> Result<Notes, ApiError> {
        let url = notes_url(&self.endpoint, id);
        let (status, mut response) = self.with_retries(&url, |client| {
            let request = ureq::get(&url).config().http_status_as_error(false).build();
            client.identify(request).call()
        })?;

        if status.as_u16() == 404 {
            return Err(ApiError(format!("findopera.com has no recording {id}")));
        }
        if !status.is_success() {
            return Err(ApiError(format!("{url} answered {status}")));
        }

        // The name comes from the header rather than from the URL: the URL is
        // whatever was asked for, and the header is what the server says the
        // file should be called.
        let filename = response
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .and_then(disposition_filename)
            .unwrap_or_else(|| format!("findopera-{id}.txt"));

        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|e| ApiError(format!("the notes could not be read: {e}")))?;

        Ok(Notes { filename, body })
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

#[cfg(test)]
mod tests {
    use super::{
        disposition_filename, found, lifespan, notes_url, percent_decode, schema_url, Kind, LOOKUPS,
    };

    const KINDS: [Kind; 6] = [
        Kind::Recording,
        Kind::Opera,
        Kind::Singer,
        Kind::Conductor,
        Kind::Composer,
        Kind::Character,
    ];

    #[test]
    fn every_operation_this_asks_for_is_in_the_document() {
        // Codegen validates search.graphql against the schema, which catches a
        // field that no longer exists. It cannot catch an operation renamed in
        // that file, because the document stays perfectly valid — the request
        // just names something that is not there, and only at runtime. This is
        // the seam between the two, so it is checked here.
        for kind in KINDS {
            let (operation, field) = kind.operation();
            assert!(
                LOOKUPS.contains(&format!("query {operation}(")),
                "search.graphql has no operation `{operation}`"
            );
            assert!(
                LOOKUPS.contains(&format!("{field}(")),
                "search.graphql never selects `{field}`"
            );
        }
    }

    #[test]
    fn a_recording_is_described_by_what_tells_it_apart() {
        let v = serde_json::json!({
            "id": 264, "year": 1953,
            "opera": { "title": "Tosca" },
            "conductor": { "lastName": "de Sabata" },
            "notedSingers": [{ "lastName": "Callas" }, { "lastName": "Gobbi" }],
        });
        let f = found(Kind::Recording, &v);
        assert_eq!(f.id, "264");
        assert_eq!(f.name, "Tosca");
        // The year, the conductor and the cast, because a title alone does not
        // distinguish one Tosca from the forty others.
        assert!(f.about.contains("1953"), "got: {}", f.about);
        assert!(f.about.contains("de Sabata"), "got: {}", f.about);
        assert!(f.about.contains("Callas, Gobbi"), "got: {}", f.about);
    }

    #[test]
    fn a_recording_missing_its_trimmings_still_prints() {
        // Every one of these is nullable, and a search is the worst place to
        // panic — the whole point is to find something you cannot name yet.
        let f = found(Kind::Recording, &serde_json::json!({ "id": 1 }));
        assert_eq!(f.id, "1");
        assert_eq!(f.name, "");
        assert_eq!(f.about, "");
    }

    #[test]
    fn dates_are_shown_only_as_far_as_they_are_known() {
        assert_eq!(
            lifespan(&serde_json::json!({"born": 1923, "died": 1977})),
            "1923–1977"
        );
        assert_eq!(lifespan(&serde_json::json!({"born": 1935})), "b1935");
        // A death with no birth reads as nonsense on its own.
        assert_eq!(lifespan(&serde_json::json!({"died": 1977})), "");
        assert_eq!(lifespan(&serde_json::json!({})), "");
    }

    #[test]
    fn a_character_is_shown_with_its_opera() {
        let f = found(
            Kind::Character,
            &serde_json::json!({"id": 910, "name": "Baron Scarpia", "opera": {"title": "Tosca"}}),
        );
        assert_eq!(
            (f.id.as_str(), f.name.as_str(), f.about.as_str()),
            ("910", "Baron Scarpia", "Tosca")
        );
    }

    #[test]
    fn the_notes_live_beside_the_api() {
        assert_eq!(
            notes_url("https://findopera.com/api/graphql", "10655"),
            "https://findopera.com/recording/10655.txt"
        );
        assert_eq!(
            notes_url("http://localhost:3333/api/graphql", "75"),
            "http://localhost:3333/recording/75.txt"
        );
        // Derived from the same place as the schema, so the two cannot end up
        // pointing at different servers.
        assert!(
            schema_url("https://findopera.com/api/graphql").starts_with("https://findopera.com/")
        );
    }

    #[test]
    fn the_utf8_filename_wins_over_the_ascii_one() {
        // The same header carries both: an ASCII `filename` for old clients and
        // a percent-encoded `filename*` with the accents intact. Taking the
        // first would silently strip the diacritics out of every name.
        let header = "inline; filename=\"Sosarme-Angioloni [findopera-10655].txt\"; \
                      filename*=UTF-8''Sosarme%2C%20Re%20di%20Media-Angioloni%20%5Bfindopera-10655%5D.txt";
        assert_eq!(
            disposition_filename(header).as_deref(),
            Some("Sosarme, Re di Media-Angioloni [findopera-10655].txt")
        );
    }

    #[test]
    fn an_ascii_only_header_still_gives_a_name() {
        let header = "inline; filename=\"findopera-10655.txt\"";
        assert_eq!(
            disposition_filename(header).as_deref(),
            Some("findopera-10655.txt")
        );
    }

    #[test]
    fn a_header_with_no_filename_gives_nothing() {
        assert_eq!(disposition_filename("inline"), None);
        // Rather than an empty name, which would become the directory itself.
        assert_eq!(disposition_filename("inline; filename=\"\""), None);
    }

    #[test]
    fn percent_decoding_survives_multibyte_characters() {
        assert_eq!(percent_decode("Op%C3%A9ra").as_deref(), Some("Opéra"));
        assert_eq!(percent_decode("plain").as_deref(), Some("plain"));
        // Truncated escapes are refused rather than guessed at.
        assert_eq!(percent_decode("bad%C"), None);
        assert_eq!(percent_decode("bad%ZZ"), None);
    }
}

/// What a lookup can look for.
///
/// Everything but a recording is a plain text search, which is why they share
/// one shape here; a recording is found by several things at once, so it
/// carries the rest.
#[derive(Debug, Default)]
pub struct Criteria {
    pub text: String,
    pub singers: Vec<String>,
    pub conductor: Option<String>,
    pub year: Option<i64>,
    pub first: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Recording,
    Opera,
    Singer,
    Conductor,
    Composer,
    Character,
}

impl Kind {
    /// The operation in `schema/search.graphql`, and the field it returns.
    fn operation(self) -> (&'static str, &'static str) {
        match self {
            Kind::Recording => ("SearchRecordings", "filterRecordings"),
            Kind::Opera => ("SearchOperas", "searchOperas"),
            Kind::Singer => ("SearchSingers", "searchSingers"),
            Kind::Conductor => ("SearchConductors", "searchConductors"),
            Kind::Composer => ("SearchComposers", "searchComposers"),
            Kind::Character => ("SearchCharacters", "searchCharacters"),
        }
    }
}

/// One result, already reduced to what a line of output needs.
///
/// Flattened here rather than in the caller because the shape differs per
/// kind and the printing does not: an id, a name, and whatever tells this one
/// apart from the next one with the same name.
#[derive(Debug, PartialEq, Eq)]
pub struct Found {
    pub id: String,
    pub name: String,
    pub about: String,
}

/// The lookup queries, checked in and validated against the schema by codegen.
const LOOKUPS: &str = include_str!("../schema/search.graphql");

fn text(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

/// A person's dates, as far as they are known.
fn lifespan(value: &serde_json::Value) -> String {
    match (value["born"].as_i64(), value["died"].as_i64()) {
        (Some(b), Some(d)) => format!("{b}–{d}"),
        (Some(b), None) => format!("b{b}"),
        // A death with no birth is not worth printing on its own.
        _ => String::new(),
    }
}

impl Client {
    /// Look something up by name.
    pub fn search(&self, kind: Kind, criteria: &Criteria) -> Result<Vec<Found>, ApiError> {
        let (operation, field) = kind.operation();
        let first = criteria.first.max(1);

        let variables = if kind == Kind::Recording {
            let mut filter = serde_json::Map::new();
            if !criteria.text.trim().is_empty() {
                filter.insert("operaTitleSearch".into(), criteria.text.trim().into());
            }
            if !criteria.singers.is_empty() {
                filter.insert("singerNameSearches".into(), criteria.singers.clone().into());
            }
            if let Some(conductor) = &criteria.conductor {
                filter.insert("conductorNameSearch".into(), conductor.clone().into());
            }
            if let Some(year) = criteria.year {
                filter.insert("approximateYear".into(), year.into());
            }
            serde_json::json!({ "filter": filter, "first": first })
        } else {
            serde_json::json!({ "query": criteria.text, "first": first })
        };

        let payload = self.query_named(LOOKUPS, operation, variables)?;
        let list = payload["data"][field]
            .as_array()
            .ok_or_else(|| ApiError(format!("the API returned no {field}")))?;

        Ok(list.iter().map(|v| found(kind, v)).collect())
    }
}

fn found(kind: Kind, v: &serde_json::Value) -> Found {
    let id = match v["id"].as_i64() {
        Some(n) => n.to_string(),
        None => text(&v["id"]),
    };
    match kind {
        Kind::Recording => {
            let singers: Vec<String> = v["notedSingers"]
                .as_array()
                .map(|s| s.iter().map(|s| text(&s["lastName"])).collect())
                .unwrap_or_default();
            let mut about = Vec::new();
            if let Some(year) = v["year"].as_i64() {
                about.push(year.to_string());
            }
            let conductor = text(&v["conductor"]["lastName"]);
            if !conductor.is_empty() {
                about.push(conductor);
            }
            if !singers.is_empty() {
                about.push(singers.join(", "));
            }
            Found {
                id,
                name: text(&v["opera"]["title"]),
                about: about.join("  "),
            }
        }
        Kind::Opera => Found {
            id,
            name: text(&v["title"]),
            about: text(&v["composer"]["lastName"]),
        },
        Kind::Character => Found {
            id,
            name: text(&v["name"]),
            about: text(&v["opera"]["title"]),
        },
        Kind::Singer | Kind::Conductor | Kind::Composer => Found {
            id,
            name: text(&v["fullName"]),
            about: lifespan(v),
        },
    }
}
