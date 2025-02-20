use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use crate::systems::{System, SystemConfig};

use super::system_id::Arkworks;
use super::SystemInputs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArkworksProofParams {
    pub r1cs: Vec<u8>, // .r1cs file bytes
    pub wasm: Vec<u8>, // .wasm witness generator
    pub inputs: Value, // Circuit input JSON
}

impl SystemConfig for ArkworksProofParams {}

impl System for ArkworksProofParams {
    type Config = Self;
    type Inputs = Value;

    fn system_id(&self) -> super::SystemId {
        Arkworks
    }

    fn config(&self) -> &Self::Config {
        self
    }

    fn inputs(&self) -> SystemInputs {
        SystemInputs::Json(self.inputs.clone())
    }

    fn validate_inputs(&self) -> Result<()> {
        if self.r1cs.is_empty() || self.wasm.is_empty() {
            return Err(crate::PrimitivesError::ProverInputsError(
                "r1cs or wasm bytes cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}
