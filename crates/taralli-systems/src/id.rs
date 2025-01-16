use crate::error::Result;
use crate::systems::{
    aligned_layer::AlignedLayerProofParams, arkworks::ArkworksProofParams, gnark::GnarkProofParams,
    risc0::Risc0ProofParams, sp1::Sp1ProofParams,
};
use crate::{ProvingSystemInformation, VerifierConstraints};
use serde::{Deserialize, Serialize};

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

        // Implement TryFrom for runtime string conversion
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

        impl ProvingSystemInformation for ProvingSystemParams {
            fn validate_prover_inputs(&self) -> Result<()> {
                match self {
                    $(Self::$variant(params) => params.validate_prover_inputs()),*
                }
            }

            fn verifier_constraints() -> VerifierConstraints {
                // This should never be called directly on ProvingSystemParams!
                // Instead, use the specific proving system implementation
                VerifierConstraints::default()
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
