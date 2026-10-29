use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;

use super::models::ScriptResponse;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::request::types::{HttpRequestInput, NameValue, RequestBody};
use crate::request::validate::validate;

/// Shorter than the default request timeout, because the script waits for it.
const SEND_TIMEOUT_MS: u64 = 10_000;

/// Lets `pm.sendRequest` reach the network through the app's HTTP client, with the same
/// validation as a request sent from a card (URL scheme, header injection, body size).
#[derive(Clone)]
pub struct Network {
    client: HttpClient,
    runtime: Handle,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SendInput {
    method: String,
    url: String,
    headers: Vec<NameValue>,
    body: Option<String>,
}

#[derive(Serialize)]
struct SendOutput {
    response: Option<ScriptResponse>,
    error: Option<String>,
}

impl Network {
    pub const fn new(client: HttpClient, runtime: Handle) -> Self {
        Self { client, runtime }
    }

    /// Takes the request as JSON from the script and returns the response (or error) as JSON.
    /// Blocking: the script thread waits for the answer.
    pub fn send(&self, request_json: &str) -> String {
        match self.exchange(request_json) {
            Ok(response) => serde_json::to_string(&SendOutput {
                response: Some(response),
                error: None,
            })
            .unwrap_or_else(|_| failure_json("The response could not be read")),
            Err(error) => failure_json(&error.to_string()),
        }
    }

    fn exchange(&self, request_json: &str) -> Result<ScriptResponse, AppError> {
        let input: SendInput = serde_json::from_str(request_json)
            .map_err(|_| AppError::invalid("pm.sendRequest received an invalid request"))?;
        let validated = validate(HttpRequestInput {
            method: input.method,
            url: input.url,
            params: Vec::new(),
            headers: input.headers,
            // Scripts send text only; they can never point the app at a local file.
            body: input.body.map(|text| RequestBody::Raw { text }),
            timeout_ms: Some(SEND_TIMEOUT_MS),
            collection_id: None,
            request_id: None,
            name: None,
            pre_request_script: None,
            test_script: None,
            auth: None,
            record_history: false,
            path_variables: Vec::new(),
            iteration_data: Vec::new(),
            iteration: Default::default(),
        })?;
        let response = self.runtime.block_on(self.client.send(validated))?;
        Ok(ScriptResponse {
            code: response.status,
            status: response.status_text,
            response_time: response.elapsed_ms,
            size: response.body_size,
            headers: response.headers,
            body: response.body_preview,
        })
    }
}

/// The reply `pm.sendRequest` gets when the request could not be made.
pub fn failure_json(message: &str) -> String {
    serde_json::json!({ "response": null, "error": message }).to_string()
}
