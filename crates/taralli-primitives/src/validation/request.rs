use alloy::primitives::{Address, FixedBytes, U256};
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::{
    abi::universal_bombetta::UniversalBombetta::ProofRequest,
    intents::ComputeRequest,
    systems::{ProvingSystem, ProvingSystemId},
    utils::{compute_request_permit2_digest, compute_request_witness},
    PrimitivesError,
};

use super::{
    CommonValidationConfig, FromMetaConfig, ProofCommon, Validate, ValidationConfig,
    ValidationMetaConfig,
};

// Specific config for requests
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestSpecificConfig {
    pub maximum_allowed_stake: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestValidationConfig {
    pub common: CommonValidationConfig,
    pub specific: RequestSpecificConfig,
}

impl ValidationConfig for RequestValidationConfig {
    fn common(&self) -> &CommonValidationConfig {
        &self.common
    }
}

impl FromMetaConfig for RequestValidationConfig {
    fn from_meta(meta: &ValidationMetaConfig) -> Self {
        Self {
            common: meta.common.clone(),
            specific: meta.request.clone(),
        }
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
impl<P: ProvingSystem> Validate for ComputeRequest<P> {
    type Config = RequestValidationConfig;

    fn proving_system_id(&self) -> ProvingSystemId {
        self.proving_system_id
    }

    fn proving_system(&self) -> &impl ProvingSystem {
        &self.proving_system
    }

    fn proof_common(&self) -> &impl ProofCommon {
        &self.proof_request
    }

    fn validate_specific(&self, config: &Self::Config) -> Result<()> {
        // Request-specific validation
        validate_request(self, &config.specific)
    }
}

pub fn validate_request<P: ProvingSystem>(
    request: &ComputeRequest<P>,
    config: &RequestSpecificConfig,
) -> Result<()> {
    // Request-specific validation logic
    validate_signature(request)?;
    validate_amount_constraints(request, config.maximum_allowed_stake)?;
    validate_verifier_details(request)?;
    Ok(())
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
