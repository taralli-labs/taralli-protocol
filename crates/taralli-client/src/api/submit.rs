use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use taralli_primitives::{env::Environment, intents::ComputeIntent};
use url::Url;

use crate::api::compression::compress_intent;
use crate::error::{ClientError, Result};

/// Submit compute intents to the server
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

    pub async fn submit_intent<I: ComputeIntent>(&self, intent: I) -> Result<reqwest::Response> {
        let endpoint = format!("/submit/{}", intent.type_string());

        let url = self
            .server_url
            .join(&endpoint)
            .map_err(|e| ClientError::ServerUrlParsingError(e.to_string()))?;

        let compressed_payload = compress_intent(intent)?;

        let response = self
            .client
            .post(url)
            .body(compressed_payload)
            .send()
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;
        Ok(response)
    }
}
