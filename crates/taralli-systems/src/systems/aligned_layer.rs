use alloy::primitives::{address, fixed_bytes, FixedBytes, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use crate::{ProvingSystemInformation, VerifierConstraints};

// aligned layer supported proving systems
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnderlyingProvingSystemParams {
    Risc0(crate::systems::risc0::Risc0ProofParams),
    SP1(crate::systems::sp1::Sp1ProofParams),
    Gnark(crate::systems::gnark::GnarkProofParams),
}

// prover api
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignedLayerProofParams {
    pub aligned_proving_system_id: String,
    pub proving_system_aux_commitment: FixedBytes<32>,
    pub prover_inputs: Value,
    pub underlying_system_params: UnderlyingProvingSystemParams,
}

// prover api
impl ProvingSystemInformation for AlignedLayerProofParams {
    fn validate_prover_inputs(&self) -> Result<()> {
        // WIP
        // check aligned layer params

        // Validate the underlying system's inputs
        match &self.underlying_system_params {
            UnderlyingProvingSystemParams::Risc0(params) => params.validate_prover_inputs()?,
            UnderlyingProvingSystemParams::SP1(params) => params.validate_prover_inputs()?,
            UnderlyingProvingSystemParams::Gnark(params) => params.validate_prover_inputs()?,
        }
        Ok(())
    }

    fn verifier_constraints() -> VerifierConstraints {
        VerifierConstraints {
            // AlignedLayerServiceManager contract
            verifier: Some(address!("58F280BeBE9B34c9939C3C39e0890C81f163B623")),
            // AlignedLayerServiceManager.verifyBatchInclusion.selector
            selector: Some(fixed_bytes!("06045a91")),
            is_sha_commitment: Some(false),
            public_inputs_offset: Some(U256::from(32)),
            public_inputs_length: Some(U256::from(64)),
            has_partial_commitment_result_check: Some(false),
            submitted_partial_commitment_result_offset: Some(U256::ZERO),
            submitted_partial_commitment_result_length: Some(U256::ZERO),
            predetermined_partial_commitment: Some(FixedBytes::ZERO),
        }
    }
}
