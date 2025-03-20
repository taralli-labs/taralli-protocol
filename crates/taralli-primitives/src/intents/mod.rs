//! This module contains the ComputeIntent Implementations used by the protocol.

use crate::systems::{System, SystemId};
use alloy::primitives::{Address, FixedBytes, PrimitiveSignature, U256};
use serde::{Deserialize, Serialize};

pub mod offer;
pub mod request;

/*
type ValidationConfig: CommonValidationConfig;
type VerifierConstraints: Default + Debug + Clone;

/// Periphery data fields
// validation config
fn validation_config(&self) -> &Self::ValidationConfig;
// verifier_constraints
fn verifier_constraints(&self) -> &Self::VerifierConstraints;


// return system params associated to the intent's system data
fn system_params(&self) -> Option<&SystemParams> {
    self.system().system_params()
}

/// validation methods
// Top level validation method for full validation of the intent
fn validate(
    &self,
    latest_timestamp: u64,
    market_address: &Address,
    //config: &Self::ValidationConfig,
    //verifier_constraints: &Self::VerifierConstraints,
) -> Result<()> {
    // Use individual validators
    self.validate_system(&self.validation_config().supported_systems())?;
    validate_market_address(self.proof_commitment().market(), market_address)?;
    validate_time_constraints(
        self.proof_commitment().start_auction_timestamp(),
        self.proof_commitment().end_auction_timestamp(),
        self.proof_commitment().proving_time(),
        latest_timestamp,
        self.validation_config().minimum_proving_time(),
        self.validation_config().maximum_start_delay(),
    )?;
    validate_nonce()?;
    // Run type-specific validation
    self.validate_specific(self.validation_config(), self.verifier_constraints())
}
// Validate the intent's system data
fn validate_system(&self, supported_systems: &[SystemId]) -> Result<()> {
    if !supported_systems.contains(&self.system_id()) {
        return Err(PrimitivesError::ValidationError(
            "unsupported system".into(),
        ));
    }
    // Validate that the proving system information matches the system ID
    if self.system().system_id() != self.system_id() {
        return Err(PrimitivesError::ValidationError(
            "provided system does not match system id".into(),
        ));
    }
    // Validate the proving system specific parameters
    self.system().validate_inputs().map_err(|e| {
        PrimitivesError::ValidationError(format!("invalid system parameters: {}", e))
    })
}
// Type-specific validation that must be implemented
fn validate_specific(
    &self,
    config: &Self::ValidationConfig,
    verifier_constraints: &Self::VerifierConstraints,
) -> Result<()>;
*/

// Common trait for shared fields across all intent type's proof commitment structures
pub trait CommonProofCommitment: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    fn market(&self) -> &Address;
    fn nonce(&self) -> &U256;
    fn start_auction_timestamp(&self) -> u64;
    fn end_auction_timestamp(&self) -> u64;
    fn proving_time(&self) -> u32;
    fn inputs_commitment(&self) -> FixedBytes<32>;
}

/// Trait representing common behavior for compute intents
pub trait ComputeIntent: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    type System: System;
    type ProofCommitment: CommonProofCommitment;

    /// Compute Intent data
    fn system_id(&self) -> SystemId;
    fn system(&self) -> &impl System;
    fn proof_commitment(&self) -> &Self::ProofCommitment;
    fn signature(&self) -> &PrimitiveSignature;

    /// utility methods
    // type string associated to this intent type
    fn type_string(&self) -> String;
    // compute intent id
    fn compute_id(&self) -> FixedBytes<32>;
    // compute permit2 digest for intent signing
    fn compute_permit2_digest(&self) -> FixedBytes<32>;
}
