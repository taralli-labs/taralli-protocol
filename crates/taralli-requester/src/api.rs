use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use taralli_primitives::{systems::ProvingSystemParams, Request};
use url::Url;

use crate::error::{RequesterError, Result};

pub struct RequesterApi {
    client: Client,
    server_url: Url,
}

impl RequesterApi {
    pub fn new(server_url: Url) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        // Todo: enum for env and usable in all crates
        let env = match std::env::var("ENV") {
            Ok(env) => env,
            Err(_) => "DEVELOPMENT".to_string(),
        };

        if env == "PRODUCTION" {
            if let Ok(api_key) = std::env::var("API_KEY") {
                headers.insert("x-api-key", HeaderValue::from_str(&api_key).unwrap());
            } else {
                return Err(RequesterError::ApiKeyError("API_KEY not found".to_string()));
            }
        }
        Ok(Self {
            client: Client::builder()
                .default_headers(headers)
                .build()
                .map_err(|e| RequesterError::BuilderError(e.to_string()))?,
            server_url,
        })
    }

    pub async fn submit_request(
        &self,
        request: Request<ProvingSystemParams>,
    ) -> Result<reqwest::Response> {
        let url = self
            .server_url
            .join("/submit")
            .map_err(|e| RequesterError::ServerUrlParsingError(e.to_string()))?;

        let response = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|e| RequesterError::ServerRequestError(e.to_string()))?;
        Ok(response)
    }
}
