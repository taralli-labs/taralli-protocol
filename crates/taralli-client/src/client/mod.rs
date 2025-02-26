use crate::api::ApiClient;
use alloy::primitives::Address;
use std::marker::PhantomData;
use url::Url;

pub mod provider;
pub mod requester;

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
