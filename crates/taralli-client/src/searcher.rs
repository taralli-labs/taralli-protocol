use alloy::primitives::Address;
use async_trait::async_trait;
use taralli_primitives::systems::SystemId;

use crate::error::Result;

/// core searcher trait
#[async_trait]
pub trait IntentSearcher {
    type Intent;
    async fn search(&self) -> Result<Vec<Self::Intent>>;
}

pub struct OfferSearcher {
    system_id: SystemId,
    market_address: Address,
}

impl OfferSearcher {
    pub fn new(system_id: SystemId, market_address: Address) -> Self {
        Self {
            system_id,
            market_address,
        }
    }
}

/*pub struct IntentSearcher {
    search_config: SearchConfig,
    searcher_type: SearcherType,
    phantom: PhantomData<(T, P, N)>,
}

enum SearcherType {
    RequestSearcher(BiddingConfig),
    OfferSearcher(AcceptanceConfig),
}

impl IntentSearcher {
    pub fn new_request_searcher(
        search_config: RequesterSearchConfig,
        bidding_config: BiddingConfig,
    ) -> Self {
        Self {
            search_config,
            searcher_type: SearcherType::RequestSearcher(bidding_config)
        }
    }

    pub fn new_provider_searcher(
        search_config: RequesterSearchConfig,
        bidding_config: BiddingConfig
    ) -> Self {
        Self {
            search_config,
            searcher_type: SearcherType::RequestSearcher(bidding_config)
        }
    }

    pub async fn search(&self) -> Result<IntentStream> {
        match &self.searcher_type {
            SearcherType::RequestSearcher(config) => self.search_requests(config).await,
            SearcherType::OfferSearcher(config) => self.search_offers(config).await,
        }
    }
}*/
