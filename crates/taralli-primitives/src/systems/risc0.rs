use alloy::primitives::{address, fixed_bytes, Address, FixedBytes};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::markets::Network;
use crate::systems::{System, SystemConfig};
use crate::validation::offer::OfferVerifierConstraints;
use crate::validation::request::RequestVerifierConstraints;

use super::system_id::Risc0;
use super::SystemInputs;

/// System proof parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Risc0ProofParams {
    pub elf: Vec<u8>,
    pub inputs: Vec<u8>,
}

impl SystemConfig for Risc0ProofParams {}

/// System implementation
impl System for Risc0ProofParams {
    type Config = Self;
    type Inputs = Vec<u8>;

    fn system_id(&self) -> super::SystemId {
        Risc0
    }

    fn config(&self) -> &Self::Config {
        self
    }

    fn inputs(&self) -> SystemInputs {
        SystemInputs::Bytes(self.inputs.clone())
    }

    fn validate_inputs(&self) -> Result<()> {
        if self.elf.is_empty() || self.inputs.is_empty() {
            return Err(crate::PrimitivesError::ProverInputsError(
                "elf or inputs bytes cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Risc0-specific verifier constraints with hardcoded values by network
pub struct Risc0VerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<FixedBytes<4>>,
    pub is_sha_commitment: Option<bool>,
    // Other fields
}

impl Risc0VerifierConstraints {
    /// Create network-specific constraints
    #[must_use]
    pub fn for_network(network: Network) -> Self {
        match network {
            Network::Sepolia => Self::sepolia(),
            // Add other networks as needed
        }
    }

    /// Sepolia network constraints
    #[must_use]
    pub fn sepolia() -> Self {
        Self {
            verifier: Some(address!("AC292cF957Dd5BA174cdA13b05C16aFC71700327")),
            selector: Some(fixed_bytes!("ab750e75")),
            is_sha_commitment: Some(true),
            // Set other fields
        }
    }

    // Add Network constraints here
}

// Implement conversion to Intent specific VerifierConstraints
impl From<Risc0VerifierConstraints> for RequestVerifierConstraints {
    fn from(constraints: Risc0VerifierConstraints) -> Self {
        RequestVerifierConstraints {
            verifier: constraints.verifier,
            selector: constraints.selector,
            is_sha_commitment: constraints.is_sha_commitment,
            ..Default::default()
        }
    }
}

impl From<Risc0VerifierConstraints> for OfferVerifierConstraints {
    fn from(constraints: Risc0VerifierConstraints) -> Self {
        OfferVerifierConstraints {
            verifier: constraints.verifier,
            selector: constraints.selector,
            is_sha_commitment: constraints.is_sha_commitment,
            ..Default::default()
        }
    }
}
