use crate::error::Result;
use crate::systems::{
    aligned_layer::AlignedLayerProofParams, arkworks::ArkworksProofParams, gnark::GnarkProofParams,
    risc0::Risc0ProofParams, sp1::Sp1ProofParams,
};
use alloy::primitives::{Address, FixedBytes, U256};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod aligned_layer;
pub mod arkworks;
pub mod gnark;
pub mod risc0;
pub mod sp1;

// Core verifier constraints all systems must provide
#[derive(Debug, Default)]
pub struct VerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<FixedBytes<4>>,
    pub is_sha_commitment: Option<bool>,
    pub inputs_offset: Option<U256>,
    pub inputs_length: Option<U256>,
    pub has_partial_commitment_result_check: Option<bool>,
    pub submitted_partial_commitment_result_offset: Option<U256>,
    pub submitted_partial_commitment_result_length: Option<U256>,
    pub predetermined_partial_commitment: Option<FixedBytes<32>>,
}

// Base trait for system configuration
pub trait SystemConfig: for<'de> Deserialize<'de> + Debug + Clone {
    // Common configuration methods all systems must implement
}

// Trait for systems that have multiple proving modes
pub trait MultiModeSystem: SystemConfig {
    type Mode: Debug + Clone;
    fn mode(&self) -> &Self::Mode;
}

// Trait for systems that can use other systems
pub trait CompositeSystem: SystemConfig {
    type UnderlyingSystem: SystemConfig;
    fn underlying_system(&self) -> &Self::UnderlyingSystem;
}

#[derive(Clone, Debug)]
pub enum SystemInputs {
    Bytes(Vec<u8>),
    Json(serde_json::Value),
}

// Main trait that all systems implement
pub trait System: Send + Sync + 'static + Clone + Serialize + for<'de> Deserialize<'de> {
    type Config: SystemConfig;
    type Inputs: Debug + Clone;

    fn system_id(&self) -> SystemId;
    fn config(&self) -> &Self::Config;
    fn inputs(&self) -> SystemInputs;
    fn validate_inputs(&self) -> Result<()>;
    fn system_params(&self) -> Option<&SystemParams> {
        None
    }
}

// Helper macro to count the number of variants
macro_rules! count {
    () => (0usize);
    ($head:tt $(,$tail:tt)*) => (1usize + count!($($tail),*));
}

// Macro for generating system IDs and basic infrastructure
macro_rules! systems {
    (
        $(
            $(#[$attr:meta])*
            ($variant:ident, $str:literal, $params:ty)
        ),* $(,)?
    ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum SystemId {
            $(
                $(#[$attr])*
                $variant
            ),*
        }

        // System ID constants and metadata
        #[allow(non_upper_case_globals)]
        #[allow(non_snake_case)]
        pub mod system_id {
            use super::SystemId;
            $(
                pub const $variant: SystemId = SystemId::$variant;
                pub mod $variant {
                    pub const NAME: &str = $str;
                }
            )*
        }

        impl SystemId {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $str),*
                }
            }
            pub const fn all() -> [SystemId; {count!($($variant),*)}] {
                use SystemId::*;
                [
                    $($variant),*
                ]
            }
        }

        pub const SYSTEMS: [SystemId; {count!($($variant),*)}] = SystemId::all();

        impl TryFrom<&str> for SystemId {
            type Error = String;

            fn try_from(s: &str) -> core::result::Result<Self, Self::Error> {
                match s.to_lowercase().as_str() {
                    $($str => Ok(Self::$variant),)*
                    _ => Err(format!("Invalid proving system: {}", s))
                }
            }
        }

        // Main params enum that contains all system configurations
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub enum SystemParams {
            $(
                #[serde(rename = $str)]
                $variant($params),
            )*
        }

        impl SystemConfig for SystemParams {}

        impl System for SystemParams {
            type Config = Self;
            type Inputs = serde_json::Value;

            fn system_id(&self) -> SystemId {
                match self {
                    $(Self::$variant(_) => SystemId::$variant),*
                }
            }

            fn config(&self) -> &Self::Config {
                self
            }

            fn inputs(&self) -> SystemInputs {
                match self {
                    $(Self::$variant(params) => params.inputs()),*
                }
            }

            fn validate_inputs(&self) -> Result<()> {
                match self {
                    $(Self::$variant(params) => params.validate_inputs()),*
                }
            }

            fn system_params(&self) -> Option<&SystemParams> {
                Some(self)
            }
        }

        impl TryFrom<(&SystemId, Vec<u8>)> for SystemParams {
            type Error = String;

            fn try_from((id, data): (&SystemId, Vec<u8>)) -> core::result::Result<Self, Self::Error> {
                match id {
                    $(SystemId::$variant => {
                        serde_json::from_slice::<$params>(&data)
                            .map(SystemParams::$variant)
                            .map_err(|e| format!("Failed to parse {} params: {}", $str, e))
                    },)*
                }
            }
        }
    }
}

systems! {
    (AlignedLayer, "aligned-layer", AlignedLayerProofParams),
    (Arkworks, "arkworks", ArkworksProofParams),
    (Gnark, "gnark", GnarkProofParams),
    (Risc0, "risc0", Risc0ProofParams),
    (Sp1, "sp1", Sp1ProofParams)
}
