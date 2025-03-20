use std::marker::PhantomData;

use alloy::{network::Network, primitives::Address, providers::Provider, transports::Transport};
use async_trait::async_trait;
use taralli_primitives::{
    intents::offer::ComputeOffer,
    systems::SystemParams,
    validation::{
        offer::{OfferValidationConfig, OfferVerifierConstraints},
        registry::{ComputeOfferValidatorRegistry, ValidatorRegistry},
    },
};

use crate::error::Result;

use super::IntentAnalyzer;

/// Analyzes a ComputeOffer's validity and profitability
pub struct ComputeOfferAnalyzer<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    _rpc_provider: P,
    pub market_address: Address,
    pub validator_registry: ComputeOfferValidatorRegistry,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> ComputeOfferAnalyzer<T, P, N>
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
            validator_registry: ComputeOfferValidatorRegistry::new(
                validation_config.clone(),
                OfferVerifierConstraints::default(),
            ),
            phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P, N> IntentAnalyzer for ComputeOfferAnalyzer<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    type Intent = ComputeOffer<SystemParams>;

    async fn analyze(&self, latest_ts: u64, intent: &Self::Intent) -> Result<()> {
        // general correctness checks
        self.validator_registry
            .validate(intent, latest_ts, &self.market_address)?;

        //// TODO: economic checks

        Ok(())
    }
}
