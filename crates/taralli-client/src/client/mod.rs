use std::marker::PhantomData;
use alloy::primitives::Address;
use url::Url;
use crate::api::ApiClient;

pub mod requester_requesting;
pub mod requester_searching;
pub mod provider_offering;
pub mod provider_searching;
pub mod provider_streaming;

// Base client components shared across all modes
pub struct BaseClient<T, P, N, S> {
    api_client: ApiClient,
    rpc_provider: P,
    signer: S,
    market_address: Address,
    phantom: PhantomData<(T, N)>,
}

impl<T, P, N, S> BaseClient<T, P, N, S> {
    pub fn new(server_url: Url, rpc_provider: P, signer: S, market_address: Address) -> Self {
        Self {
            api_client: ApiClient::new(server_url),
            rpc_provider,
            signer,
            market_address,
            phantom: PhantomData,
        }
    }
}
