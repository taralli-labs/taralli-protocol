use crate::error::{Result, SystemsError};
use crate::{ProvingSystemInformation, VerifierConstraints};
use alloy::primitives::{FixedBytes, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GnarkSchemeConfig {
    Groth16Bn254,
    PlonkBn254,
    PlonkBls12_381,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GnarkProofParams {
    pub scheme_config: GnarkSchemeConfig,
    pub r1cs: Vec<u8>,
    pub public_inputs: Value,
    pub input: Value,
}

impl ProvingSystemInformation for GnarkProofParams {
    fn validate_prover_inputs(&self) -> Result<()> {
        // Add validation logic specific to Arkworks
        if self.r1cs.is_empty() {
            return Err(SystemsError::ProverInputsError(
                "r1cs bytes cannot be empty".to_string(),
            ));
        }

        // Validate based on specific configuration
        match self.scheme_config {
            GnarkSchemeConfig::Groth16Bn254 => {
                // Validate Groth16/BN254 specific requirements
            }
            GnarkSchemeConfig::PlonkBn254 => {
                // Validate Plonk/BN254 specific requirements
            }
            GnarkSchemeConfig::PlonkBls12_381 => {
                // Validate Plonk/BLS12-381 specific requirements
            }
        }

        // WIP
        // assert other things
        Ok(())
    }

    fn verifier_constraints() -> VerifierConstraints {
        VerifierConstraints {
            verifier: None,
            selector: None,
            is_sha_commitment: Some(false),
            public_inputs_offset: None,
            public_inputs_length: None,
            has_partial_commitment_result_check: Some(false),
            submitted_partial_commitment_result_offset: Some(U256::ZERO),
            submitted_partial_commitment_result_length: Some(U256::ZERO),
            predetermined_partial_commitment: Some(FixedBytes::ZERO),
        }
    }
}
