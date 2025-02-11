use std::marker::PhantomData;
use std::time::Duration;
use taralli_primitives::alloy::{
    network::Ethereum, primitives::Address, providers::Provider, transports::Transport,
};
use taralli_primitives::validation::ValidationMetaConfig;

pub mod offer;
pub mod request;

// Common base state with shared fields
#[derive(Clone)]
pub struct BaseState<T, P> {
    rpc_provider: P,
    market_address: Address,
    validation_timeout_seconds: Duration,
    validation_config: ValidationMetaConfig,
    phantom: PhantomData<T>,
}

impl<T, P> BaseState<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub fn new(
        rpc_provider: P,
        market_address: Address,
        validation_timeout_seconds: Duration,
        validation_config: ValidationMetaConfig,
    ) -> Self {
        Self {
            rpc_provider,
            market_address,
            validation_timeout_seconds,
            validation_config,
            phantom: PhantomData,
        }
    }

    pub fn rpc_provider(&self) -> P {
        self.rpc_provider.clone()
    }

    pub fn market_address(&self) -> Address {
        self.market_address
    }

    pub fn validation_timeout_seconds(&self) -> Duration {
        self.validation_timeout_seconds
    }

    pub fn validation_config(&self) -> &ValidationMetaConfig {
        &self.validation_config
    }
}
