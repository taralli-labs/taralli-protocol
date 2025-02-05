use std::marker::PhantomData;
use taralli_primitives::markets::UNIVERSAL_BOMBETTA_ADDRESS;
use taralli_primitives::systems::ProvingSystemParams;
use taralli_primitives::validation::Validate;
use taralli_primitives::{
    alloy::{network::Network, providers::Provider, transports::Transport},
    intents::ComputeRequest,
};

use crate::{config::AnalyzerConfig, error::Result};

// TODO: complete a default analyzer with full validation for all existing systems, then start adding in economic logic
// Take incoming requests coming from server side events stream
// decide wether or not the inbound proof request is `safe` and `profitable` to bid upon.
// uses the bidder to submit the bid with a given target price if all the checks pass
pub struct RequestAnalyzer<T, P, N> {
    _rpc_provider: P,
    config: AnalyzerConfig,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> RequestAnalyzer<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new(rpc_provider: P, config: AnalyzerConfig) -> Self {
        Self {
            _rpc_provider: rpc_provider,
            config,
            phantom_data: PhantomData,
        }
    }

    pub fn analyze(
        &self,
        request: &ComputeRequest<ProvingSystemParams>,
        latest_timestamp: u64,
    ) -> Result<()> {
        // general correctness checks
        request.validate(
            latest_timestamp,
            &UNIVERSAL_BOMBETTA_ADDRESS,
            &self.config.validation_config,
        )?;

        //// TODO: economic checks

        Ok(())
    }
}
