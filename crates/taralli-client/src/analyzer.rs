use alloy::primitives::Address;
use async_trait::async_trait;
use std::marker::PhantomData;
use taralli_primitives::{
    alloy::{network::Network, providers::Provider, transports::Transport},
    intents::ComputeIntent,
    validation::ValidationConfig,
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
pub struct GenericAnalyzer<T, P, N, I, C>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    _rpc_provider: P,
    market_address: Address,
    validation_config: C,
    phantom_data: PhantomData<(T, N, I)>,
}

impl<T, P, N, I, C> GenericAnalyzer<T, P, N, I, C>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    pub fn new(rpc_provider: P, market_address: Address, validation_config: C) -> Self {
        Self {
            _rpc_provider: rpc_provider,
            market_address,
            validation_config,
            phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P, N, I, C> IntentAnalyzer for GenericAnalyzer<T, P, N, I, C>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
    I: ComputeIntent,
    C: ValidationConfig + Send + Sync + Clone,
    I::Config: From<C>,
{
    type Intent = I;

    async fn analyze(&self, latest_ts: u64, intent: &Self::Intent) -> Result<()> {
        let validation_config = I::Config::from(self.validation_config.clone());

        // general correctness checks
        intent.validate(latest_ts, &self.market_address, &validation_config)?;

        //// TODO: economic checks

        Ok(())
    }
}
