use alloy::primitives::{Address, U256};

use crate::abi::universal_bombetta::ProofRequestVerifierDetails;

use super::ValidateCommon;

// ComputeRequest specific validation
pub trait ValidateRequest: ValidateCommon {
    fn reward_token(&self) -> &Address;
    fn min_reward_amount(&self) -> &U256;
    fn max_reward_amount(&self) -> &U256;
    fn minimum_stake(&self) -> &u128;
    fn verifier_details(&self) -> &ProofRequestVerifierDetails;
}
