use std::marker::PhantomData;
use taralli_primitives::taralli_systems::id::ProvingSystemParams;
use taralli_primitives::{
    alloy::{network::Network, providers::Provider, transports::Transport},
    validation::{validate_amount_constraints, validate_market_address, validate_signature},
    ProofRequest,
};

use crate::{config::AnalyzerConfig, error::Result};

// Take incoming proof requests coming from server side events stream
// decide wether or not the inbound proof request is `safe` and also `profitable` to bid upon.
// uses the proof request bidder to submit the bid with a given target price if all the checks pass
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
    pub fn new(_rpc_provider: P, config: AnalyzerConfig) -> Self {
        Self {
            _rpc_provider,
            config,
            phantom_data: PhantomData,
        }
    }

    pub fn analyze(
        &self,
        request: &ProofRequest<ProvingSystemParams>,
        _latest_timestamp: u64,
    ) -> Result<()> {
        // check that the incoming proof request's described proof workload matches what the provider
        // client gets back for the given inuts...

        //// general correctness checks
        // proving_system_id should exist
        // proving_system_commitment_id should exist at the given proving_system_id
        // market address exists and is correct
        // reward token is acceptable
        // end timestamp of auction is still far enough away from current time to continue analyzing
        self.validate_request(request, _latest_timestamp)?;

        // check decoded verifier details from submitted extra_data to make sure the proving_system_id,
        // verifier address, function selector, isShaCommitment (use keccak256 or sha256 for
        // commitments bool), public inputs offset/length, hasPartialCommitmentCheck (provide partial
        // pre-image to a partially commited to hash), submitted partial commitment result offset/length
        // predetermined partial commitment is correct

        // if there are public_inputs, hash them and make sure hash is the same as public inputs commitment

        //// economic checks

        Ok(())
    }

    pub fn validate_request(
        &self,
        request: &ProofRequest<ProvingSystemParams>,
        _latest_timestamp: u64,
    ) -> Result<()> {
        validate_market_address(request, self.config.market_address)?;
        // TODO better design for time constraint checking
        //validate_time_constraints(
        //    latest_timestamp,
        //    self.config.validation.minimum_allowed_proving_time,
        //    self.config.validation.maximum_start_delay,
        //    request,
        //)?;
        validate_amount_constraints(self.config.validation.maximum_allowed_stake, request)?;
        validate_signature(request)?;
        Ok(())
    }
}
