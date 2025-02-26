use alloy::primitives::Address;
use async_trait::async_trait;
use std::marker::PhantomData;
use taralli_primitives::{
    alloy::{network::Network, providers::Provider, transports::Transport},
    intents::{offer::ComputeOffer, request::ComputeRequest, ComputeIntent},
    systems::SystemParams,
    validation::{offer::OfferValidationConfig, request::RequestValidationConfig, Validate},
};

use crate::error::Result;

/// core analyzer trait
#[async_trait]
pub trait IntentAnalyzer {
    type Intent;
    async fn analyze(&self, latest_ts: u64, intent: &Self::Intent) -> Result<()>;
}

// TODO: add in in economic logic for intents coming from server stream
// decide wether or not the inbound intent is `safe` and `profitable` to bid upon.
// uses the bidder to submit the bid with a given target price if all the checks pass
pub struct ComputeRequestAnalyzer<T, P, N, I>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    _rpc_provider: P,
    pub market_address: Address,
    pub validation_config: RequestValidationConfig,
    phantom_data: PhantomData<(T, N, I)>,
}

impl<T, P, N, I> ComputeRequestAnalyzer<T, P, N, I>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    pub fn new(
        rpc_provider: P,
        market_address: Address,
        validation_config: RequestValidationConfig,
    ) -> Self {
        Self {
            _rpc_provider: rpc_provider,
            market_address,
            validation_config,
            phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P, N, I> IntentAnalyzer for ComputeRequestAnalyzer<T, P, N, I>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
    I: ComputeIntent,
{
    type Intent = ComputeRequest<SystemParams>;

    async fn analyze(&self, latest_ts: u64, intent: &Self::Intent) -> Result<()> {
        // general correctness checks
        intent.validate(latest_ts, &self.market_address, &self.validation_config)?;

        //// TODO: economic checks

        Ok(())
    }
}

pub struct ComputeOfferAnalyzer<T, P, N, I>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    _rpc_provider: P,
    pub market_address: Address,
    pub validation_config: OfferValidationConfig,
    phantom_data: PhantomData<(T, N, I)>,
}

impl<T, P, N, I> ComputeOfferAnalyzer<T, P, N, I>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    pub fn new(
        rpc_provider: P,
        market_address: Address,
        validation_config: OfferValidationConfig,
    ) -> Self {
        Self {
            _rpc_provider: rpc_provider,
            market_address,
            validation_config,
            phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P, N, I> IntentAnalyzer for ComputeOfferAnalyzer<T, P, N, I>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
    I: ComputeIntent,
{
    type Intent = ComputeOffer<SystemParams>;

    async fn analyze(&self, latest_ts: u64, intent: &Self::Intent) -> Result<()> {
        // general correctness checks
        intent.validate(latest_ts, &self.market_address, &self.validation_config)?;

        //// TODO: economic checks

        Ok(())
    }
}
