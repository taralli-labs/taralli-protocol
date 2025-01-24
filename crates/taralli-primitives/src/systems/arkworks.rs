use crate::abi::universal_bombetta::VerifierDetails;
use crate::error::Result;
use crate::systems::{ProofConfiguration, ProvingSystemInformation, VerifierConstraints};
use crate::PrimitivesError;
use alloy::primitives::{fixed_bytes, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::system_id::Arkworks;

#[derive(Clone, Debug)]
pub struct ArkworksConfig;

impl ProofConfiguration for ArkworksConfig {
    fn verifier_constraints(&self) -> VerifierConstraints {
        VerifierConstraints {
            verifier: None,
            selector: None,
            is_sha_commitment: Some(false),
            inputs_offset: None,
            inputs_length: None,
            has_partial_commitment_result_check: Some(false),
            submitted_partial_commitment_result_offset: Some(U256::ZERO),
            submitted_partial_commitment_result_length: Some(U256::ZERO),
            predetermined_partial_commitment: Some(fixed_bytes!(
                "0000000000000000000000000000000000000000000000000000000000000000"
            )),
        }
    }

    fn validate(&self, _verifier_details: &VerifierDetails) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArkworksProofParams {
    pub r1cs: Vec<u8>, // .r1cs file bytes
    pub wasm: Vec<u8>, // .wasm witness generator
    pub input: Value,  // Circuit input JSON
}

impl ProvingSystemInformation for ArkworksProofParams {
    type Config = ArkworksConfig;

    fn proof_configuration(&self) -> Self::Config {
        ArkworksConfig
    }

    fn validate_inputs(&self) -> Result<()> {
        if self.r1cs.is_empty() || self.wasm.is_empty() {
            return Err(PrimitivesError::ProverInputsError(
                "r1cs or wasm bytes cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn proving_system_id(&self) -> super::ProvingSystemId {
        Arkworks
    }
}
