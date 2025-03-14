use alloy::primitives::FixedBytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use crate::systems::SystemParams;
use crate::systems::{CompositeSystem, System, SystemConfig};

use super::system_id::AlignedLayer;
use super::SystemInputs;

// Core configuration for AlignedLayer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignedLayerConfig {
    pub underlying_system: Box<SystemParams>,
}

impl SystemConfig for AlignedLayerConfig {}

impl CompositeSystem for AlignedLayerConfig {
    type UnderlyingSystem = SystemParams;

    fn underlying_system(&self) -> &Self::UnderlyingSystem {
        &self.underlying_system
    }
}

/// System proof parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignedLayerProofParams {
    pub aligned_proving_system_id: String,
    pub config: AlignedLayerConfig,
    pub proving_system_aux_commitment: FixedBytes<32>,
}

/// System implementation
impl System for AlignedLayerProofParams {
    type Config = AlignedLayerConfig;
    type Inputs = Value;

    fn system_id(&self) -> super::SystemId {
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
            SystemParams::Risc0(params) => params.validate_inputs()?,
            SystemParams::Sp1(params) => params.validate_inputs()?,
            SystemParams::Gnark(params) => params.validate_inputs()?,
            _ => {
                return Err(crate::PrimitivesError::InvalidSystem(
                    "Unsupported underlying system".into(),
                ))
            }
        }
        Ok(())
    }
}
