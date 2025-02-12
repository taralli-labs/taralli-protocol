use alloy::primitives::{address, fixed_bytes, FixedBytes, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::error::Result;
use crate::systems::ProvingSystemParams;
use crate::systems::{CompositeSystem, ProvingSystem, SystemConfig, VerifierConstraints};

use super::system_id::AlignedLayer;
use super::SystemInputs;

// Core configuration for AlignedLayer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignedLayerConfig {
    pub underlying_system: Box<ProvingSystemParams>,
}

impl SystemConfig for AlignedLayerConfig {
    fn verifier_constraints(&self) -> VerifierConstraints {
        VerifierConstraints {
            verifier: Some(address!("58F280BeBE9B34c9939C3C39e0890C81f163B623")),
            selector: Some(fixed_bytes!("06045a91")),
            is_sha_commitment: Some(false),
            inputs_offset: Some(U256::from(32)),
            inputs_length: Some(U256::from(64)),
            has_partial_commitment_result_check: Some(false),
            submitted_partial_commitment_result_offset: Some(U256::ZERO),
            submitted_partial_commitment_result_length: Some(U256::ZERO),
            predetermined_partial_commitment: Some(FixedBytes::ZERO),
        }
    }

    fn validate_request(&self, details: &ProofRequestVerifierDetails) -> Result<()> {
        // Validate both aligned layer constraints and underlying system
        match *self.underlying_system.clone() {
            ProvingSystemParams::Risc0(params) => params.config().validate_request(details)?,
            ProvingSystemParams::Sp1(params) => params.config().validate_request(details)?,
            ProvingSystemParams::Gnark(params) => params.config().validate_request(details)?,
            _ => {
                return Err(crate::PrimitivesError::InvalidSystem(
                    "Unsupported underlying system".into(),
                ))
            }
        };
        Ok(())
    }

    fn validate_offer(&self, details: &ProofOfferVerifierDetails) -> Result<()> {
        // Similar validation for offers
        match *self.underlying_system.clone() {
            ProvingSystemParams::Risc0(params) => params.config().validate_offer(details)?,
            ProvingSystemParams::Sp1(params) => params.config().validate_offer(details)?,
            ProvingSystemParams::Gnark(params) => params.config().validate_offer(details)?,
            _ => {
                return Err(crate::PrimitivesError::InvalidSystem(
                    "Unsupported underlying system".into(),
                ))
            }
        }
        Ok(())
    }
}

impl CompositeSystem for AlignedLayerConfig {
    type UnderlyingSystem = ProvingSystemParams;

    fn underlying_system(&self) -> &Self::UnderlyingSystem {
        &self.underlying_system
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignedLayerProofParams {
    pub aligned_proving_system_id: String,
    pub config: AlignedLayerConfig,
    pub proving_system_aux_commitment: FixedBytes<32>,
}

impl ProvingSystem for AlignedLayerProofParams {
    type Config = AlignedLayerConfig;
    type Inputs = Value;

    fn system_id(&self) -> super::ProvingSystemId {
        AlignedLayer
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn inputs(&self) -> SystemInputs {
        self.config.underlying_system.inputs()
    }

    fn validate_inputs(&self) -> Result<()> {
        // Validate both aligned layer inputs and underlying system
        match *self.config.underlying_system.clone() {
            ProvingSystemParams::Risc0(params) => params.validate_inputs()?,
            ProvingSystemParams::Sp1(params) => params.validate_inputs()?,
            ProvingSystemParams::Gnark(params) => params.validate_inputs()?,
            _ => {
                return Err(crate::PrimitivesError::InvalidSystem(
                    "Unsupported underlying system".into(),
                ))
            }
        }
        Ok(())
    }
}
