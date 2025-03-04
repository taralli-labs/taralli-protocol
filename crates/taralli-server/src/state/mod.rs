use std::marker::PhantomData;
use std::time::Duration;
use taralli_primitives::alloy::{
    network::Ethereum, primitives::Address, providers::Provider, transports::Transport,
};

use crate::config::{Markets, ServerValidationConfigs};

pub mod offer;
pub mod request;

// Common base state with shared fields
#[derive(Clone)]
pub struct BaseState<T, P> {
    rpc_provider: P,
    markets: Markets,
    validation_timeout_seconds: Duration,
    validation_configs: ServerValidationConfigs,
    phantom: PhantomData<T>,
}

impl<T, P> BaseState<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub fn new(
        rpc_provider: P,
        markets: Markets,
        validation_timeout_seconds: Duration,
        validation_configs: ServerValidationConfigs,
    ) -> Self {
        Self {
            rpc_provider,
            markets,
            validation_timeout_seconds,
            validation_configs,
            phantom: PhantomData,
        }
    }

    pub fn rpc_provider(&self) -> P {
        self.rpc_provider.clone()
    }

    pub fn universal_bombetta_address(&self) -> Address {
        self.markets.universal_bombetta
    }

    pub fn universal_porchetta_address(&self) -> Address {
        self.markets.universal_porchetta
    }

    pub fn validation_timeout_seconds(&self) -> Duration {
        self.validation_timeout_seconds
    }

    pub fn validation_configs(&self) -> &ServerValidationConfigs {
        &self.validation_configs
    }
}
