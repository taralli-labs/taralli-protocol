use super::{validate_common, ValidateCommon};
use crate::{
    abi::universal_bombetta::ProofRequestVerifierDetails,
    request::ComputeRequest,
    systems::{ProvingSystem, ProvingSystemId},
    utils::{compute_request_permit2_digest, compute_request_witness},
    PrimitivesError, Result,
};
use alloy::primitives::{Address, FixedBytes, U256};

// ComputeRequest specific validation
impl<P: ProvingSystem> ValidateCommon for ComputeRequest<P> {
    type System = P;
    fn proving_system_id(&self) -> ProvingSystemId {
        self.proving_system_id
    }

    fn proving_system(&self) -> &Self::System {
        &self.proving_system
    }

    fn market_address(&self) -> &Address {
        &self.proof_request.market
    }

    fn nonce(&self) -> &U256 {
        &self.proof_request.nonce
    }

    fn start_auction_timestamp(&self) -> u64 {
        self.proof_request.startAuctionTimestamp
    }

    fn end_auction_timestamp(&self) -> u64 {
        self.proof_request.endAuctionTimestamp
    }

    fn proving_time(&self) -> u32 {
        self.proof_request.provingTime
    }

    fn inputs_commitment(&self) -> FixedBytes<32> {
        self.proof_request.inputsCommitment
    }
}

pub trait ValidateRequest: ValidateCommon {
    fn reward_token(&self) -> &Address;
    fn min_reward_amount(&self) -> &U256;
    fn max_reward_amount(&self) -> &U256;
    fn minimum_stake(&self) -> &u128;
    fn verifier_details(&self) -> &ProofRequestVerifierDetails;
}

pub fn validate_amount_constraints<P: ProvingSystem>(
    request: &ComputeRequest<P>,
    maximum_allowed_stake: u128,
) -> Result<()> {
    if request.proof_request.maxRewardAmount < request.proof_request.minRewardAmount {
        Err(PrimitivesError::ValidationError(
            "reward token amounts invalid".to_string(),
        ))
    } else if request.proof_request.minimumStake > maximum_allowed_stake {
        Err(PrimitivesError::ValidationError(
            "eth stake amount invalid".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_verifier_details<P: ProvingSystem>(_request: &ComputeRequest<P>) -> Result<()> {
    Ok(())
}

pub fn validate_signature<P: ProvingSystem>(request: &ComputeRequest<P>) -> Result<()> {
    // compute witness
    let witness = compute_request_witness(&request.proof_request);
    // compute permit digest
    let computed_digest = compute_request_permit2_digest(&request.proof_request, witness);
    // ec recover signing public key
    let computed_verifying_key = request
        .signature
        .recover_from_prehash(&computed_digest)
        .map_err(|e| PrimitivesError::ValidationError(format!("ec recover failed: {}", e)))?;
    let computed_signer = Address::from_public_key(&computed_verifying_key);

    // check signature validity
    if computed_signer != request.proof_request.signer {
        Err(PrimitivesError::ValidationError(
            "signature invalid: computed signer != request.signer".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_request<P: ProvingSystem>(
    request: &ComputeRequest<P>,
    latest_timestamp: u64,
    market_address: &Address,
    minimum_proving_time: u32,
    maximum_start_delay: u32,
    maximum_allowed_stake: u128,
    supported_proving_systems: &[ProvingSystemId],
) -> Result<()> {
    validate_common(
        request,
        latest_timestamp,
        market_address,
        minimum_proving_time,
        maximum_start_delay,
        supported_proving_systems,
    )?;
    validate_amount_constraints(request, maximum_allowed_stake)?;
    validate_verifier_details(request)?;
    validate_signature(request)?;
    Ok(())
}
