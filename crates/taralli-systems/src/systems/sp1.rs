use alloy::primitives::{address, fixed_bytes, U256};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::{ProvingSystemInformation, VerifierConstraints};

// prover api
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sp1ProofParams {
    pub elf: Vec<u8>,
    pub inputs: Vec<u8>,
}

// prover api
impl ProvingSystemInformation for Sp1ProofParams {
    fn validate_prover_inputs(&self) -> Result<()> {
        // WIP
        // check elf makes sense
        // check image id matches?
        // check other stuff
        Ok(())
    }

    fn verifier_constraints() -> VerifierConstraints {
        VerifierConstraints {
            verifier: Some(address!("397A5f7f3dBd538f23DE225B51f532c34448dA9B")),
            selector: Some(fixed_bytes!("ab750e75")),
            is_sha_commitment: Some(true),
            public_inputs_offset: Some(U256::from(0)),
            public_inputs_length: Some(U256::from(64)),
            has_partial_commitment_result_check: None,
            submitted_partial_commitment_result_offset: None,
            submitted_partial_commitment_result_length: None,
            predetermined_partial_commitment: None,
        }
    }
}
