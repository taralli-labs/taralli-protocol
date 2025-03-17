use std::any::Any;

use crate::{
    systems::{System, SystemId, SYSTEMS},
    PrimitivesError, Result,
};
use alloy::primitives::{Address, FixedBytes, U256};
use serde::{Deserialize, Serialize};

pub mod offer;
pub mod request;

/// Common validation values needed across all intent types
pub trait CommonValidationConfig: Any {
    fn minimum_proving_time(&self) -> u32;
    fn maximum_start_delay(&self) -> u32;
    fn supported_systems(&self) -> Vec<SystemId>;
}

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

// Common trait for shared fields across all intent type's proof commitment structures
pub trait ProofCommon {
    fn market(&self) -> &Address;
    fn nonce(&self) -> &U256;
    fn start_auction_timestamp(&self) -> u64;
    fn end_auction_timestamp(&self) -> u64;
    fn proving_time(&self) -> u32;
    fn inputs_commitment(&self) -> FixedBytes<32>;
}

// Trait describing how intents are validated
pub trait Validate: Sized + Clone {
    type Config: CommonValidationConfig;
    type VerifierConstraints: Default;

    fn system_id(&self) -> SystemId;
    fn system(&self) -> &impl System;
    fn proof_common(&self) -> &impl ProofCommon;

    fn validate_system(&self, supported_systems: &[SystemId]) -> Result<()> {
        if !supported_systems.contains(&self.system_id()) {
            return Err(PrimitivesError::ValidationError(
                "unsupported proving system".into(),
            ));
        }

        // Validate that the proving system information matches the system ID
        if self.system().system_id() != self.system_id() {
            return Err(PrimitivesError::ValidationError(
                "provided proving system does not match system id".into(),
            ));
        }

        // Validate the proving system specific parameters
        self.system().validate_inputs().map_err(|e| {
            PrimitivesError::ValidationError(format!("invalid proving system parameters: {}", e))
        })
    }

    fn validate_market_address(&self, expected_market: &Address) -> Result<()> {
        if self.proof_common().market() != expected_market {
            return Err(PrimitivesError::ValidationError(
                "invalid market address".into(),
            ));
        }
        Ok(())
    }

    fn validate_time_constraints(
        &self,
        latest_timestamp: u64,
        min_proving_time: u32,
        max_start_delay: u32,
    ) -> Result<()> {
        let proof = self.proof_common();
        let start = proof.start_auction_timestamp();
        let end = proof.end_auction_timestamp();

        if latest_timestamp < start.saturating_sub(max_start_delay as u64)
            || latest_timestamp >= end
        {
            return Err(PrimitivesError::ValidationError("invalid timestamp".into()));
        }

        if proof.proving_time() < min_proving_time {
            return Err(PrimitivesError::ValidationError(
                "proving time too low".into(),
            ));
        }

        Ok(())
    }

    fn validate_nonce(&self) -> Result<()> {
        // TODO: Implement nonce validation logic
        Ok(())
    }

    // Type-specific validation that must be implemented
    fn validate_specific(
        &self,
        config: &Self::Config,
        verifier_constraints: &Self::VerifierConstraints,
    ) -> Result<()>;

    /// High-level validation that performs all checks
    fn validate(
        &self,
        latest_timestamp: u64,
        market_address: &Address,
        config: &Self::Config,
        verifier_constraints: &Self::VerifierConstraints,
    ) -> Result<()> {
        // Use individual validators
        self.validate_system(&config.supported_systems())?;
        self.validate_market_address(market_address)?;
        self.validate_time_constraints(
            latest_timestamp,
            config.minimum_proving_time(),
            config.maximum_start_delay(),
        )?;
        self.validate_nonce()?;

        // Run type-specific validation
        self.validate_specific(config, verifier_constraints)
    }
}
