use alloy::primitives::{Address, FixedBytes, PrimitiveSignature, B256, U256};
use alloy::sol_types::SolValue;
use serde::{Deserialize, Serialize};

use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::intents::request::compute_request_permit2_digest;
use crate::Result;
use crate::{
    abi::universal_bombetta::UniversalBombetta::ProofRequest,
    intents::request::ComputeRequest,
    systems::{System, SystemId},
    PrimitivesError,
};

use super::{
    BaseValidationConfig, CommonValidationConfig, CommonVerifierConstraints, IntentValidator,
};

/// Verifier constraints specific to `ProofRequest` proof commitments withing `ComputeRequest` intents
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestVerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<FixedBytes<4>>,
    pub is_sha_commitment: Option<bool>,
    pub inputs_offset: Option<U256>,
    pub inputs_length: Option<U256>,
    pub has_partial_commitment_result_check: Option<bool>,
    pub submitted_partial_commitment_result_offset: Option<U256>,
    pub submitted_partial_commitment_result_length: Option<U256>,
    pub predetermined_partial_commitment: Option<B256>,
}

impl CommonVerifierConstraints for RequestVerifierConstraints {
    fn verifier(&self) -> Option<Address> {
        self.verifier
    }

    fn selector(&self) -> Option<FixedBytes<4>> {
        self.selector
    }

    fn inputs_offset(&self) -> Option<U256> {
        self.inputs_offset
    }

    fn inputs_length(&self) -> Option<U256> {
        self.inputs_length
    }
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

#[derive(Debug, Clone)]
pub struct ComputeRequestValidator {
    validation_config: RequestValidationConfig,
    verifier_constraints: RequestVerifierConstraints,
}

impl ComputeRequestValidator {
    #[must_use]
    pub fn new(
        validation_config: RequestValidationConfig,
        verifier_constraints: RequestVerifierConstraints,
    ) -> Self {
        Self {
            validation_config,
            verifier_constraints,
        }
    }
}

impl<S: System> IntentValidator<ComputeRequest<S>> for ComputeRequestValidator {
    type ValidationConfig = RequestValidationConfig;
    type VerifierConstraints = RequestVerifierConstraints;

    fn validation_config(&self) -> &RequestValidationConfig {
        &self.validation_config
    }

    fn verifier_constraints(&self) -> &RequestVerifierConstraints {
        &self.verifier_constraints
    }

    fn validate_specific(&self, request: &ComputeRequest<S>) -> Result<()> {
        validate_request(request, &self.validation_config, &self.verifier_constraints)
    }
}

/// `ComputeRequest` specific validation
pub fn validate_request<S: System>(
    request: &ComputeRequest<S>,
    validation_config: &RequestValidationConfig,
    verifier_constraints: &RequestVerifierConstraints,
) -> Result<()> {
    // Request-specific validation logic
    validate_request_signature(&request.proof_request, &request.signature)?;
    validate_request_amount_constraints(
        &request.proof_request,
        validation_config.maximum_allowed_stake,
    )?;
    validate_request_verifier_details(&request.proof_request, verifier_constraints)?;
    Ok(())
}

pub fn validate_request_amount_constraints(
    proof_request: &ProofRequest,
    maximum_allowed_stake: u128,
) -> Result<()> {
    if proof_request.maxRewardAmount < proof_request.minRewardAmount {
        Err(PrimitivesError::ValidationError(
            "reward token amounts invalid".to_string(),
        ))
    } else if proof_request.minimumStake > maximum_allowed_stake {
        Err(PrimitivesError::ValidationError(
            "eth stake amount invalid".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_request_verifier_details(
    proof_request: &ProofRequest,
    verifier_constraints: &RequestVerifierConstraints,
) -> Result<()> {
    // Decode and validate verifier details structure from the intent
    let verifier_details = ProofRequestVerifierDetails::abi_decode(&proof_request.extraData, true)
        .map_err(|e| {
            PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {e}"))
        })?;

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

pub fn validate_request_signature(
    proof_request: &ProofRequest,
    signature: &PrimitiveSignature,
) -> Result<()> {
    // compute permit digest
    let computed_digest = compute_request_permit2_digest(proof_request);
    // ec recover signing public key
    let computed_verifying_key = signature
        .recover_from_prehash(&computed_digest)
        .map_err(|e| PrimitivesError::ValidationError(format!("ec recover failed: {e}")))?;
    let computed_signer = Address::from_public_key(&computed_verifying_key);

    // check signature validity
    if computed_signer == proof_request.signer {
        Ok(())
    } else {
        Err(PrimitivesError::ValidationError(
            "signature invalid: computed signer != request.signer".to_string(),
        ))
    }
}
