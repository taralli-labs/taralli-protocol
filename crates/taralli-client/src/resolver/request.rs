use std::marker::PhantomData;

use alloy::network::Network;
use alloy::primitives::{Address, Bytes, FixedBytes, B256};
use async_trait::async_trait;
use taralli_primitives::abi::universal_bombetta::UniversalBombetta::UniversalBombettaInstance;
use taralli_primitives::alloy::providers::Provider;
use taralli_primitives::alloy::transports::Transport;
use taralli_primitives::intents::request::ComputeRequest;
use taralli_primitives::systems::SystemParams;

use crate::error::{ClientError, Result};

use super::IntentResolver;

/// Resolver for ComputeRequests
pub struct ComputeRequestResolver<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    rpc_provider: P,
    market_address: Address,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> ComputeRequestResolver<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new(rpc_provider: P, market_address: Address) -> Self {
        Self {
            rpc_provider,
            market_address,
            phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P, N> IntentResolver<N> for ComputeRequestResolver<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    type Intent = ComputeRequest<SystemParams>;

    async fn resolve_intent(
        &self,
        intent_id: FixedBytes<32>,
        opaque_submission: Bytes,
    ) -> Result<N::ReceiptResponse> {
        tracing::info!("resolving intent");

        let market_contract =
            UniversalBombettaInstance::new(self.market_address, self.rpc_provider.clone());

        let call_return = market_contract
            .resolve(intent_id, opaque_submission, B256::ZERO)
            .send()
            .await
            .map_err(|e| ClientError::TransactionError(e.to_string()))?;

        let receipt = call_return
            .get_receipt()
            .await
            .map_err(|e| ClientError::TransactionFailure(e.to_string()))?;

        tracing::info!("resolve txs receipt: {:?}", receipt);

        Ok(receipt)
    }
}
