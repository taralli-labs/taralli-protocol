use crate::abi::universal_bombetta::VerifierDetails;
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

#[derive(Debug, Default)]
pub struct VerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<FixedBytes<4>>,
    pub is_sha_commitment: Option<bool>,
    pub public_inputs_offset: Option<U256>,
    pub public_inputs_length: Option<U256>,
    pub has_partial_commitment_result_check: Option<bool>,
    pub submitted_partial_commitment_result_offset: Option<U256>,
    pub submitted_partial_commitment_result_length: Option<U256>,
    pub predetermined_partial_commitment: Option<FixedBytes<32>>,
}

pub trait ProofConfiguration: Debug + Send + Sync + 'static {
    // return pre-determined verifier constraints of the system
    fn verifier_constraints(&self) -> VerifierConstraints;
    // validate verification configuration
    fn validate(&self, verifier_details: &VerifierDetails) -> Result<()>;
}

pub trait ProvingSystemInformation: Send + Sync + Clone + Serialize + 'static {
    type Config: ProofConfiguration;
    fn proof_configuration(&self) -> &Self::Config;
    // Validate the inputs needed for proof generation
    fn validate_inputs(&self) -> Result<()>;
    // return system id based on information type
    fn proving_system_id(&self) -> ProvingSystemId;
}

macro_rules! proving_systems {
    ($(($variant:ident, $params:ty, $str:literal)),* $(,)?) => {
        // Generate ProvingSystemId enum
        #[allow(non_upper_case_globals)]
        #[allow(non_snake_case)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum ProvingSystemId {
            $($variant),*
        }

        // Generate const identifiers and strings
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
        }

        impl TryFrom<&str> for ProvingSystemId {
            type Error = String;

            fn try_from(s: &str) -> core::result::Result<Self, Self::Error> {
                match s.to_lowercase().as_str() {
                    $($str => Ok(Self::$variant),)*
                    _ => Err(format!("Invalid proving system: {}", s))
                }
            }
        }

        // Generate ProvingSystemParams enum
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub enum ProvingSystemParams {
            $(
                #[serde(rename = $str)]
                $variant($params),
            )*
        }

        impl ProvingSystemParams {
            pub fn proving_system_id(&self) -> ProvingSystemId {
                match self {
                    $(Self::$variant(_) => ProvingSystemId::$variant,)*
                }
            }
        }

        #[derive(Clone, Debug)]
        pub enum ProvingSystemParamsConfig {
            $(
                $variant(<$params as ProvingSystemInformation>::Config),
            )*
        }

        impl ProofConfiguration for ProvingSystemParamsConfig {
            fn verifier_constraints(&self) -> VerifierConstraints {
                match self {
                    $(Self::$variant(config) => config.verifier_constraints()),*
                }
            }

            fn validate(&self, verifier_details: &VerifierDetails) -> Result<()> {
                match self {
                    $(Self::$variant(config) => config.validate(verifier_details)),*
                }
            }
        }

        impl ProvingSystemInformation for ProvingSystemParams {
            type Config = ProvingSystemParamsConfig;

            fn proof_configuration(&self) -> &Self::Config {
                static CONFIG: std::sync::OnceLock<ProvingSystemParamsConfig> = std::sync::OnceLock::new();

                CONFIG.get_or_init(|| match self {
                    $(Self::$variant(params) =>
                        ProvingSystemParamsConfig::$variant(params.proof_configuration().clone()),)*
                })
            }

            fn validate_inputs(&self) -> Result<()> {
                match self {
                    $(Self::$variant(params) => params.validate_inputs()),*
                }
            }

            fn proving_system_id(&self) -> ProvingSystemId {
                match self {
                    $(Self::$variant(_) => ProvingSystemId::$variant),*
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
    (AlignedLayer, AlignedLayerProofParams, "aligned-layer"),
    (Arkworks, ArkworksProofParams, "arkworks"),
    (Gnark, GnarkProofParams, "gnark"),
    (Risc0, Risc0ProofParams, "risc0"),
    (Sp1, Sp1ProofParams, "sp1")
}
