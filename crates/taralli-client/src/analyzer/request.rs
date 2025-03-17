use std::{collections::HashMap, marker::PhantomData};

use alloy::{network::Network, primitives::Address, providers::Provider, transports::Transport};
use async_trait::async_trait;
use taralli_primitives::{
    intents::{request::ComputeRequest, ComputeIntent},
    systems::{SystemId, SystemParams},
    validation::{
        request::{RequestValidationConfig, RequestVerifierConstraints},
        Validate,
    },
};

use crate::error::Result;

use super::IntentAnalyzer;

/// Analyzes a ComputeRequest's validity and profitability
pub struct ComputeRequestAnalyzer<T, P, N, I>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    _rpc_provider: P,
    pub market_address: Address,
    pub validation_config: RequestValidationConfig,
    pub verifier_constraints: Option<HashMap<SystemId, RequestVerifierConstraints>>,
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
        verifier_constraints: Option<HashMap<SystemId, RequestVerifierConstraints>>,
    ) -> Self {
        Self {
            _rpc_provider: rpc_provider,
            market_address,
            validation_config,
            verifier_constraints,
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
        let system_id = intent.system_id();

        let default_constraint = Default::default();

        // Get the verifier constraints for this system ID if they exist
        let verifier_constraint = self
            .verifier_constraints
            .as_ref()
            .and_then(|constraints| constraints.get(&system_id))
            .unwrap_or(&default_constraint);

        // general correctness checks
        intent.validate(
            latest_ts,
            &self.market_address,
            &self.validation_config,
            verifier_constraint,
        )?;

        //// TODO: economic checks

        Ok(())
    }
}
