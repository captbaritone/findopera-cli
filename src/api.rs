//! Fetching recordings from the FindOpera GraphQL API.

use crate::model::{Recording, QUERY};
use std::collections::BTreeMap;

pub const DEFAULT_ENDPOINT: &str = "https://findopera.com/api/graphql";

/// Ids per request. A scan of a real library turns up thousands, and asking
/// for them all in one query is a good way to be told no.
const BATCH: usize = 100;

/// What this program calls itself when it asks findopera.com for something.
///
/// Taken from the package version, so it cannot fall behind a release, and
/// sent on every request — someone reading their own logs should be able to
/// tell this apart from a browser, and tell one version of it from another.
pub const USER_AGENT: &str = concat!("findopera-cli/", env!("CARGO_PKG_VERSION"));

/// Start a request to the API.
///
/// Every request goes through here, so there is one place that decides what
/// findopera.com is told about the caller, and no way to add a second request
/// that quietly says nothing.
fn request(endpoint: &str) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
    ureq::post(endpoint).header("User-Agent", USER_AGENT)
}

#[derive(Debug)]
pub struct ApiError(String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Fetch recordings by id.
///
/// The API returns a list positionally aligned with the ids it was given, with
/// `null` for any it does not know, so the returned map simply omits those and
/// the caller reports them as not found.
pub fn recordings(endpoint: &str, ids: &[String]) -> Result<BTreeMap<String, Recording>, ApiError> {
    let mut out = BTreeMap::new();
    for chunk in ids.chunks(BATCH) {
        out.extend(fetch_batch(endpoint, chunk)?);
    }
    Ok(out)
}

fn fetch_batch(endpoint: &str, ids: &[String]) -> Result<BTreeMap<String, Recording>, ApiError> {
    let body = serde_json::json!({ "query": QUERY, "variables": { "ids": ids } });

    let mut response = request(endpoint)
        .send_json(&body)
        .map_err(|e| ApiError(format!("cannot reach {endpoint}: {e}")))?;

    let payload: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| ApiError(format!("the API returned something unreadable: {e}")))?;

    // A GraphQL response can carry both data and errors, and an error here is
    // always fatal: `@semanticNonNull` means a null in an annotated position
    // is explained by exactly one of these, so data alongside an error cannot
    // be trusted to be whole.
    //
    // It is also how the server talks to whoever is running this — every
    // request says which version it is, and this is the channel for the answer
    // "that one is too old". So the words come through as they were written,
    // one to a line, rather than being summarised into a single line that a
    // long explanation would not survive.
    if let Some(errors) = payload.get("errors").and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            let mut said = String::from("the server refused the request:");
            for error in errors {
                let message = error["message"].as_str().unwrap_or("(no message given)");
                // A code is where a server puts something a program can act
                // on, so it is worth showing beside the words meant for a
                // person.
                match error["extensions"]["code"].as_str() {
                    Some(code) => said.push_str(&format!("\n    [{code}] {message}")),
                    None => said.push_str(&format!("\n    {message}")),
                }
            }
            return Err(ApiError(said));
        }
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
