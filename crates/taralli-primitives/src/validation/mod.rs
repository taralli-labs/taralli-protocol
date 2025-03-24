use std::any::Any;
use std::fmt::Debug;

use crate::{
    intents::{CommonProofCommitment, ComputeIntent},
    systems::{System, SystemId, SYSTEMS},
    PrimitivesError, Result,
};
use alloy::primitives::{Address, FixedBytes, U256};
use serde::{Deserialize, Serialize};

pub mod offer;
pub mod registry;
pub mod request;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseValidationConfig {
    pub minimum_proving_time: u32,
    pub maximum_start_delay: u32,
    pub supported_systems: Vec<SystemId>,
}

impl Default for BaseValidationConfig {
    fn default() -> Self {
        Self {
            minimum_proving_time: 30, // 30 secs,
            maximum_start_delay: 300, // 5 mins
            supported_systems: SYSTEMS.to_vec(),
        }
    }
}

/// Common validation values needed across all intent types
pub trait CommonValidationConfig: Any {
    fn minimum_proving_time(&self) -> u32;
    fn maximum_start_delay(&self) -> u32;
    fn supported_systems(&self) -> Vec<SystemId>;
}

/// Common verifier constraints across all intent types
pub trait CommonVerifierConstraints: Default + Debug + Clone {
    fn verifier(&self) -> Option<Address>;
    fn selector(&self) -> Option<FixedBytes<4>>;
    fn inputs_offset(&self) -> Option<U256>;
    fn inputs_length(&self) -> Option<U256>;
}

/// Trait for validating compute intents
pub trait IntentValidator<I: ComputeIntent>: Send + Sync {
    type ValidationConfig: CommonValidationConfig;
    type VerifierConstraints: CommonVerifierConstraints;

    /// Get the validation configuration
    fn validation_config(&self) -> &Self::ValidationConfig;
    /// Get the verifier constraints
    fn verifier_constraints(&self) -> &Self::VerifierConstraints;

    /// Validate the intent with the given parameters
    fn validate(&self, intent: &I, latest_timestamp: u64, market_address: &Address) -> Result<()> {
        // Full validation logic
        validate_system(intent, &self.validation_config().supported_systems())?;
        validate_market_address(intent.proof_commitment().market(), market_address)?;
        validate_time_constraints(
            intent.proof_commitment().start_auction_timestamp(),
            intent.proof_commitment().end_auction_timestamp(),
            intent.proof_commitment().proving_time(),
            latest_timestamp,
            self.validation_config().minimum_proving_time(),
            self.validation_config().maximum_start_delay(),
        )?;
        validate_nonce()?;
        self.validate_specific(intent)
    }

    /// Validate intent-specific constraints
    fn validate_specific(&self, intent: &I) -> Result<()>;
}

pub fn validate_system<I: ComputeIntent>(intent: &I, supported_systems: &[SystemId]) -> Result<()> {
    if !supported_systems.contains(&intent.system_id()) {
        return Err(PrimitivesError::ValidationError(
            "unsupported system".into(),
        ));
    }

    // Validate that the proving system information matches the system ID
    if intent.system().system_id() != intent.system_id() {
        return Err(PrimitivesError::ValidationError(
            "provided system does not match system id".into(),
        ));
    }

    // Validate the proving system specific parameters
    intent.system().validate_inputs().map_err(|e| {
        PrimitivesError::ValidationError(format!("invalid system parameters: {}", e))
    })?;

    Ok(())
}

pub fn validate_market_address(market: &Address, expected_market: &Address) -> Result<()> {
    if market != expected_market {
        return Err(PrimitivesError::ValidationError(
            "invalid market address".into(),
        ));
    }
    Ok(())
}

pub fn validate_time_constraints(
    start_auction_timestamp: u64,
    end_auction_timestamp: u64,
    proving_time: u32,
    latest_timestamp: u64,
    min_proving_time: u32,
    max_start_delay: u32,
) -> Result<()> {
    if latest_timestamp < start_auction_timestamp.saturating_sub(max_start_delay as u64)
        || latest_timestamp >= end_auction_timestamp
    {
        return Err(PrimitivesError::ValidationError("invalid timestamp".into()));
    }

    if proving_time < min_proving_time {
        return Err(PrimitivesError::ValidationError(
            "proving time too low".into(),
        ));
    }

    Ok(())
}

pub fn validate_nonce() -> Result<()> {
    // TODO: Implement nonce validation logic
    Ok(())
}
