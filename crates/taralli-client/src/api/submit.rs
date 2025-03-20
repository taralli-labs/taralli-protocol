use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart::{Form, Part},
    Client,
};
use serde_json::json;
use taralli_primitives::{env::Environment, intents::ComputeIntent};
use url::Url;

use crate::api::compression::compress_system;
use crate::error::{ClientError, Result};

/// Submit compute intents to the protocol server
pub struct SubmitApiClient {
    _api_key: String,
    client: Client,
    server_url: Url,
}

impl SubmitApiClient {
    pub fn new(server_url: Url) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Content-Encoding", HeaderValue::from_static("br"));

        let mut api_key = String::new();
        if Environment::from_env_var() == Environment::Production {
            api_key = std::env::var("API_KEY").expect("API_KEY env variable is not set");
        }

        Self {
            _api_key: api_key,
            client: Client::builder()
                .default_headers(headers)
                .build()
                .expect("Failed to build reqwest client"),
            server_url,
        }
    }

    /// Returns Multipart intent Form with two parts: `System` as a `application/octet-stream` and remaining
    /// fields as `application/json`.
    fn build_multipart<I: ComputeIntent>(&self, intent: I) -> Result<Form> {
        let partial_intent = json!({
            "system_id": intent.system_id(),
            "proof_commitment": intent.proof_commitment(),
            "signature": intent.signature(),
        });

        let partial_intent_string = serde_json::to_string(&partial_intent)
            .map_err(|e| ClientError::IntentSubmissionFailed(e.to_string()))?;
        let partial_request_part = Part::text(partial_intent_string);

        let compressed = compress_system(intent.system())?;
        let compressed_part = Part::bytes(compressed);

        let form = Form::new()
            .part("partial_request", partial_request_part)
            .part("proving_system_information", compressed_part);

        Ok(form)
    }

    pub async fn submit_intent<I: ComputeIntent>(&self, intent: I) -> Result<reqwest::Response> {
        let url = self
            .server_url
            .join("/submit")
            .map_err(|e| ClientError::ServerUrlParsingError(e.to_string()))?;

        let payload = self.build_multipart(intent)?;

        let response = self
            .client
            .post(url)
            .multipart(payload)
            .send()
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;
        Ok(response)
    }
}
