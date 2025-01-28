use alloy::primitives::{Address, FixedBytes, U256};

use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::offer::ComputeOffer;
use crate::systems::{ProvingSystem, ProvingSystemId};
use crate::utils::{compute_offer_permit2_digest, compute_offer_witness};
use crate::{PrimitivesError, Result};

use super::{validate_common, ValidateCommon};

// ComputeOffer specific validation
impl<P: ProvingSystem> ValidateCommon for ComputeOffer<P> {
    type System = P;
    fn proving_system_id(&self) -> ProvingSystemId {
        self.proving_system_id
    }

    fn proving_system(&self) -> &Self::System {
        &self.proving_system
    }

    fn market_address(&self) -> &Address {
        &self.proof_offer.market
    }

    fn nonce(&self) -> &U256 {
        &self.proof_offer.nonce
    }

    fn start_auction_timestamp(&self) -> u64 {
        self.proof_offer.startAuctionTimestamp
    }

    fn end_auction_timestamp(&self) -> u64 {
        self.proof_offer.endAuctionTimestamp
    }

    fn proving_time(&self) -> u32 {
        self.proof_offer.provingTime
    }

    fn inputs_commitment(&self) -> FixedBytes<32> {
        self.proof_offer.inputsCommitment
    }
}

// ComputeOffer specific validation
pub trait ValidateOffer: ValidateCommon {
    fn reward_token(&self) -> &Address;
    fn reward_amount(&self) -> &U256;
    fn stake_token(&self) -> &Address;
    fn stake_amount(&self) -> &U256;
    fn verifier_details(&self) -> &ProofOfferVerifierDetails;
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

#[allow(clippy::too_many_arguments)]
pub fn validate_offer<P: ProvingSystem>(
    offer: &ComputeOffer<P>,
    latest_timestamp: u64,
    market_address: &Address,
    minimum_proving_time: u32,
    maximum_start_delay: u32,
    maximum_allowed_stake: U256,
    minimum_allowed_stake: U256,
    supported_proving_systems: &[ProvingSystemId],
) -> Result<()> {
    validate_common(
        offer,
        latest_timestamp,
        market_address,
        minimum_proving_time,
        maximum_start_delay,
        supported_proving_systems,
    )?;
    validate_amount_constraints(offer, maximum_allowed_stake, minimum_allowed_stake)?;
    validate_verifier_details(offer)?;
    validate_signature(offer)?;
    Ok(())
}
