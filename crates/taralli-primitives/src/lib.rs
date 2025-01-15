//! Core types and traits for the Taralli protocol
//!
//! This module re-exports commonly used types from alloy-primitives and taralli-systems
//! to ensure version compatibility and provide a single source of truth.

pub mod alloy {
    pub mod primitives {
        pub use taralli_systems::alloy::primitives::{
            address, b256, bytes, fixed_bytes, Address, Bytes, FixedBytes, U256,
        };
        // Keep additional alloy types that primitives needs
        pub use alloy::primitives::{PrimitiveSignature, B256};
    }

    pub mod network {
        pub use alloy::network::{
            primitives::{BlockResponse, BlockTransactionsKind, HeaderResponse},
            Ethereum, EthereumWallet, Network,
        };
    }

    pub mod consensus {
        pub use alloy::consensus::BlockHeader;
    }

    pub mod providers {
        pub use alloy::providers::{Provider, ProviderBuilder};
    }

    pub mod transports {
        pub use alloy::transports::Transport;
    }

    pub mod eips {
        pub use alloy::eips::{BlockId, BlockNumberOrTag::Latest};
    }

    pub mod utils {
        pub use alloy::hex;
    }

    pub mod dyn_abi {
        pub use alloy::dyn_abi;
    }

    pub mod signers {
        pub use alloy::signers::Signer;
    }
}

pub mod taralli_systems {
    pub mod id {
        pub use taralli_systems::id::*;
    }

    pub mod systems {
        pub use taralli_systems::systems::*;
    }

    pub mod traits {
        pub use taralli_systems::{ProvingSystemInformation, VerifierConstraints};
    }
}

// Taralli primitives
pub mod abi;
pub mod error;
pub mod market;
pub mod request;
pub mod utils;
pub mod validation;

pub use error::{PrimitivesError, Result};
pub use request::*;
