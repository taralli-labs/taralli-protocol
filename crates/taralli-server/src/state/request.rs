use std::sync::Arc;

use taralli_primitives::alloy::{network::Ethereum, providers::Provider, transports::Transport};
use taralli_primitives::{intents::ComputeRequest, systems::ProvingSystemParams};

use crate::subscription_manager::SubscriptionManager;

use super::BaseState;

#[derive(Clone)]
pub struct RequestState<T, P> {
    base: BaseState<T, P>,
    subscription_manager: Arc<SubscriptionManager<ComputeRequest<ProvingSystemParams>>>,
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

    pub fn subscription_manager(
        &self,
    ) -> Arc<SubscriptionManager<ComputeRequest<ProvingSystemParams>>> {
        self.subscription_manager.clone()
    }
}

impl<T, P> std::ops::Deref for RequestState<T, P> {
    type Target = BaseState<T, P>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
