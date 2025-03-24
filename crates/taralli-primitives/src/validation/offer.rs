use alloy::primitives::{Address, FixedBytes, PrimitiveSignature, U256};
use alloy::sol_types::SolValue;
use serde::{Deserialize, Serialize};

use super::{
    BaseValidationConfig, CommonValidationConfig, CommonVerifierConstraints, IntentValidator,
};
use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::intents::offer::compute_offer_permit2_digest;
use crate::Result;
use crate::{
    abi::universal_porchetta::UniversalPorchetta::ProofOffer,
    intents::offer::ComputeOffer,
    systems::{System, SystemId},
    PrimitivesError,
};

/// Verifier constraints specific to ProofOffer proof commitments within ComputeOffer intents
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfferVerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<FixedBytes<4>>,
    pub is_sha_commitment: Option<bool>,
    pub inputs_offset: Option<U256>,
    pub inputs_length: Option<U256>,
}

impl CommonVerifierConstraints for OfferVerifierConstraints {
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
pub struct OfferValidationConfig {
    pub base: BaseValidationConfig,
    pub maximum_allowed_reward: U256,
    pub minimum_allowed_stake: U256,
}

impl CommonValidationConfig for OfferValidationConfig {
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
pub struct ComputeOfferValidator {
    validation_config: OfferValidationConfig,
    verifier_constraints: OfferVerifierConstraints,
}

impl ComputeOfferValidator {
    pub fn new(
        validation_config: OfferValidationConfig,
        verifier_constraints: OfferVerifierConstraints,
    ) -> Self {
        Self {
            validation_config,
            verifier_constraints,
        }
    }
}

impl<S: System> IntentValidator<ComputeOffer<S>> for ComputeOfferValidator {
    type ValidationConfig = OfferValidationConfig;
    type VerifierConstraints = OfferVerifierConstraints;

    fn validation_config(&self) -> &OfferValidationConfig {
        &self.validation_config
    }

    fn verifier_constraints(&self) -> &OfferVerifierConstraints {
        &self.verifier_constraints
    }

    fn validate_specific(&self, offer: &ComputeOffer<S>) -> Result<()> {
        validate_offer(offer, &self.validation_config, &self.verifier_constraints)
    }
}

/// ComputeOffer specific validation
pub fn validate_offer<S: System>(
    offer: &ComputeOffer<S>,
    config: &OfferValidationConfig,
    verifier_constraints: &OfferVerifierConstraints,
) -> Result<()> {
    // Offer-specific validation logic
    validate_offer_signature(&offer.proof_offer, &offer.signature)?;
    validate_offer_amount_constraints(
        &offer.proof_offer,
        config.maximum_allowed_reward,
        config.minimum_allowed_stake,
    )?;
    validate_offer_verifier_details(&offer.proof_offer, verifier_constraints)?;
    Ok(())
}

pub fn validate_offer_amount_constraints(
    proof_offer: &ProofOffer,
    maximum_allowed_reward: U256,
    minimum_allowed_stake: U256,
) -> Result<()> {
    if proof_offer.rewardAmount > maximum_allowed_reward {
        Err(PrimitivesError::ValidationError(
            "token reward amount invalid".to_string(),
        ))
    } else if proof_offer.stakeAmount < minimum_allowed_stake {
        Err(PrimitivesError::ValidationError(
            "token stake amount invalid".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_offer_verifier_details(
    proof_offer: &ProofOffer,
    verifier_constraints: &OfferVerifierConstraints,
) -> Result<()> {
    // Decode and validate verifier details structure from the intent
    let verifier_details = ProofOfferVerifierDetails::abi_decode(&proof_offer.extraData, true)
        .map_err(|e| {
            PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {}", e))
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

    Ok(())
}

pub fn validate_offer_signature(
    proof_offer: &ProofOffer,
    signature: &PrimitiveSignature,
) -> Result<()> {
    // compute permit digest
    let computed_digest = compute_offer_permit2_digest(proof_offer);
    // ec recover signing public key
    let computed_verifying_key = signature
        .recover_from_prehash(&computed_digest)
        .map_err(|e| PrimitivesError::ValidationError(format!("ec recover failed: {}", e)))?;
    let computed_signer = Address::from_public_key(&computed_verifying_key);

    // check signature validity
    if computed_signer != proof_offer.signer {
        Err(PrimitivesError::ValidationError(
            "signature invalid: computed signer != request.signer".to_string(),
        ))
    } else {
        Ok(())
    }
}
