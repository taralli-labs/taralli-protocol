use std::time::Duration;

use alloy::consensus::BlockHeader;
use alloy::eips::BlockId;
use alloy::eips::BlockNumberOrTag::Latest;
use alloy::network::BlockResponse;
use alloy::primitives::Address;
use alloy::rpc::types::BlockTransactionsKind;
use alloy::signers::Signer;
use alloy::{network::Network, providers::Provider, transports::Transport};
use taralli_primitives::intents::ComputeIntent;
use taralli_primitives::systems::SystemId;
use taralli_primitives::validation::offer::OfferValidationConfig;
use url::Url;

use crate::analyzer::{offer::ComputeOfferAnalyzer, IntentAnalyzer};
use crate::api::submit::SubmitApiClient;
use crate::bidder::offer::{ComputeOfferBidParams, ComputeOfferBidder};
use crate::bidder::IntentBidder;
use crate::error::{ClientError, Result};
use crate::searcher::{offer::ComputeOfferSearcher, IntentSearcher};
use crate::tracker::{offer::ComputeOfferTracker, IntentResolveTracker};

use crate::client::BaseClient;

/// Client that queries the server for a given system ID to search for a compute offering
/// they want to bid upon. Once an offer has been found and analyzed, it is bid upon thereafter
/// being tracked until resolution of the offered compute workload.
pub struct RequesterSearchingClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub base: BaseClient<T, P, N, S>,
    pub api: SubmitApiClient,
    pub searcher: ComputeOfferSearcher,
    pub analyzer: ComputeOfferAnalyzer<T, P, N>,
    pub bidder: ComputeOfferBidder<T, P, N>,
    pub tracker: ComputeOfferTracker<T, P, N>,
}

impl<T, P, N, S> RequesterSearchingClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub fn new(
        server_url: Url,
        rpc_provider: P,
        signer: S,
        market_address: Address,
        system_id: SystemId,
        validation_config: OfferValidationConfig,
    ) -> Self {
        Self {
            base: BaseClient::new(rpc_provider.clone(), signer.clone(), market_address),
            api: SubmitApiClient::new(server_url.clone()),
            searcher: ComputeOfferSearcher::new(server_url, system_id, market_address),
            analyzer: ComputeOfferAnalyzer::new(
                rpc_provider.clone(),
                system_id,
                market_address,
                validation_config,
            ),
            bidder: ComputeOfferBidder::new(rpc_provider.clone(), market_address),
            tracker: ComputeOfferTracker::new(rpc_provider, market_address),
        }
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!("searcher execution started");
        // search for a compute offer
        let offer = self.searcher.search().await?;
        // compute id
        let offer_id = offer.compute_id();

        tracing::info!(
            "searching execution finished, analyzing offer: {:?}",
            offer.proof_offer
        );

        // Fetch latest block timestamp
        // TODO: remove this call from the request processing work flow, instead passing it in as input from another external process
        let current_ts = self
            .base
            .rpc_provider
            .get_block(BlockId::Number(Latest), BlockTransactionsKind::Hashes)
            .await
            .map_err(|e| ClientError::RpcRequestError(e.to_string()))?
            .ok_or_else(|| ClientError::RpcRequestError("Block header not found".to_string()))?
            .header()
            .timestamp();

        // compute resolve deadline timestamp
        let resolve_deadline_ts =
            offer.proof_offer.endAuctionTimestamp + offer.proof_offer.provingTime as u64;

        // analyze the validity and profitability of the offer
        self.analyzer
            .analyze(current_ts, &offer)
            .await
            .map_err(|e| ClientError::IntentAnalysisError(e.to_string()))?;
        tracing::info!("analysis done, bidding");

        // Submit a bid for the offer
        self.bidder
            .submit_bid(
                current_ts,
                offer_id,
                ComputeOfferBidParams {},
                offer.proof_offer.clone(),
                offer.signature,
            )
            .await
            .map_err(|e| ClientError::TransactionFailure(format!("bid txs failed: {}", e)))?;

        tracing::info!("bid transaction submitted successfully, tracking resolution of the offer");

        // setup tracking
        self.tracker
            .track_resolve(offer_id, Duration::from_secs(resolve_deadline_ts))
            .await?;

        tracing::info!("Compute offer resolved");
        Ok(())
    }
}
