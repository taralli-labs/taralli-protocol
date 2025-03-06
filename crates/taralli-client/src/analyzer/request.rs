use std::marker::PhantomData;

use alloy::{network::Network, primitives::Address, providers::Provider, transports::Transport};
use async_trait::async_trait;
use taralli_primitives::{
    intents::{request::ComputeRequest, ComputeIntent},
    systems::SystemParams,
    validation::{request::RequestValidationConfig, Validate},
};

use crate::error::Result;

use super::IntentAnalyzer;

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
