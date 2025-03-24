use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use taralli_primitives::compression_utils::db::StoredIntent;
use taralli_primitives::env::Environment;
use taralli_primitives::{
    intents::offer::ComputeOffer,
    systems::{SystemId, SystemParams},
};
use url::Url;

use crate::error::{ClientError, Result};

/// Query `ComputeOffers` stored within the protocol server's intent db
pub struct QueryApiClient {
    _api_key: String,
    client: Client,
    server_url: Url,
}

impl QueryApiClient {
    #[must_use]
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

    pub async fn query_market_offers(
        &self,
        system_id: SystemId,
    ) -> Result<Vec<ComputeOffer<SystemParams>>> {
        let url = self
            .server_url
            .join(&format!("/query/{}", system_id.as_str()))
            .map_err(|e| ClientError::ServerUrlParsingError(e.to_string()))?;

        tracing::info!("Querying market offers at URL: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;

        // Check if the response is successful
        if !response.status().is_success() {
            return Err(ClientError::ServerRequestError(format!(
                "Server returned error status: {}",
                response.status()
            )));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;

        if response_text.trim().is_empty() {
            tracing::warn!("Server returned empty response");
            return Ok(Vec::new());
        }

        // Parse response into JSON
        let json: serde_json::Value = serde_json::from_str(&response_text).map_err(|e| {
            tracing::error!("Failed to parse JSON response: {}", e);
            tracing::debug!("Response text: {}", response_text);
            ClientError::ServerRequestError(format!("Invalid JSON response: {e}"))
        })?;

        let offers = json.get("intents").ok_or_else(|| {
            tracing::error!("Response missing 'intents' field: {}", json);
            ClientError::ServerRequestError(
                "Invalid response format: missing 'intents' field".into(),
            )
        })?;

        let stored_intents: Vec<StoredIntent> =
            serde_json::from_value(offers.clone()).map_err(|e| {
                tracing::error!("Failed to parse stored intents: {}", e);
                ClientError::ServerRequestError(format!("Failed to parse stored intents: {e}"))
            })?;

        if stored_intents.is_empty() {
            tracing::info!("No offers available for system: {:?}", system_id);
            return Ok(Vec::new());
        }

        // Convert stored intents into ComputeOffers
        let offers = stored_intents
            .into_iter()
            .map(|stored| {
                let result = ComputeOffer::<SystemParams>::try_from(stored.clone());
                if let Err(ref e) = result {
                    tracing::error!("Failed to convert stored intent to offer: {}", e);
                }
                result
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| ClientError::ServerRequestError(format!("Failed to parse offers: {e}")))?;

        tracing::info!("Successfully parsed {} offers", offers.len());

        Ok(offers)
    }
}
