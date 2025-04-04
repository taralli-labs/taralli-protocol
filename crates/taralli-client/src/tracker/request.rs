use async_trait::async_trait;
use futures_util::StreamExt;
use std::marker::PhantomData;
use std::time::Duration;
use taralli_primitives::alloy::{
    network::Network,
    primitives::{Address, B256},
    providers::Provider,
    transports::Transport,
};
use taralli_primitives::{
    abi::universal_bombetta::UniversalBombetta::{self, UniversalBombettaInstance},
    intents::request::ComputeRequest,
    systems::SystemParams,
};

use crate::error::{ClientError, Result};

use super::{IntentAuctionTracker, IntentResolveTracker};

/// `ComputeRequest` tracker for both auctions and resolutons
pub struct ComputeRequestTracker<T, P, N> {
    rpc_provider: P,
    market_address: Address,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> ComputeRequestTracker<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    pub fn new(rpc_provider: P, market_address: Address) -> Self {
        Self {
            rpc_provider,
            market_address,
            phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P, N> IntentAuctionTracker for ComputeRequestTracker<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    type Intent = ComputeRequest<SystemParams>;
    type BidEvent = UniversalBombetta::Bid;

    /// Start tracking auction events for a request
    async fn track_auction(
        &self,
        intent_id: B256,
        timeout: Duration,
    ) -> Result<Option<Self::BidEvent>> {
        let market_contract =
            UniversalBombettaInstance::new(self.market_address, self.rpc_provider.clone());

        let bid_filter = market_contract.Bid_filter().topic2(intent_id);

        let event_poller = bid_filter
            .watch()
            .await
            .map_err(|e| ClientError::TrackIntentError(e.to_string()))?;

        let mut bid_stream = event_poller.into_stream();

        let result = tokio::time::timeout(timeout, async move {
            while let Some(log_result) = bid_stream.next().await {
                match log_result {
                    Ok((bid_event, _)) => {
                        tracing::info!("Bid event found: {:?}", bid_event);
                        return Some(bid_event);
                    }
                    Err(e) => {
                        tracing::error!("Error processing log: {:?}", e);
                    }
                }
            }
            None
        })
        .await;

        if let Ok(event) = result {
            Ok(event)
        } else {
            tracing::info!("Auction watching timed out");
            Ok(None)
        }
    }
}

#[async_trait]
impl<T, P, N> IntentResolveTracker for ComputeRequestTracker<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    type Intent = ComputeRequest<SystemParams>;
    type ResolveEvent = UniversalBombetta::Resolve;

    /// Start tracking auction events for a request
    async fn track_resolve(
        &self,
        intent_id: B256,
        timeout: Duration,
    ) -> Result<Option<Self::ResolveEvent>> {
        let market_contract =
            UniversalBombettaInstance::new(self.market_address, self.rpc_provider.clone());

        let resolve_filter = market_contract.Resolve_filter().topic2(intent_id);

        let event_poller = resolve_filter
            .watch()
            .await
            .map_err(|e| ClientError::TrackIntentError(e.to_string()))?;

        let mut resolve_stream = event_poller.into_stream();

        let result = tokio::time::timeout(timeout, async move {
            while let Some(log_result) = resolve_stream.next().await {
                match log_result {
                    Ok((resolve_event, _)) => {
                        tracing::info!("Resolve event found: {:?}", resolve_event);
                        return Some(resolve_event);
                    }
                    Err(e) => {
                        tracing::error!("Error processing log: {:?}", e);
                    }
                }
            }
            None
        })
        .await;

        if let Ok(event) = result {
            Ok(event)
        } else {
            tracing::info!("Auction watching timed out");
            Ok(None)
        }
    }
}
