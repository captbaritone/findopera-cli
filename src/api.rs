//! Fetching recordings from the FindOpera GraphQL API.

use crate::model::{Recording, QUERY};
use std::collections::BTreeMap;

pub const DEFAULT_ENDPOINT: &str = "https://findopera.com/api/graphql";

/// Ids per request. A scan of a real library turns up thousands, and asking
/// for them all in one query is a good way to be told no.
const BATCH: usize = 100;

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

    let mut response = ureq::post(endpoint)
        .send_json(&body)
        .map_err(|e| ApiError(format!("cannot reach {endpoint}: {e}")))?;

    let payload: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| ApiError(format!("the API returned something unreadable: {e}")))?;

    // A GraphQL response can carry both data and errors. Any error at all is
    // worth reporting: `@semanticNonNull` means a null in an annotated
    // position is explained by exactly one of these.
    if let Some(errors) = payload.get("errors").and_then(|e| e.as_array()) {
        let messages: Vec<&str> = errors
            .iter()
            .filter_map(|e| e["message"].as_str())
            .collect();
        return Err(ApiError(format!(
            "the API reported: {}",
            messages.join("; ")
        )));
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
