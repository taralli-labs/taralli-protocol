use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use taralli_primitives::common::types::Environment;
use taralli_primitives::{systems::ProvingSystemParams, Request};
use url::Url;

use crate::error::{RequesterError, Result};

pub struct RequesterApi {
    client: Client,
    server_url: Url,
}

impl RequesterApi {
    pub fn new(server_url: Url) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        if Environment::from_env_var() == Environment::Production {
            let api_key = std::env::var("API_KEY").expect("API_KEY env variable is not set");
            headers.insert(
                "x-api-key",
                HeaderValue::from_str(&api_key).expect("API_KEY is invalid as a header"),
            );
        }

        Self {
            client: Client::builder()
                .default_headers(headers)
                .build()
                .expect("Failed to build reqwest client"),
            server_url,
        }
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
