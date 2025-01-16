use crate::error::{Result, SystemsError};
use crate::{ProvingSystemInformation, VerifierConstraints};
use alloy::primitives::{FixedBytes, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// prover api
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArkworksProofParams {
    pub r1cs: Vec<u8>, // .r1cs file bytes
    pub wasm: Vec<u8>, // .wasm witness generator
    pub input: Value,  // Circuit input JSON
}

impl ProvingSystemInformation for ArkworksProofParams {
    fn validate_prover_inputs(&self) -> Result<()> {
        // TODO: Add validation logic specific to Arkworks
        if self.r1cs.is_empty() || self.wasm.is_empty() {
            return Err(SystemsError::ProverInputsError(
                "r1cs or wasm bytes cannot be empty".to_string(),
            ));
        }

        // assert input structure is correct

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
