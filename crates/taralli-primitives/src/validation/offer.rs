use alloy::primitives::{Address, FixedBytes, U256};
use serde::{Deserialize, Serialize};

use super::{
    CommonValidationConfig, FromMetaConfig, ProofCommon, Validate, ValidationConfig,
    ValidationMetaConfig,
};
use crate::Result;
use crate::{
    abi::universal_porchetta::UniversalPorchetta::ProofOffer,
    intents::ComputeOffer,
    systems::{ProvingSystem, ProvingSystemId},
    utils::{compute_offer_permit2_digest, compute_offer_witness},
    PrimitivesError,
};

// Specific config for offers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OfferSpecificConfig {
    pub maximum_allowed_reward: Option<U256>,
    pub minimum_allowed_stake: Option<U256>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OfferValidationConfig {
    pub common: CommonValidationConfig,
    pub specific: OfferSpecificConfig,
}

impl ValidationConfig for OfferValidationConfig {
    fn common(&self) -> &CommonValidationConfig {
        &self.common
    }
}

impl FromMetaConfig for OfferValidationConfig {
    fn from_meta(meta: &ValidationMetaConfig) -> Self {
        Self {
            common: meta.common.clone(),
            specific: meta.offer.clone(),
        }
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
impl<P: ProvingSystem> Validate for ComputeOffer<P> {
    type Config = OfferValidationConfig;

    fn proving_system_id(&self) -> ProvingSystemId {
        self.proving_system_id
    }

    fn proving_system(&self) -> &impl ProvingSystem {
        &self.proving_system
    }

    fn proof_common(&self) -> &impl ProofCommon {
        &self.proof_offer
    }

    fn validate_specific(&self, config: &Self::Config) -> Result<()> {
        // Offer-specific validation
        validate_offer(self, config)
    }
}

pub fn validate_offer<P: ProvingSystem>(
    offer: &ComputeOffer<P>,
    config: &OfferValidationConfig,
) -> Result<()> {
    let maximum_allowed_reward = config.specific.maximum_allowed_reward.ok_or_else(|| {
        PrimitivesError::ConfigError("maximum_allowed_reward must be configured".to_string())
    })?;

    let minimum_allowed_stake = config.specific.minimum_allowed_stake.ok_or_else(|| {
        PrimitivesError::ConfigError("minimum_allowed_stake must be configured".to_string())
    })?;

    // Offer-specific validation logic
    validate_signature(offer)?;
    validate_amount_constraints(offer, maximum_allowed_reward, minimum_allowed_stake)?;
    validate_verifier_details(offer)?;
    Ok(())
}

pub fn validate_amount_constraints<P: ProvingSystem>(
    offer: &ComputeOffer<P>,
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

pub fn validate_verifier_details<P: ProvingSystem>(_offer: &ComputeOffer<P>) -> Result<()> {
    Ok(())
}

pub fn validate_signature<P: ProvingSystem>(offer: &ComputeOffer<P>) -> Result<()> {
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
