use alloy::primitives::{address, fixed_bytes, Address, FixedBytes};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::markets::Network;
use crate::systems::{MultiModeSystem, System, SystemConfig};
use crate::validation::offer::OfferVerifierConstraints;
use crate::validation::request::RequestVerifierConstraints;

use super::system_id::Sp1;
use super::SystemInputs;

// SP1 proving mode
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Sp1Mode {
    Groth16,
    Plonk,
}

// Core configuration for SP1
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sp1Config {
    pub mode: Sp1Mode,
}

impl SystemConfig for Sp1Config {}

// Implement MultiModeSystem to indicate SP1 supports multiple proving modes
impl MultiModeSystem for Sp1Config {
    type Mode = Sp1Mode;

    fn mode(&self) -> &Self::Mode {
        &self.mode
    }
}

/// System proof parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sp1ProofParams {
    pub config: Sp1Config,
    pub elf: Vec<u8>,    // ELF binary containing the program
    pub inputs: Vec<u8>, // Program inputs
}

/// System implementation
impl System for Sp1ProofParams {
    type Config = Sp1Config;
    type Inputs = Vec<u8>;

    fn system_id(&self) -> super::SystemId {
        Sp1
    }

    fn config(&self) -> &Self::Config {
        &self.config
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

/// Sp1-specific verifier constraints with hardcoded values by network
pub struct Sp1VerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<FixedBytes<4>>,
    pub is_sha_commitment: Option<bool>,
    // Other fields
}

impl Sp1VerifierConstraints {
    /// Create network-specific constraints
    pub fn for_network(network: Network) -> Self {
        match network {
            Network::Sepolia => Self::sepolia(),
            // Add other networks as needed
        }
    }

    /// Sepolia network constraints
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

// Implement conversion to RequestVerifierConstraints
impl From<Sp1VerifierConstraints> for RequestVerifierConstraints {
    fn from(constraints: Sp1VerifierConstraints) -> Self {
        RequestVerifierConstraints {
            verifier: constraints.verifier,
            selector: constraints.selector,
            is_sha_commitment: constraints.is_sha_commitment,
            ..Default::default()
        }
    }
}

// Implement conversion to OfferVerifierConstraints
impl From<Sp1VerifierConstraints> for OfferVerifierConstraints {
    fn from(constraints: Sp1VerifierConstraints) -> Self {
        OfferVerifierConstraints {
            verifier: constraints.verifier,
            selector: constraints.selector,
            is_sha_commitment: constraints.is_sha_commitment,
            ..Default::default()
        }
    }
}
