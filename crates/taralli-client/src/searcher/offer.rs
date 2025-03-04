use alloy::primitives::Address;
use async_trait::async_trait;
use taralli_primitives::{
    intents::offer::ComputeOffer,
    systems::{SystemId, SystemParams},
};
use url::Url;

use crate::api::query::QueryApiClient;
use crate::error::{ClientError, Result};

use super::IntentSearcher;

pub struct ComputeOfferSearcher {
    api_client: QueryApiClient,
    system_id: SystemId,
    _market_address: Address,
}

impl ComputeOfferSearcher {
    pub fn new(server_url: Url, system_id: SystemId, market_address: Address) -> Self {
        Self {
            api_client: QueryApiClient::new(server_url),
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
        let offers = self.api_client.query_market_offers(self.system_id).await?;

        let first_offer = offers
            .first()
            .ok_or_else(|| ClientError::ServerRequestError("No offers available".into()))?;

        Ok(first_offer.clone())
    }
}
