use std::sync::Arc;

use taralli_primitives::alloy::{network::Ethereum, providers::Provider, transports::Transport};

use crate::subscription_manager::SubscriptionManager;

use super::BaseState;

/// `ComputeRequest` specific state
#[derive(Clone)]
pub struct RequestState<T, P> {
    pub base: BaseState<T, P>,
    subscription_manager: Arc<SubscriptionManager>,
}

impl<T, P> RequestState<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub fn new(base: BaseState<T, P>, subscription_manager: Arc<SubscriptionManager>) -> Self {
        Self {
            base,
            subscription_manager,
        }
    }

    pub fn subscription_manager(&self) -> Arc<SubscriptionManager> {
        self.subscription_manager.clone()
    }
}

impl<T, P> std::ops::Deref for RequestState<T, P> {
    type Target = BaseState<T, P>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
