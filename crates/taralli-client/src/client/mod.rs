//! This module contains the various client configurations

use alloy::primitives::Address;
use std::marker::PhantomData;

pub mod provider;
pub mod requester;

/// Base client components shared across all client configurations
pub struct BaseClient<T, P, N, S> {
    rpc_provider: P,
    signer: S,
    market_address: Address,
    phantom: PhantomData<(T, N)>,
}

impl<T, P, N, S> BaseClient<T, P, N, S> {
    pub fn new(rpc_provider: P, signer: S, market_address: Address) -> Self {
        Self {
            rpc_provider,
            signer,
            market_address,
            phantom: PhantomData,
        }
    }
}
