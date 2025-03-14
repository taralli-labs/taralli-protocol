use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::systems::{MultiModeSystem, System, SystemConfig};

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

impl SystemConfig for Sp1Config {}

// Implement MultiModeSystem to indicate SP1 supports multiple proving modes
impl MultiModeSystem for Sp1Config {
    type Mode = Sp1Mode;

    fn mode(&self) -> &Self::Mode {
        &self.mode
    }
}

/// System proof parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sp1ProofParams {
    pub config: Sp1Config,
    pub elf: Vec<u8>,    // ELF binary containing the program
    pub inputs: Vec<u8>, // Program inputs
}

/// System implementation
impl System for Sp1ProofParams {
    type Config = Sp1Config;
    type Inputs = Vec<u8>;

    fn system_id(&self) -> super::SystemId {
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
