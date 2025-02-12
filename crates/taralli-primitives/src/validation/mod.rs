use crate::{
    systems::{ProvingSystem, ProvingSystemId, SYSTEMS},
    PrimitivesError, Result,
};
use alloy::primitives::{Address, FixedBytes, U256};
use serde::{Deserialize, Serialize};

pub mod offer;
pub mod request;

// Common validation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonValidationConfig {
    pub minimum_proving_time: u32,
    pub maximum_start_delay: u32,
    pub supported_proving_systems: Vec<ProvingSystemId>,
}

impl Default for CommonValidationConfig {
    fn default() -> Self {
        Self {
            minimum_proving_time: 30, // 30 secs,
            maximum_start_delay: 300, // 5 mins
            supported_proving_systems: SYSTEMS.to_vec(),
        }
    }
}

// Trait for type-specific validation configs
pub trait ValidationConfig: Clone {
    fn common(&self) -> &CommonValidationConfig;
}

// Meta config that contains all validation configs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationMetaConfig {
    // Common config shared by all intent types
    pub common: CommonValidationConfig,
    // Intent-specific configs
    pub request: request::RequestSpecificConfig,
    pub offer: offer::OfferSpecificConfig,
}

// Helper trait to get specific config from meta config
pub trait FromMetaConfig {
    fn from_meta(meta: &ValidationMetaConfig) -> Self;
}

// Common trait for shared fields across all intent type's proof structures
pub trait ProofCommon {
    fn market(&self) -> &Address;
    fn nonce(&self) -> &U256;
    fn start_auction_timestamp(&self) -> u64;
    fn end_auction_timestamp(&self) -> u64;
    fn proving_time(&self) -> u32;
    fn inputs_commitment(&self) -> FixedBytes<32>;
}

// Trait for intent types that can be validated
pub trait Validate: Sized + Clone {
    type Config: ValidationConfig;

    fn proving_system_id(&self) -> ProvingSystemId;
    fn proving_system(&self) -> &impl ProvingSystem;
    fn proof_common(&self) -> &impl ProofCommon;

    fn validate_proving_system(&self, supported_systems: &[ProvingSystemId]) -> Result<()> {
        if !supported_systems.contains(&self.proving_system_id()) {
            return Err(PrimitivesError::ValidationError(
                "unsupported proving system".into(),
            ));
        }

        // Validate that the proving system information matches the system ID
        if self.proving_system().system_id() != self.proving_system_id() {
            return Err(PrimitivesError::ValidationError(
                "provided proving system does not match system id".into(),
            ));
        }

        // Validate the proving system specific parameters
        self.proving_system().validate_inputs().map_err(|e| {
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
    fn validate_specific(&self, config: &Self::Config) -> Result<()>;

    /// High-level validation that performs all checks
    fn validate(
        &self,
        latest_timestamp: u64,
        market_address: &Address,
        config: &Self::Config,
    ) -> Result<()> {
        // Use individual validators
        self.validate_proving_system(&config.common().supported_proving_systems)?;
        self.validate_market_address(market_address)?;
        self.validate_time_constraints(
            latest_timestamp,
            config.common().minimum_proving_time,
            config.common().maximum_start_delay,
        )?;
        self.validate_nonce()?;

        // Run type-specific validation
        self.validate_specific(config)
    }
}
