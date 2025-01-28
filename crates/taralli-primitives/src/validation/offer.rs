use alloy::primitives::{Address, U256};

use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::offer::ComputeOffer;
use crate::systems::ProvingSystem;
use crate::{PrimitivesError, Result};

use super::ValidateCommon;

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
