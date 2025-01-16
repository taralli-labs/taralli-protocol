use alloy::primitives::{address, fixed_bytes, U256};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::{ProvingSystemInformation, VerifierConstraints};

// prover api
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Risc0ProofParams {
    pub elf: Vec<u8>,
    pub inputs: Vec<u8>,
}

// prover api
impl ProvingSystemInformation for Risc0ProofParams {
    fn validate_prover_inputs(&self) -> Result<()> {
        // WIP!
        // check elf makes sense
        // check image id matches?
        // check other stuff
        Ok(())
    }

    fn verifier_constraints() -> VerifierConstraints {
        VerifierConstraints {
            verifier: Some(address!("31766974fb795dF3f7d0c010a3D5c55e4bd8113e")),
            selector: Some(fixed_bytes!("ab750e75")),
            is_sha_commitment: Some(true),
            public_inputs_offset: Some(U256::from(32)),
            public_inputs_length: Some(U256::from(64)),
            has_partial_commitment_result_check: None,
            submitted_partial_commitment_result_offset: None,
            submitted_partial_commitment_result_length: None,
            predetermined_partial_commitment: None,
        }
    }
}
