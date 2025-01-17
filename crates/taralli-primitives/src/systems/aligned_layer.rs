use alloy::primitives::{address, fixed_bytes, FixedBytes, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::abi::universal_bombetta::VerifierDetails;
use crate::systems::{ProofConfiguration, ProvingSystemInformation, VerifierConstraints};
use crate::error::Result;
use crate::systems::{risc0, sp1, gnark};
use super::system_id::AlignedLayer;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnderlyingProvingSystemParams {
    Risc0(risc0::Risc0ProofParams),
    SP1(sp1::Sp1ProofParams),
    Gnark(gnark::GnarkProofParams),
}

#[derive(Clone, Debug)]
pub struct AlignedLayerConfig {
    pub aligned_proving_system_id: String,
    pub proving_system_aux_commitment: FixedBytes<32>,
    pub underlying_config: UnderlyingConfig,
}

#[derive(Clone, Debug)]
pub enum UnderlyingConfig {
    Risc0(risc0::Risc0Config),
    SP1(sp1::Sp1Config),
    Gnark(gnark::GnarkConfig),
}

impl ProofConfiguration for AlignedLayerConfig {
    fn verifier_constraints(&self) -> VerifierConstraints {
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

    fn validate(&self, verifier_details: &VerifierDetails) -> Result<()> {
        // Validate underlying system's configuration
        match &self.underlying_config {
            UnderlyingConfig::Risc0(config) => config.validate(verifier_details)?,
            UnderlyingConfig::SP1(config) => config.validate(verifier_details)?,
            UnderlyingConfig::Gnark(config) => config.validate(verifier_details)?,
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignedLayerProofParams {
    pub aligned_proving_system_id: String,
    pub proving_system_aux_commitment: FixedBytes<32>,
    pub prover_inputs: Value,
    pub underlying_system_params: UnderlyingProvingSystemParams,
}

impl ProvingSystemInformation for AlignedLayerProofParams {
    type Config = AlignedLayerConfig;

    fn proof_configuration(&self) -> &Self::Config {
        static CONFIG: std::sync::OnceLock<AlignedLayerConfig> = std::sync::OnceLock::new();
        
        CONFIG.get_or_init(|| {
            let underlying_config = match &self.underlying_system_params {
                UnderlyingProvingSystemParams::Risc0(params) => 
                    UnderlyingConfig::Risc0(params.proof_configuration().clone()),
                UnderlyingProvingSystemParams::SP1(params) => 
                    UnderlyingConfig::SP1(params.proof_configuration().clone()),
                UnderlyingProvingSystemParams::Gnark(params) => 
                    UnderlyingConfig::Gnark(params.proof_configuration().clone()),
            };

            AlignedLayerConfig {
                aligned_proving_system_id: self.aligned_proving_system_id.clone(),
                proving_system_aux_commitment: self.proving_system_aux_commitment,
                underlying_config,
            }
        })
    }

    fn validate_inputs(&self) -> Result<()> {
        // Validate underlying system's inputs
        match &self.underlying_system_params {
            UnderlyingProvingSystemParams::Risc0(params) => params.validate_inputs()?,
            UnderlyingProvingSystemParams::SP1(params) => params.validate_inputs()?,
            UnderlyingProvingSystemParams::Gnark(params) => params.validate_inputs()?,
        }
        Ok(())
    }

    fn proving_system_id(&self) -> super::ProvingSystemId {
        AlignedLayer
    }
}

/*use alloy::primitives::{address, fixed_bytes, FixedBytes, U256};
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
        // TODO:
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
}*/
