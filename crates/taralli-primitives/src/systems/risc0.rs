use crate::abi::universal_bombetta::VerifierDetails;
use crate::error::Result;
use crate::systems::{ProofConfiguration, ProvingSystemInformation, VerifierConstraints};
use alloy::primitives::{address, fixed_bytes, U256};
use serde::{Deserialize, Serialize};

use super::system_id::Risc0;

#[derive(Clone, Debug)]
pub struct Risc0Config;

impl ProofConfiguration for Risc0Config {
    fn verifier_constraints(&self) -> VerifierConstraints {
        VerifierConstraints {
            verifier: Some(address!("31766974fb795dF3f7d0c010a3D5c55e4bd8113e")),
            selector: Some(fixed_bytes!("ab750e75")),
            is_sha_commitment: Some(true),
            inputs_offset: Some(U256::from(32)),
            inputs_length: Some(U256::from(64)),
            has_partial_commitment_result_check: None,
            submitted_partial_commitment_result_offset: None,
            submitted_partial_commitment_result_length: None,
            predetermined_partial_commitment: None,
        }
    }

    fn validate(&self, _verifier_details: &VerifierDetails) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Risc0ProofParams {
    pub elf: Vec<u8>,
    pub inputs: Vec<u8>,
}

impl ProvingSystemInformation for Risc0ProofParams {
    type Config = Risc0Config;

    fn proof_configuration(&self) -> Self::Config {
        Risc0Config
    }

    fn validate_inputs(&self) -> Result<()> {
        // TODO: Validate ELF and inputs
        Ok(())
    }

    fn proving_system_id(&self) -> super::ProvingSystemId {
        Risc0
    }
}
