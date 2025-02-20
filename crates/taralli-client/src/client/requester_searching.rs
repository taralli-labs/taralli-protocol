use alloy::{network::Network, providers::Provider, transports::Transport};

use crate::{analyzer::GenericAnalyzer, searcher::OfferSearcher, tracker::ComputeRequestTracker};
use crate::error::{Result, ClientError};

use super::BaseClient;


pub struct RequesterSearchingClient<T, P, N, S, I, C>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    base: BaseClient<T, P, N, S>,
    searcher: OfferSearcher,
    analyzer: GenericAnalyzer<T, P, N, I, C>,
    tracker: ComputeRequestTracker<T, P, N>,
}

impl<T, P, N, S, I, C> RequesterSearchingClient<T, P, N, S, I, C>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new() -> Self {
        Self {
            base: todo!(),
            searcher: todo!(),
            analyzer: todo!(),
            tracker: todo!(),
        }
    }

    pub fn run() -> Result<()> {
        Ok(())
    }
}