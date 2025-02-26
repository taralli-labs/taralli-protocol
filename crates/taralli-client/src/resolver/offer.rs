use std::marker::PhantomData;

use alloy::network::Network;
use alloy::primitives::{Address, Bytes, FixedBytes};
use async_trait::async_trait;
use taralli_primitives::abi::universal_porchetta::UniversalPorchetta::UniversalPorchettaInstance;
use taralli_primitives::alloy::providers::Provider;
use taralli_primitives::alloy::transports::Transport;
use taralli_primitives::intents::offer::ComputeOffer;
use taralli_primitives::systems::SystemParams;

use crate::error::{ClientError, Result};

use super::IntentResolver;

pub struct ComputeOfferResolver<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    rpc_provider: P,
    market_address: Address,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> ComputeOfferResolver<T, P, N>
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
impl<T, P, N> IntentResolver<N> for ComputeOfferResolver<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    type Intent = ComputeOffer<SystemParams>;

    async fn resolve_intent(
        &self,
        intent_id: FixedBytes<32>,
        opaque_submission: Bytes,
    ) -> Result<N::ReceiptResponse> {
        let market_contract =
            UniversalPorchettaInstance::new(self.market_address, self.rpc_provider.clone());

        let call_return = market_contract
            .resolve(intent_id, opaque_submission)
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
