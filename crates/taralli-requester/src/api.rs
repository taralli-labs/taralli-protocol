use reqwest::Client;
use taralli_primitives::{taralli_systems::id::ProvingSystemParams, Request};
use url::Url;

use crate::error::{RequesterError, RequesterResult};

pub struct RequesterApi {
    client: Client,
    server_url: Url,
}

impl RequesterApi {
    pub fn new(server_url: Url) -> Self {
        Self {
            client: Client::new(),
            server_url,
        }
    }

    pub async fn submit_request(
        &self,
        request: Request<ProvingSystemParams>,
    ) -> RequesterResult<reqwest::Response> {
        let submit_endpoint = self
            .server_url
            .join("/submit")
            .map_err(|e| RequesterError::ServerUrlParsingError(e.to_string()))?;
        let response = self
            .client
            .post(submit_endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| RequesterError::ServerRequestError(e.to_string()))?;
        Ok(response)
    }
}
