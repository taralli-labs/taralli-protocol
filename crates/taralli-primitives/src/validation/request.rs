use alloy::primitives::{Address, FixedBytes, B256, U256};
use alloy::sol_types::SolValue;
use serde::{Deserialize, Serialize};

use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::intents::ComputeIntent;
use crate::Result;
use crate::{
    abi::universal_bombetta::UniversalBombetta::ProofRequest,
    intents::request::ComputeRequest,
    systems::{System, SystemId},
    PrimitivesError,
};

use super::{BaseValidationConfig, CommonValidationConfig, ProofCommon, Validate};

/// Verifier constraints specific to ProofRequest proof commitments withing ComputeRequest intents
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestVerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<[u8; 4]>,
    pub is_sha_commitment: Option<bool>,
    pub inputs_offset: Option<U256>,
    pub inputs_length: Option<U256>,
    pub has_partial_commitment_result_check: Option<bool>,
    pub submitted_partial_commitment_result_offset: Option<U256>,
    pub submitted_partial_commitment_result_length: Option<U256>,
    pub predetermined_partial_commitment: Option<B256>,
}

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

// Implement Validate for ComputeRequest
impl<S: System> Validate for ComputeRequest<S> {
    type Config = RequestValidationConfig;
    type VerifierConstraints = RequestVerifierConstraints;

    fn system_id(&self) -> SystemId {
        self.system_id
    }

    fn system(&self) -> &impl System {
        &self.system
    }

    fn proof_common(&self) -> &impl ProofCommon {
        &self.proof_request
    }

    fn validate_specific(
        &self,
        config: &Self::Config,
        verifier_constraints: &Self::VerifierConstraints,
    ) -> Result<()> {
        // Request-specific validation
        validate_request(self, config, verifier_constraints)
    }
}

/// ComputeRequest specific validation
pub fn validate_request<S: System>(
    request: &ComputeRequest<S>,
    config: &RequestValidationConfig,
    verifier_constraints: &RequestVerifierConstraints,
) -> Result<()> {
    // Request-specific validation logic
    validate_signature(request)?;
    validate_amount_constraints(request, config.maximum_allowed_stake)?;
    validate_request_verifier_details(request, verifier_constraints)?;
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

pub fn validate_request_verifier_details<S: System>(
    request: &ComputeRequest<S>,
    verifier_constraints: &RequestVerifierConstraints,
) -> Result<()> {
    // Decode and validate verifier details structure from the intent
    let verifier_details =
        ProofRequestVerifierDetails::abi_decode(&request.proof_request.extraData, true).map_err(
            |e| {
                PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {}", e))
            },
        )?;

    // Check each constraint only if it's set
    if let Some(expected_verifier) = verifier_constraints.verifier {
        if verifier_details.verifier != expected_verifier {
            return Err(PrimitivesError::ValidationError(
                "verifier address does not match constraints".to_string(),
            ));
        }
    }

    if let Some(expected_selector) = verifier_constraints.selector {
        if verifier_details.selector != expected_selector {
            return Err(PrimitivesError::ValidationError(
                "verifier selector does not match constraints".to_string(),
            ));
        }
    }

    if let Some(expected_is_sha_commitment) = verifier_constraints.is_sha_commitment {
        if verifier_details.isShaCommitment != expected_is_sha_commitment {
            return Err(PrimitivesError::ValidationError(
                "isShaCommitment flag does not match constraints".to_string(),
            ));
        }
    }

    if let Some(expected_inputs_offset) = verifier_constraints.inputs_offset {
        if verifier_details.inputsOffset != expected_inputs_offset {
            return Err(PrimitivesError::ValidationError(
                "inputs offset does not match constraints".to_string(),
            ));
        }
    }

    if let Some(expected_inputs_length) = verifier_constraints.inputs_length {
        if verifier_details.inputsLength != expected_inputs_length {
            return Err(PrimitivesError::ValidationError(
                "inputs length does not match constraints".to_string(),
            ));
        }
    }

    if let Some(expected_has_partial) = verifier_constraints.has_partial_commitment_result_check {
        if verifier_details.hasPartialCommitmentResultCheck != expected_has_partial {
            return Err(PrimitivesError::ValidationError(
                "hasPartialCommitmentResultCheck flag does not match constraints".to_string(),
            ));
        }
    }

    if let Some(expected_offset) = verifier_constraints.submitted_partial_commitment_result_offset {
        if verifier_details.submittedPartialCommitmentResultOffset != expected_offset {
            return Err(PrimitivesError::ValidationError(
                "submittedPartialCommitmentResultOffset does not match constraints".to_string(),
            ));
        }
    }

    if let Some(expected_length) = verifier_constraints.submitted_partial_commitment_result_length {
        if verifier_details.submittedPartialCommitmentResultLength != expected_length {
            return Err(PrimitivesError::ValidationError(
                "submittedPartialCommitmentResultLength does not match constraints".to_string(),
            ));
        }
    }

    if let Some(expected_commitment) = verifier_constraints.predetermined_partial_commitment {
        if verifier_details.predeterminedPartialCommitment != expected_commitment {
            return Err(PrimitivesError::ValidationError(
                "predeterminedPartialCommitment does not match constraints".to_string(),
            ));
        }
    }

    Ok(())
}

pub fn validate_signature<S: System>(request: &ComputeRequest<S>) -> Result<()> {
    // compute permit digest
    let computed_digest = request.compute_permit2_digest();
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
