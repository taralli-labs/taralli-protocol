use alloy::{network::Network, providers::Provider, transports::Transport};
use taralli_primitives::intents::ComputeIntent;

use crate::{analyzer::GenericAnalyzer, bidder::ComputeRequestBidder, resolver::ComputeRequestResolver, searcher::OfferSearcher, worker::WorkerManager};

use super::BaseClient;

pub struct ProviderSearchingClient<T, P, N, S, I, C>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    I: ComputeIntent
{
    base: BaseClient<T, P, N, S>,
    searcher: OfferSearcher,
    analyzer: GenericAnalyzer<T, P, N, I, C>,
    bidder: ComputeRequestBidder<T, P, N>,
    worker_manager: WorkerManager<I>,
    resolver: ComputeRequestResolver<T, P, N>,
}

// TODO