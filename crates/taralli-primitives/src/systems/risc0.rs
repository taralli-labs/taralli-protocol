use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::systems::{System, SystemConfig};

use super::system_id::Risc0;
use super::SystemInputs;

/// System proof parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Risc0ProofParams {
    pub elf: Vec<u8>,
    pub inputs: Vec<u8>,
}

impl SystemConfig for Risc0ProofParams {}

/// System implementation
impl System for Risc0ProofParams {
    type Config = Self;
    type Inputs = Vec<u8>;

    fn system_id(&self) -> super::SystemId {
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
