use alloy::primitives::{Address, FixedBytes, U256};
use alloy::sol_types::SolValue;
use serde::{Deserialize, Serialize};

use super::{
    BaseValidationConfig, CommonValidationConfig, ProofCommon, Validate
};
use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::Result;
use crate::{
    abi::universal_porchetta::UniversalPorchetta::ProofOffer,
    intents::ComputeOffer,
    systems::{System, SystemId},
    utils::{compute_offer_permit2_digest, compute_offer_witness},
    PrimitivesError,
};

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

impl ProofCommon for ProofOffer {
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

// Implement for ComputeOffer
impl<S: System> Validate for ComputeOffer<S> {
    type Config = OfferValidationConfig;
    type VerifierConstraints = ProofOfferVerifierDetails;

    fn system_id(&self) -> SystemId {
        self.system_id
    }

    fn system(&self) -> &impl System {
        &self.system
    }

    fn proof_common(&self) -> &impl ProofCommon {
        &self.proof_offer
    }

    fn validate_specific(&self, config: &Self::Config) -> Result<()> {
        // ComputeOffer-specific validation
        validate_offer(self, config)
    }
}

pub fn validate_offer<S: System>(
    offer: &ComputeOffer<S>,
    config: &OfferValidationConfig,
) -> Result<()> {
    // Offer-specific validation logic
    validate_signature(offer)?;
    validate_amount_constraints(offer, config.maximum_allowed_reward, config.minimum_allowed_stake)?;
    validate_offer_verifier_details(offer)?;
    Ok(())
}

pub fn validate_amount_constraints<S: System>(
    offer: &ComputeOffer<S>,
    maximum_allowed_reward: U256,
    minimum_allowed_stake: U256,
) -> Result<()> {
    if offer.proof_offer.rewardAmount > maximum_allowed_reward {
        Err(PrimitivesError::ValidationError(
            "token reward amount invalid".to_string(),
        ))
    } else if offer.proof_offer.stakeAmount < minimum_allowed_stake {
        Err(PrimitivesError::ValidationError(
            "token stake amount invalid".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_offer_verifier_details<S: System>(offer: &ComputeOffer<S>) -> Result<()> {
    // Decode and validate verifier details from the request
    let _verifier_details =
        ProofOfferVerifierDetails::abi_decode(&offer.proof_offer.extraData, true).map_err(
            |e| {
                PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {}", e))
            },
        )?;
    Ok(())
}

pub fn validate_signature<S: System>(offer: &ComputeOffer<S>) -> Result<()> {
    // compute witness
    let witness = compute_offer_witness(&offer.proof_offer);
    // compute permit digest
    let computed_digest = compute_offer_permit2_digest(&offer.proof_offer, witness);
    // ec recover signing public key
    let computed_verifying_key = offer
        .signature
        .recover_from_prehash(&computed_digest)
        .map_err(|e| PrimitivesError::ValidationError(format!("ec recover failed: {}", e)))?;
    let computed_signer = Address::from_public_key(&computed_verifying_key);

    // check signature validity
    if computed_signer != offer.proof_offer.signer {
        Err(PrimitivesError::ValidationError(
            "signature invalid: computed signer != request.signer".to_string(),
        ))
    } else {
        Ok(())
    }
}
