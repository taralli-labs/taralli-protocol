use alloy::primitives::{Address, U256};

use crate::abi::universal_porchetta::ProofOfferVerifierDetails;

use super::ValidateCommon;

// ComputeOffer specific validation
pub trait ValidateOffer: ValidateCommon {
    fn reward_token(&self) -> &Address;
    fn reward_amount(&self) -> &U256;
    fn stake_token(&self) -> &Address;
    fn stake_amount(&self) -> &U256;
    fn verifier_details(&self) -> &ProofOfferVerifierDetails;
}
