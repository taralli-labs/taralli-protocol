use crate::error::{Result, ServerError};
use crate::subscription_manager::*;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use taralli_primitives::alloy::{
    network::Ethereum, primitives::Address, providers::Provider, transports::Transport,
};
use taralli_primitives::taralli_systems::id::ProvingSystemId;

// Generic over the type of request so that we can change it later without
// breaking the API
#[derive(Clone)]
pub struct AppState<T, P, M>
where
    M: Clone,
{
    rpc_provider: P,
    subscription_manager: Arc<SubscriptionManager<M>>,
    market_address: Address,
    proving_system_ids: Vec<ProvingSystemId>,
    validation_timeout_seconds: Duration,
    minimum_allowed_proving_time: u32,
    maximum_allowed_start_delay: u32,
    maximum_allowed_stake: u128,
    phantom: PhantomData<T>,
}

impl<T, P, M> AppState<T, P, M>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
    M: Clone,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rpc_provider: P,
        subscription_manager: Arc<SubscriptionManager<M>>,
        market_address: Address,
        proving_system_ids: Vec<String>,
        minimum_allowed_proving_time: u32,
        maximum_allowed_start_delay: u32,
        maximum_allowed_stake: u128,
        validation_timeout_seconds: Duration,
    ) -> Self {
        // Convert proving system IDs
        let proving_system_ids = proving_system_ids
            .iter()
            .map(|id| {
                ProvingSystemId::try_from(id.as_str())
                    .map_err(|e| ServerError::AppStateError(e.to_string()))
            })
            .collect::<Result<Vec<_>>>()
            .expect("failed to convert proving system ids");

        Self {
            rpc_provider,
            subscription_manager,
            market_address,
            proving_system_ids,
            minimum_allowed_proving_time,
            maximum_allowed_start_delay,
            maximum_allowed_stake,
            validation_timeout_seconds,
            phantom: PhantomData,
        }
    }

    pub fn rpc_provider(&self) -> P {
        self.rpc_provider.clone()
    }

    pub fn subscription_manager(&self) -> Arc<SubscriptionManager<M>> {
        self.subscription_manager.clone()
    }

    pub fn market_address(&self) -> Address {
        self.market_address
    }

    pub fn proving_system_ids(&self) -> Vec<ProvingSystemId> {
        self.proving_system_ids.clone()
    }

    pub fn minimum_allowed_proving_time(&self) -> u32 {
        self.minimum_allowed_proving_time
    }

    pub fn maximum_allowed_start_delay(&self) -> u32 {
        self.maximum_allowed_start_delay
    }

    pub fn maximum_allowed_stake(&self) -> u128 {
        self.maximum_allowed_stake
    }

    pub fn validation_timeout_seconds(&self) -> Duration {
        self.validation_timeout_seconds
    }
}
