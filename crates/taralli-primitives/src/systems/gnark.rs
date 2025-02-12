use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::error::Result;
use crate::systems::{MultiModeSystem, ProvingSystem, SystemConfig, VerifierConstraints};

use super::system_id::Gnark;
use super::SystemInputs;

// Gnark proving mode
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GnarkMode {
    Groth16Bn254,
    PlonkBn254,
    PlonkBls12_381,
}

// Core configuration for Gnark
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GnarkConfig {
    pub mode: GnarkMode,
}

impl SystemConfig for GnarkConfig {
    fn verifier_constraints(&self) -> VerifierConstraints {
        match self.mode {
            GnarkMode::Groth16Bn254 => VerifierConstraints::default(),
            GnarkMode::PlonkBn254 => VerifierConstraints::default(),
            GnarkMode::PlonkBls12_381 => VerifierConstraints::default(),
        }
    }

    fn validate_request(&self, _details: &ProofRequestVerifierDetails) -> Result<()> {
        Ok(())
    }

    fn validate_offer(&self, _details: &ProofOfferVerifierDetails) -> Result<()> {
        Ok(())
    }
}

// Implement MultiModeSystem to indicate Gnark supports multiple proving modes
impl MultiModeSystem for GnarkConfig {
    type Mode = GnarkMode;

    fn proving_mode(&self) -> &Self::Mode {
        &self.mode
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GnarkProofParams {
    pub config: GnarkConfig,
    pub r1cs: Vec<u8>,        // R1CS constraint system
    pub inputs: Value,        // Private circuit inputs in JSON format
    pub public_inputs: Value, // Public inputs in JSON format
}

impl ProvingSystem for GnarkProofParams {
    type Config = GnarkConfig;
    type Inputs = Value;

    fn system_id(&self) -> super::ProvingSystemId {
        Gnark
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn inputs(&self) -> SystemInputs {
        SystemInputs::Json(self.inputs.clone())
    }

    fn validate_inputs(&self) -> Result<()> {
        if self.r1cs.is_empty() {
            return Err(crate::PrimitivesError::ProverInputsError(
                "r1cs bytes cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}
