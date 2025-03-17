//! This module contains the system trait and its implementations.

use crate::error::Result;
use crate::systems::{
    aligned_layer::AlignedLayerProofParams, arkworks::ArkworksProofParams, gnark::GnarkProofParams,
    risc0::Risc0ProofParams, sp1::Sp1ProofParams,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod aligned_layer;
pub mod arkworks;
pub mod gnark;
pub mod risc0;
pub mod sp1;

/// traits for system configuration
pub trait SystemConfig: for<'de> Deserialize<'de> + Debug + Clone {}

// system has multiple proving modes
pub trait MultiModeSystem: SystemConfig {
    type Mode: Debug + Clone;
    fn mode(&self) -> &Self::Mode;
}

// system can use other systems
pub trait CompositeSystem: SystemConfig {
    type UnderlyingSystem: SystemConfig;
    fn underlying_system(&self) -> &Self::UnderlyingSystem;
}

/// inputs can be represented as raw bytes or json
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

// Macro for generating system ID and system param utilities
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
