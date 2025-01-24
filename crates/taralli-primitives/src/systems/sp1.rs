use crate::abi::universal_bombetta::VerifierDetails;
use crate::error::Result;
use crate::systems::{ProofConfiguration, ProvingSystemInformation, VerifierConstraints};
use alloy::primitives::{address, fixed_bytes, U256};
use serde::{Deserialize, Serialize};

use super::system_id::Sp1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Sp1Config {
    Groth16,
    Plonk,
}

impl ProofConfiguration for Sp1Config {
    fn verifier_constraints(&self) -> VerifierConstraints {
        let verifier = match self {
            Self::Groth16 => address!("E780809121774D06aD9B0EEeC620fF4B3913Ced1"),
            Self::Plonk => address!("d2832Cf1fC8bA210FfABF62Db9A8781153131d16"),
        };

        VerifierConstraints {
            verifier: Some(verifier),
            selector: Some(fixed_bytes!("41493c60")),
            is_sha_commitment: Some(true),
            inputs_offset: Some(U256::from(0)),
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
pub struct Sp1ProofParams {
    pub elf: Vec<u8>,
    pub inputs: Vec<u8>,
    pub proof_config: Sp1Config,
}

impl ProvingSystemInformation for Sp1ProofParams {
    type Config = Sp1Config;

    fn proof_configuration(&self) -> Self::Config {
        self.proof_config.clone()
    }

    fn validate_inputs(&self) -> Result<()> {
        // TODO: Validate ELF and inputs
        Ok(())
    }

    fn proving_system_id(&self) -> super::ProvingSystemId {
        Sp1
    }
}
