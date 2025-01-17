//use alloy::primitives::{address, fixed_bytes, U256};
use crate::abi::universal_bombetta::VerifierDetails;
use crate::error::{PrimitivesError, Result};
use crate::systems::{ProofConfiguration, ProvingSystemInformation, VerifierConstraints};
use serde::{Deserialize, Serialize};

use super::system_id::Gnark;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GnarkConfig {
    Groth16Bn254,
    PlonkBn254,
    PlonkBls12_381,
}

impl ProofConfiguration for GnarkConfig {
    fn verifier_constraints(&self) -> VerifierConstraints {
        match self {
            Self::Groth16Bn254 => VerifierConstraints::default(),
            Self::PlonkBn254 => VerifierConstraints::default(),
            Self::PlonkBls12_381 => VerifierConstraints::default(),
        }
    }

    fn validate(&self, _verifier_details: &VerifierDetails) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GnarkProofParams {
    pub scheme_config: GnarkConfig,
    pub r1cs: Vec<u8>,
    pub public_inputs: serde_json::Value,
    pub input: serde_json::Value,
}

impl ProvingSystemInformation for GnarkProofParams {
    type Config = GnarkConfig;

    fn proof_configuration(&self) -> &Self::Config {
        &self.scheme_config
    }

    fn validate_inputs(&self) -> Result<()> {
        if self.r1cs.is_empty() {
            return Err(PrimitivesError::ProverInputsError(
                "r1cs bytes cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn proving_system_id(&self) -> super::ProvingSystemId {
        Gnark
    }
}
