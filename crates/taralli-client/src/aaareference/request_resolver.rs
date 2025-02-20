use crate::config::ResolverConfig;
use crate::error::{ClientError, Result};
use std::marker::PhantomData;
use taralli_primitives::abi::universal_bombetta::UniversalBombetta::UniversalBombettaInstance;
use taralli_primitives::alloy::network::Network;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::alloy::providers::Provider;
use taralli_primitives::alloy::transports::Transport;

pub struct RequestResolver<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    rpc_provider: P,
    config: ResolverConfig,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> RequestResolver<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new(rpc_provider: P, config: ResolverConfig) -> Self {
        Self {
            rpc_provider,
            config,
            phantom_data: PhantomData,
        }
    }

    pub async fn resolve_request(
        &self,
        request_id: FixedBytes<32>,
        opaque_submission: Bytes,
        submitted_partial_commitment: FixedBytes<32>,
    ) -> Result<N::ReceiptResponse> {
        let market_contract =
            UniversalBombettaInstance::new(self.config.market_address, self.rpc_provider.clone());

        let call_return = market_contract
            .resolve(request_id, opaque_submission, submitted_partial_commitment)
            .send()
            .await
            .map_err(|e| ClientError::TransactionError(e.to_string()))?;

        tracing::info!("resolve call_return done, getting txs recipt");

        let receipt = call_return
            .get_receipt()
            .await
            .map_err(|e| ClientError::TransactionFailure(e.to_string()))?;

        tracing::info!("resolve txs receipt: {:?}", receipt);

        Ok(receipt)
    }
}
