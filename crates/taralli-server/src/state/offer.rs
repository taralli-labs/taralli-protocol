use taralli_primitives::alloy::{network::Ethereum, providers::Provider, transports::Transport};

use crate::postgres::Db;

use super::BaseState;

/// ComputeOffer specific state
#[derive(Clone)]
pub struct OfferState<T, P> {
    pub base: BaseState<T, P>,
    intent_db: Db,
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

impl<T, P> std::ops::Deref for OfferState<T, P> {
    type Target = BaseState<T, P>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
