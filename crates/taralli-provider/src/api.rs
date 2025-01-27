use futures::{Stream, StreamExt, TryStreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde_json::from_str;
use std::pin::Pin;
use taralli_common::types::Environment;
use taralli_primitives::{systems::ProvingSystemParams, Request};
use url::Url;

use crate::{
    config::ApiConfig,
    error::{ProviderError, Result},
};

pub struct ProviderApi {
    client: Client,
    server_url: Url,
}

// type alias for SSE stream returned by the protocol server
pub type RequestStream = Pin<Box<dyn Stream<Item = Result<Request<ProvingSystemParams>>> + Send>>;

impl ProviderApi {
    pub fn new(config: ApiConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("text/event-stream"));
        if let Ok(api_key) = std::env::var("API_KEY") {
            headers.insert("x-api-key", HeaderValue::from_str(&api_key).unwrap());
        }

        if Environment::from_env_var() == Environment::Production {
            if let Ok(api_key) = std::env::var("API_KEY") {
                headers.insert("x-api-key", HeaderValue::from_str(&api_key).unwrap());
            } else {
                return Err(ProviderError::ApiKeyError("API_KEY not found".to_string()));
            }
        }

        Ok(Self {
            client: Client::builder()
                .default_headers(headers)
                .build()
                .map_err(|e| ProviderError::BuilderError(e.to_string()))?,
            server_url: config.server_url,
        })
    }

    pub async fn subscribe_to_markets(&self) -> Result<RequestStream> {
        // Attempt to join the URL, log error if it fails
        let url = self
            .server_url
            .join("/subscribe")
            .map_err(|e| ProviderError::ServerSubscriptionError(e.to_string()))?;

        // Send the GET request
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| ProviderError::ServerSubscriptionError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::ServerSubscriptionError(format!(
                "Failed to connect to /subscribe: {}",
                response.status()
            )));
        }

        // Convert the response stream into a stream of SSE events
        let stream = response
            .bytes_stream()
            .map_err(|e| ProviderError::RequestParsingError(e.to_string()));

        // Attempt to create the EventSource, log error if it fails
        Ok(Box::pin(stream.filter_map(|event| async move {
            match event {
                Ok(bytes) => {
                    let message = String::from_utf8(bytes.to_vec()).ok()?;
                    match from_str::<Request<ProvingSystemParams>>(&message) {
                        Ok(proof_request) => Some(Ok(proof_request)),
                        Err(e) => Some(Err(ProviderError::RequestParsingError(format!(
                            "Failed to parse proof request: {}",
                            e
                        )))),
                    }
                }
                Err(e) => Some(Err(ProviderError::RequestParsingError(format!(
                    "Stream encountered an error: {}",
                    e
                )))),
            }
        })))
    }
}
