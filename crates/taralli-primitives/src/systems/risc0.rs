use alloy::primitives::{address, fixed_bytes, U256};
use serde::{Deserialize, Serialize};

use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::error::Result;
use crate::systems::{ProvingSystem, SystemConfig, VerifierConstraints};

use super::system_id::Risc0;
use super::SystemInputs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Risc0ProofParams {
    pub elf: Vec<u8>,
    pub inputs: Vec<u8>,
}

impl SystemConfig for Risc0ProofParams {
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

    fn validate_request(&self, _details: &ProofRequestVerifierDetails) -> Result<()> {
        Ok(())
    }

    fn validate_offer(&self, _details: &ProofOfferVerifierDetails) -> Result<()> {
        Ok(())
    }
}

impl ProvingSystem for Risc0ProofParams {
    type Config = Self;
    type Inputs = Vec<u8>;

    fn system_id(&self) -> super::ProvingSystemId {
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
