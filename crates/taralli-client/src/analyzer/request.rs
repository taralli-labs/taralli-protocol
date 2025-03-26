use std::marker::PhantomData;

use async_trait::async_trait;
use taralli_primitives::alloy::{
    network::Network, primitives::Address, providers::Provider, transports::Transport,
};
use taralli_primitives::{
    intents::request::ComputeRequest,
    systems::SystemParams,
    validation::{
        registry::{ComputeRequestValidatorRegistry, ValidatorRegistry},
        request::{RequestValidationConfig, RequestVerifierConstraints},
    },
};

use crate::error::Result;

use super::IntentAnalyzer;

/// Analyzes a `ComputeRequest`'s validity and profitability
pub struct ComputeRequestAnalyzer<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    _rpc_provider: P,
    pub market_address: Address,
    pub validator_registry: ComputeRequestValidatorRegistry,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> ComputeRequestAnalyzer<T, P, N>
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
            validator_registry: ComputeRequestValidatorRegistry::new(
                validation_config.clone(),
                RequestVerifierConstraints::default(),
            ),
            phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P, N> IntentAnalyzer for ComputeRequestAnalyzer<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    type Intent = ComputeRequest<SystemParams>;

    async fn analyze(&self, latest_ts: u64, intent: &Self::Intent) -> Result<()> {
        // general correctness checks
        self.validator_registry
            .validate(intent, latest_ts, &self.market_address)?;

        //// TODO: economic checks

        Ok(())
    }
}
