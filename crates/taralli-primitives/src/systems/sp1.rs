use alloy::primitives::{address, fixed_bytes, U256};
use serde::{Deserialize, Serialize};

use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::error::Result;
use crate::systems::{MultiModeSystem, ProvingSystem, SystemConfig, VerifierConstraints};

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

impl SystemConfig for Sp1Config {
    fn verifier_constraints(&self) -> VerifierConstraints {
        let verifier = match self.mode {
            Sp1Mode::Groth16 => address!("E780809121774D06aD9B0EEeC620fF4B3913Ced1"),
            Sp1Mode::Plonk => address!("d2832Cf1fC8bA210FfABF62Db9A8781153131d16"),
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

    fn validate_request(&self, _details: &ProofRequestVerifierDetails) -> Result<()> {
        Ok(())
    }

    fn validate_offer(&self, _details: &ProofOfferVerifierDetails) -> Result<()> {
        Ok(())
    }
}

// Implement MultiModeSystem to indicate SP1 supports multiple proving modes
impl MultiModeSystem for Sp1Config {
    type Mode = Sp1Mode;

    fn proving_mode(&self) -> &Self::Mode {
        &self.mode
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sp1ProofParams {
    pub config: Sp1Config,
    pub elf: Vec<u8>,    // ELF binary containing the program
    pub inputs: Vec<u8>, // Program inputs
}

impl ProvingSystem for Sp1ProofParams {
    type Config = Sp1Config;
    type Inputs = Vec<u8>;

    fn system_id(&self) -> super::ProvingSystemId {
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
