use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
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
pub trait SystemConfig: Debug + Clone {
    // Common configuration methods all systems must implement
    fn verifier_constraints(&self) -> VerifierConstraints;
    fn validate_request(&self, details: &ProofRequestVerifierDetails) -> Result<()>;
    fn validate_offer(&self, details: &ProofOfferVerifierDetails) -> Result<()>;
}

// Trait for systems that have multiple proving modes
pub trait MultiModeSystem: SystemConfig {
    type Mode: Debug + Clone;
    fn proving_mode(&self) -> &Self::Mode;
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

// Main trait that all proving systems implement
pub trait ProvingSystem: Send + Sync + Clone + Serialize + 'static {
    type Config: SystemConfig;
    type Inputs: Debug + Clone;

    fn system_id(&self) -> ProvingSystemId;
    fn config(&self) -> &Self::Config;
    fn inputs(&self) -> SystemInputs;
    fn validate_inputs(&self) -> Result<()>;
}

// Helper macro to count the number of variants - define this first
macro_rules! count {
    () => (0usize);
    ($head:tt $(,$tail:tt)*) => (1usize + count!($($tail),*));
}

// Macro for generating system IDs and basic infrastructure
macro_rules! proving_systems {
    (
        $(
            $(#[$attr:meta])*
            ($variant:ident, $str:literal, $params:ty)
        ),* $(,)?
    ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum ProvingSystemId {
            $(
                $(#[$attr])*
                $variant
            ),*
        }

        // System ID constants and metadata
        #[allow(non_upper_case_globals)]
        #[allow(non_snake_case)]
        pub mod system_id {
            use super::ProvingSystemId;
            $(
                pub const $variant: ProvingSystemId = ProvingSystemId::$variant;
                pub mod $variant {
                    pub const NAME: &str = $str;
                }
            )*
        }

        impl ProvingSystemId {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $str),*
                }
            }
            pub const fn all() -> [ProvingSystemId; {count!($($variant),*)}] {
                use ProvingSystemId::*;
                [
                    $($variant),*
                ]
            }
        }

        pub const SYSTEMS: [ProvingSystemId; {count!($($variant),*)}] = ProvingSystemId::all();

        impl TryFrom<&str> for ProvingSystemId {
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
        pub enum ProvingSystemParams {
            $(
                #[serde(rename = $str)]
                $variant($params),
            )*
        }

        impl SystemConfig for ProvingSystemParams {
            fn verifier_constraints(&self) -> VerifierConstraints {
                match self {
                    $(Self::$variant(params) => params.config().verifier_constraints()),*
                }
            }

            fn validate_request(&self, details: &ProofRequestVerifierDetails) -> Result<()> {
                match self {
                    $(Self::$variant(params) => params.config().validate_request(details)),*
                }
            }

            fn validate_offer(&self, details: &ProofOfferVerifierDetails) -> Result<()> {
                match self {
                    $(Self::$variant(params) => params.config().validate_offer(details)),*
                }
            }
        }

        impl ProvingSystem for ProvingSystemParams {
            type Config = Self;
            type Inputs = serde_json::Value;

            fn system_id(&self) -> ProvingSystemId {
                match self {
                    $(Self::$variant(_) => ProvingSystemId::$variant),*
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
        }

        impl TryFrom<(&ProvingSystemId, Vec<u8>)> for ProvingSystemParams {
            type Error = String;

            fn try_from((id, data): (&ProvingSystemId, Vec<u8>)) -> core::result::Result<Self, Self::Error> {
                match id {
                    $(ProvingSystemId::$variant => {
                        serde_json::from_slice::<$params>(&data)
                            .map(ProvingSystemParams::$variant)
                            .map_err(|e| format!("Failed to parse {} params: {}", $str, e))
                    },)*
                }
            }
        }
    }
}

proving_systems! {
    (AlignedLayer, "aligned-layer", AlignedLayerProofParams),
    (Arkworks, "arkworks", ArkworksProofParams),
    (Gnark, "gnark", GnarkProofParams),
    (Risc0, "risc0", Risc0ProofParams),
    (Sp1, "sp1", Sp1ProofParams)
}
