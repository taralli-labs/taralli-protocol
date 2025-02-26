use alloy::primitives::Address;
use async_trait::async_trait;
use taralli_primitives::{
    intents::offer::ComputeOffer,
    systems::{SystemId, SystemParams},
};
use url::Url;

use crate::{
    api::ApiClient,
    error::{ClientError, Result},
};

/// core searcher trait
#[async_trait]
pub trait IntentSearcher {
    type Intent;
    async fn search(&self) -> Result<Self::Intent>;
}

pub struct ComputeOfferSearcher {
    api_client: ApiClient,
    system_id: SystemId,
    _market_address: Address,
}

impl ComputeOfferSearcher {
    pub fn new(server_url: Url, system_id: SystemId, market_address: Address) -> Self {
        Self {
            api_client: ApiClient::new(server_url),
            system_id,
            _market_address: market_address,
        }
    }
}

#[async_trait]
impl IntentSearcher for ComputeOfferSearcher {
    type Intent = ComputeOffer<SystemParams>;

    async fn search(&self) -> Result<Self::Intent> {
        // Query the server for active offers matching system_id
        let response = self.api_client.query_market_offers(self.system_id).await?;

        // Parse response into JSON and extract intents array
        let json = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;

        let offers = json
            .get("intents")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ClientError::ServerRequestError("Invalid response format".into()))?;

        // Convert stored intents into ComputeOffers
        let offers = offers
            .iter()
            .map(|stored| serde_json::from_value::<ComputeOffer<SystemParams>>(stored.clone()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                ClientError::ServerRequestError(format!("Failed to parse offers: {}", e))
            })?;

        let first_offer = offers
            .first()
            .ok_or_else(|| ClientError::ServerRequestError("No offers available".into()))?;

        Ok(first_offer.clone())
    }
}
