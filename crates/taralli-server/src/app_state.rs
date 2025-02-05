use crate::postgres::Db;
use crate::subscription_manager::SubscriptionManager;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use taralli_primitives::alloy::{
    network::Ethereum, primitives::Address, providers::Provider, transports::Transport,
};
use taralli_primitives::intents::ComputeRequest;
use taralli_primitives::systems::{ProvingSystemId, ProvingSystemParams};
use taralli_primitives::validation::ValidationMetaConfig;

// Common base state with shared fields
#[derive(Clone)]
pub struct BaseState<T, P> {
    rpc_provider: P,
    market_address: Address,
    validation_timeout_seconds: Duration,
    validation_config: ValidationMetaConfig,
    phantom: PhantomData<T>,
}

#[derive(Clone)]
pub struct RequestState<T, P> {
    base: BaseState<T, P>,
    subscription_manager: Arc<SubscriptionManager<ComputeRequest<ProvingSystemParams>>>,
}

#[derive(Clone)]
pub struct OfferState<T, P> {
    base: BaseState<T, P>,
    intent_db: Db,
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

impl<T, P> std::ops::Deref for RequestState<T, P> {
    type Target = BaseState<T, P>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T, P> std::ops::Deref for OfferState<T, P> {
    type Target = BaseState<T, P>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T, P> RequestState<T, P> 
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub fn new(
        base: BaseState<T, P>,
        subscription_manager: SubscriptionManager<ComputeRequest<ProvingSystemParams>>,
    ) -> Self {
        Self {
            base,
            subscription_manager: Arc::new(subscription_manager),
        }
    }

    pub fn subscription_manager(&self) -> Arc<SubscriptionManager<ComputeRequest<ProvingSystemParams>>> {
        self.subscription_manager.clone()
    }
}

impl<T, P> OfferState<T, P> 
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub fn new(base: BaseState<T, P>, intent_db: Db) -> Self {
        Self { base, intent_db }
    }

    pub fn intent_db(&self) -> &Db {
        &self.intent_db
    }
}

// Implement constructors and methods for each state

/*#[derive(Clone)]
pub struct AppState<T, P, I> 
where
    I: Send + Clone
{
    rpc_provider: P,
    market_address: Address,
    subscription_manager: Arc<SubscriptionManager<I>>,
    intent_db: Db,
    validation_timeout_seconds: Duration,
    validation_config: ValidationMetaConfig,
    phantom: PhantomData<T>,
}

impl<T, P, I> AppState<T, P, I>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
    I: Send + Clone
{
    pub fn new(
        rpc_provider: P,
        market_address: Address,
        subscription_manager: SubscriptionManager<I>,
        intent_db: Db,
        validation_timeout_seconds: Duration,
        validation_config: ValidationMetaConfig,
    ) -> Self {
        Self {
            rpc_provider,
            subscription_manager: Arc::new(subscription_manager),
            intent_db,
            market_address,
            validation_timeout_seconds,
            validation_config,
            phantom: PhantomData,
        }
    }

    pub fn rpc_provider(&self) -> P {
        self.rpc_provider.clone()
    }

    pub fn subscription_manager(
        &self,
    ) -> Arc<SubscriptionManager<I>> {
        self.subscription_manager.clone()
    }

    pub fn validation_config(&self) -> ValidationMetaConfig {
        self.validation_config.clone()
    }

    pub fn market_address(&self) -> Address {
        self.market_address
    }

    pub fn supported_proving_systems(&self) -> Vec<ProvingSystemId> {
        self.validation_config
            .common
            .supported_proving_systems
            .clone()
    }

    pub fn minimum_allowed_proving_time(&self) -> u32 {
        self.validation_config.common.minimum_proving_time
    }

    pub fn maximum_allowed_start_delay(&self) -> u32 {
        self.validation_config.common.maximum_start_delay
    }

    pub fn maximum_allowed_stake(&self) -> u128 {
        self.validation_config.request.maximum_allowed_stake
    }

    pub fn intent_db(&self) -> &Db {
        &self.intent_db
    }

    pub fn validation_timeout_seconds(&self) -> Duration {
        self.validation_timeout_seconds
    }
}*/
