//! Thin GraphQL client for the FindOpera API.

use crate::model::{Recording, QUERY};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const DEFAULT_ENDPOINT: &str = "https://findopera.com/api/graphql";

/// Recordings are fetched in batches; the API takes a list of ids and returns a
/// positionally-aligned list with `null` for ids it doesn't know.
const BATCH_SIZE: usize = 100;

#[derive(Debug)]
pub struct ApiError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Deserialize)]
struct GraphQlResponse {
    data: Option<ResponseData>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Deserialize)]
struct ResponseData {
    #[serde(rename = "getRecordingByIds")]
    get_recording_by_ids: Vec<Option<Recording>>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
}

pub struct Client {
    endpoint: String,
    timeout_secs: u64,
}

impl Client {
    pub fn new(endpoint: String, timeout_secs: u64) -> Self {
        Client {
            endpoint,
            timeout_secs,
        }
    }

    /// Fetch recordings by id. The returned map omits ids the API doesn't know,
    /// leaving the caller to report them as not-found.
    pub fn recordings(&self, ids: &[String]) -> Result<BTreeMap<String, Recording>, ApiError> {
        let mut out = BTreeMap::new();
        for chunk in ids.chunks(BATCH_SIZE) {
            let body = serde_json::json!({
                "query": QUERY,
                "variables": { "ids": chunk },
            });
            let response = ureq::post(&self.endpoint)
                .timeout(std::time::Duration::from_secs(self.timeout_secs))
                .set("content-type", "application/json")
                .set(
                    "user-agent",
                    concat!("findopera/", env!("CARGO_PKG_VERSION")),
                )
                .send_json(body);

            let parsed: GraphQlResponse = match response {
                Ok(r) => r.into_json().map_err(|e| ApiError {
                    code: "api_bad_response",
                    message: format!("could not parse the API response: {e}"),
                    retryable: false,
                })?,
                Err(ureq::Error::Status(status, _)) => {
                    return Err(ApiError {
                        code: "api_error",
                        // 5xx and 429 are worth another attempt; 4xx are not.
                        message: format!("the FindOpera API returned HTTP {status}"),
                        retryable: status >= 500 || status == 429,
                    });
                }
                Err(e) => {
                    return Err(ApiError {
                        code: "network_error",
                        message: format!("could not reach {}: {e}", self.endpoint),
                        retryable: true,
                    })
                }
            };

            if let Some(errors) = parsed.errors {
                let joined: Vec<String> = errors.into_iter().map(|e| e.message).collect();
                if !joined.is_empty() {
                    return Err(ApiError {
                        code: "api_error",
                        message: format!("the FindOpera API reported: {}", joined.join("; ")),
                        retryable: false,
                    });
                }
            }
            let data = parsed.data.ok_or_else(|| ApiError {
                code: "api_bad_response",
                message: "the FindOpera API returned no data".to_string(),
                retryable: false,
            })?;

            for (id, rec) in chunk.iter().zip(data.get_recording_by_ids) {
                if let Some(rec) = rec {
                    out.insert(id.clone(), rec);
                }
            }
        }
        Ok(out)
    }
}
