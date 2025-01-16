use futures_util::StreamExt;
use std::marker::PhantomData;
use std::time::Duration;
use taralli_primitives::abi::universal_bombetta::UniversalBombetta::{
    Bid, Resolve, UniversalBombettaInstance,
};
use taralli_primitives::alloy::{
    network::Network,
    primitives::{Address, B256},
    providers::Provider,
    transports::Transport,
};

use crate::error::{RequesterError, Result};

pub struct RequestTracker<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    rpc_provider: P,
    market_address: Address,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> RequestTracker<T, P, N>
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

    /// track the proof request from auction to resolution
    pub async fn track_request(
        &self,
        request_id: B256,
        auction_timeout: Duration,
        resolution_timeout: Duration,
    ) -> Result<()> {
        // track the auction
        // if auction result doesn't show up by end ts of auction, stop
        // if a successful bid event for the given request ID is seen, proceed
        tracing::info!("watching auction");
        let _auction_event = self
            .watch_auction(&request_id, auction_timeout)
            .await
            .map_err(|e| RequesterError::TrackRequestError(e.to_string()))?
            .ok_or(RequesterError::AuctionTimeoutError())?;

        // track the resolution of the request until provingDeadline
        // if by the provingDeadline no resolution event is seen, send the slash txs
        tracing::info!("watching resolution");
        let _resolution_event = self
            .watch_resolution(&request_id, resolution_timeout)
            .await?;

        // if a resolution event is seen check if it's a success or a slash.

        Ok(())
    }

    pub async fn watch_auction(
        &self,
        request_id: &B256,
        timeout: Duration,
    ) -> Result<Option<Bid>> {
        // Create an instance of the UniversalBombetta contract
        let market_contract =
            UniversalBombettaInstance::new(self.market_address, self.rpc_provider.clone());

        // Set up the event filter for Bid event
        let bid_filter = market_contract
            .Bid_filter()
            // Filter by the specific requestId
            .topic2(*request_id);

        // Watch for Bid events
        let event_poller = bid_filter
            .watch()
            .await
            .map_err(|e| RequesterError::TrackRequestError(e.to_string()))?;
        // Convert the EventPoller into a stream
        let mut bid_stream = event_poller.into_stream();

        // Use tokio's timeout mechanism to stop listening after the specified timeout
        let result = tokio::time::timeout(timeout, async {
            while let Some(log_result) = bid_stream.next().await {
                match log_result {
                    Ok((bid_event, _)) => {
                        // We've found a matching Bid event
                        return Some(bid_event);
                    }
                    Err(e) => {
                        // Log the error but continue watching
                        tracing::error!("Error processing log: {:?}", e);
                    }
                }
            }
            None
        })
        .await;

        match result {
            Ok(Some(bid_event)) => {
                tracing::info!("Bid event found: {:?}", bid_event);
                Ok(Some(bid_event))
            }
            Ok(None) => {
                tracing::info!(
                    "No matching bid event found for request ID: {:?}",
                    request_id
                );
                Ok(None)
            }
            Err(_) => {
                tracing::info!("Auction watching timed out.");
                Ok(None)
            }
        }
    }

    pub async fn watch_resolution(
        &self,
        request_id: &B256,
        timeout: Duration,
    ) -> Result<Option<Resolve>> {
        // Implementation to watch the resolution for a specific request
        let market_contract =
            UniversalBombettaInstance::new(self.market_address, self.rpc_provider.clone());

        // Set up the event filter for Bid event
        let resolve_filter = market_contract
            .Resolve_filter()
            // Filter by the specific requestId
            .topic2(*request_id);

        // Watch for Bid events
        let event_poller = resolve_filter
            .watch()
            .await
            .map_err(|e| RequesterError::TrackRequestError(e.to_string()))?;
        // Convert the EventPoller into a stream
        let mut resolve_stream = event_poller.into_stream();

        // Use tokio's timeout mechanism to stop listening after the specified timeout
        let result = tokio::time::timeout(timeout, async {
            while let Some(log_result) = resolve_stream.next().await {
                match log_result {
                    Ok((resolve_event, _)) => {
                        // We've found a matching Bid event
                        return Some(resolve_event);
                    }
                    Err(e) => {
                        // Log the error but continue watching
                        tracing::error!("Error processing log: {:?}", e);
                    }
                }
            }
            None
        })
        .await;

        match result {
            Ok(Some(resolve_event)) => {
                tracing::info!("Resolve event found: {:?}", resolve_event);
                Ok(Some(resolve_event))
            }
            Ok(None) => {
                tracing::info!(
                    "No matching resolve event found for request ID: {:?}",
                    request_id
                );
                Ok(None)
            }
            Err(_) => {
                tracing::info!("Resolution watching timed out.");
                Ok(None)
            }
        }
    }
}
