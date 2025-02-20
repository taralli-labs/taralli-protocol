use futures_util::StreamExt;
use std::marker::PhantomData;
use std::time::Duration;
use taralli_primitives::abi::universal_porchetta::UniversalPorchetta::{
    Bid, Resolve, UniversalPorchettaInstance,
};
use taralli_primitives::alloy::{
    network::Network,
    primitives::{Address, B256},
    providers::Provider,
    transports::Transport,
};

use crate::error::{ClientError, Result};

pub struct OfferTracker<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    rpc_provider: P,
    market_address: Address,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> OfferTracker<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new(rpc_provider: P, market_address: Address) -> Self {
        Self {
            rpc_provider,
            market_address,
            phantom_data: PhantomData,
        }
    }

    /// Start tracking auction events for an offer
    pub async fn start_auction_tracking(
        &self,
        offer_id: B256,
        timeout: Duration,
    ) -> Result<Option<Bid>> {
        let market_contract =
            UniversalPorchettaInstance::new(self.market_address, self.rpc_provider.clone());

        let bid_filter = market_contract.Bid_filter().topic2(offer_id);

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

        match result {
            Ok(event) => Ok(event),
            Err(_) => {
                tracing::info!("Auction watching timed out");
                Ok(None)
            }
        }
    }

    /// Start tracking resolution events for a request
    pub async fn start_resolution_tracking(
        &self,
        offer_id: B256,
        timeout: Duration,
    ) -> Result<Option<Resolve>> {
        let market_contract =
            UniversalPorchettaInstance::new(self.market_address, self.rpc_provider.clone());

        let resolve_filter = market_contract.Resolve_filter().topic2(offer_id);

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

        match result {
            Ok(event) => Ok(event),
            Err(_) => {
                tracing::info!("Resolution watching timed out");
                Ok(None)
            }
        }
    }
}
