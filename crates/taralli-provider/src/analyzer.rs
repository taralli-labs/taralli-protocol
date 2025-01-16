use std::marker::PhantomData;
use taralli_primitives::taralli_systems::id::ProvingSystemParams;
use taralli_primitives::validation::{
    validate_nonce, validate_proving_system_information, validate_signature, validate_verification_commitments 
};
use taralli_primitives::{
    alloy::{network::Network, providers::Provider, transports::Transport},
    Request,
};

use crate::{config::AnalyzerConfig, error::Result};

// TODO: complete a default analyzer with full validation for all existing systems, then start adding in economic logic
// Take incoming requests coming from server side events stream
// decide wether or not the inbound proof request is `safe` and `profitable` to bid upon.
// uses the bidder to submit the bid with a given target price if all the checks pass
pub struct RequestAnalyzer<T, P, N> {
    _rpc_provider: P,
    _config: AnalyzerConfig,
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
            _config: config,
            phantom_data: PhantomData,
        }
    }

    pub fn analyze(
        &self,
        request: &Request<ProvingSystemParams>,
        latest_timestamp: u64,
    ) -> Result<()> {
        // general correctness checks
        self.validate_request(request, latest_timestamp)?;

        //// TODO: economic checks

        Ok(())
    }

    pub fn validate_request(
        &self,
        request: &Request<ProvingSystemParams>,
        _latest_timestamp: u64,
    ) -> Result<()> {
        // all validation checks that are commented out with the exception of signature validation are trusted to 
        // be done before hand by the server as of now.

        //validate_proving_system_id(request, proving_system_ids)?;
        validate_proving_system_information(request)?;
        //validate_market_address(request, self.config.market_address)?;
        validate_verification_commitments(request)?;
        validate_nonce(request)?;
        //validate_amount_constraints(self.config.validation.maximum_allowed_stake, request)?;
        // TODO: better design for time constraint checking
        //validate_time_constraints(
        //    latest_timestamp,
        //    self.config.validation.minimum_allowed_proving_time,
        //    self.config.validation.maximum_start_delay,
        //    request,
        //)?;
        validate_signature(request)?;
        Ok(())
    }
}
