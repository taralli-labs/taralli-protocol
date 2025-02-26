use alloy::primitives::{Address, FixedBytes, U256};
use alloy::sol_types::SolValue;
use serde::{Deserialize, Serialize};

use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::Result;
use crate::{
    abi::universal_bombetta::UniversalBombetta::ProofRequest,
    intents::request::ComputeRequest,
    systems::{System, SystemId},
    utils::{compute_request_permit2_digest, compute_request_witness},
    PrimitivesError,
};

use super::{BaseValidationConfig, CommonValidationConfig, ProofCommon, Validate};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestValidationConfig {
    pub base: BaseValidationConfig,
    pub maximum_allowed_stake: u128,
}

impl CommonValidationConfig for RequestValidationConfig {
    fn minimum_proving_time(&self) -> u32 {
        self.base.minimum_proving_time
    }

    fn maximum_start_delay(&self) -> u32 {
        self.base.maximum_start_delay
    }

    fn supported_systems(&self) -> Vec<SystemId> {
        self.base.supported_systems.clone()
    }
}

// Implement for both proof types
impl ProofCommon for ProofRequest {
    fn market(&self) -> &Address {
        &self.market
    }
    fn nonce(&self) -> &U256 {
        &self.nonce
    }
    fn start_auction_timestamp(&self) -> u64 {
        self.startAuctionTimestamp
    }
    fn end_auction_timestamp(&self) -> u64 {
        self.endAuctionTimestamp
    }
    fn proving_time(&self) -> u32 {
        self.provingTime
    }
    fn inputs_commitment(&self) -> FixedBytes<32> {
        self.inputsCommitment
    }
}

// Implement for ComputeRequest
impl<S: System> Validate for ComputeRequest<S> {
    type Config = RequestValidationConfig;
    type VerifierConstraints = ProofRequestVerifierDetails;

    fn system_id(&self) -> SystemId {
        self.system_id
    }

    fn system(&self) -> &impl System {
        &self.system
    }

    fn proof_common(&self) -> &impl ProofCommon {
        &self.proof_request
    }

    fn validate_specific(&self, config: &Self::Config) -> Result<()> {
        // Request-specific validation
        validate_request(self, config)
    }
}

pub fn validate_request<S: System>(
    request: &ComputeRequest<S>,
    config: &RequestValidationConfig,
) -> Result<()> {
    // Request-specific validation logic
    validate_signature(request)?;
    validate_amount_constraints(request, config.maximum_allowed_stake)?;
    validate_request_verifier_details(request)?;
    Ok(())
}

pub fn validate_amount_constraints<S: System>(
    request: &ComputeRequest<S>,
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

pub fn validate_request_verifier_details<S: System>(request: &ComputeRequest<S>) -> Result<()> {
    // Decode and validate verifier details from the request
    let _verifier_details =
        ProofRequestVerifierDetails::abi_decode(&request.proof_request.extraData, true).map_err(
            |e| {
                PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {}", e))
            },
        )?;
    Ok(())
}

pub fn validate_signature<S: System>(request: &ComputeRequest<S>) -> Result<()> {
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
