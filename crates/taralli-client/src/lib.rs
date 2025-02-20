pub mod analyzer;
pub mod api;
pub mod bidder;
pub mod client;
pub mod config;
pub mod error;
pub mod intent_builder;
pub mod nonce_manager;
pub mod resolver;
pub mod searcher;
pub mod tracker;
pub mod worker;

/*pub struct TaralliClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone
{
    config: ClientConfig<T, P, N, S>,
    api: ApiClient,
    tracker: Option<IntentTracker<T, P, N>>,
    builder: Option<IntentBuilder<T, P, N>>,
    searcher: Option<IntentSearcher>,
    worker_manager: Option<WorkerManager>,
    resolver: Option<IntentResolver<T, P, N>>,
}

impl<T, P, N, S> TaralliClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone
{
    pub fn new(config: ClientConfig<T, P, N, S>) -> Self {
        let api = ApiClient::new(config.server_url.clone());

        // Configure components based on role and mode
        let (builder, tracker, searcher, worker_manager, resolver) = match &config.role {
            ClientMode::Requester(mode) => match mode {
                RequesterMode::Searcher { search_config, acceptance_config } => {
                    // Searcher needs offer-focused builder and tracker
                    let builder = IntentBuilder::new_offer_acceptor(
                        config.rpc_provider.clone(),
                        config.signer.address(),
                        config.market_address,
                        acceptance_config.clone(),
                    );

                    let tracker = IntentTracker::new_offer_tracker(
                        config.rpc_provider.clone(),
                        config.market_address,
                    );

                    let searcher = Some(IntentSearcher::new_offer_searcher(
                        search_config.clone(),
                        acceptance_config.clone(),
                    ));

                    (builder, tracker, searcher, None, None)
                },
                RequesterMode::Requesting { request_config } => {
                    // Requesting needs request-focused builder and tracker
                    let builder = IntentBuilder::new_request_builder(
                        config.rpc_provider.clone(),
                        config.signer.address(),
                        config.market_address,
                        request_config.clone(),
                    );

                    let tracker = IntentTracker::new_request_tracker(
                        config.rpc_provider.clone(),
                        config.market_address,
                    );

                    (builder, tracker, None, None, None)
                }
            },
            ClientMode::Provider(mode) => match mode {
                ProviderMode::Searcher {
                    search_config,
                    bidding_config,
                    worker_config,
                    resolver_config,
                } => {
                    // Searcher needs request-focused builder and tracker
                    let builder = IntentBuilder::new_bid_builder(
                        config.rpc_provider.clone(),
                        config.signer.address(),
                        config.market_address,
                        bidding_config.clone(),
                    );

                    let tracker = IntentTracker::new_request_tracker(
                        config.rpc_provider.clone(),
                        config.market_address,
                    );

                    let searcher = Some(IntentSearcher::new_request_searcher(
                        search_config.clone(),
                        bidding_config.clone(),
                    ));

                    let worker_manager = Some(WorkerManager::new(worker_config.clone()));
                    let resolver = Some(IntentResolver::new(resolver_config.clone()));

                    (builder, tracker, searcher, worker_manager, resolver)
                },
                ProviderMode::Offering {
                    offer_config,
                    worker_config,
                    resolver_config,
                } => {
                    // Offering needs offer-focused builder and tracker
                    let builder = IntentBuilder::new_offer_builder(
                        config.rpc_provider.clone(),
                        config.signer.address(),
                        config.market_address,
                        offer_config.clone(),
                    );

                    let tracker = IntentTracker::new_offer_tracker(
                        config.rpc_provider.clone(),
                        config.market_address,
                    );

                    let worker_manager = Some(WorkerManager::new(worker_config.clone()));
                    let resolver = Some(IntentResolver::new(resolver_config.clone()));

                    (builder, tracker, None, worker_manager, resolver)
                }
            }
        };

        Self {
            config,
            api,
            builder,
            tracker,
            searcher,
            worker_manager,
            resolver,
        }
    }

    // Mode-specific run methods
    pub async fn run(&self) -> Result<()> {
        match &self.config.role {
            ClientMode::Requester(mode) => match mode {
                RequesterMode::Searcher { .. } => self.run_requester_searcher().await,
                RequesterMode::Requesting { .. } => self.run_requester_requesting().await,
            },
            ClientMode::Provider(mode) => match mode {
                ProviderMode::Searcher { .. } => self.run_provider_searcher().await,
                ProviderMode::Offering { .. } => self.run_provider_offering().await,
            },
        }
    }

    // Requester searcher mode - searches for and accepts offers
    async fn run_requester_searcher(&self) -> Result<()> {
        let searcher = self.searcher.as_ref()
            .ok_or_else(|| Error::InvalidMode("Searcher not configured".into()))?;

        let mut offer_stream = searcher.subscribe_to_offers().await?;
        while let Some(offer) = offer_stream.next().await {
            if searcher.should_accept(&offer)? {
                self.accept_and_track_offer(offer).await?;
            }
        }
        Ok(())
    }

    // Requester requesting mode - creates new requests
    async fn run_requester_requesting(&self) -> Result<()> {
        let request = self.builder.build_request()?;
        let signed_request = self.sign_intent(request).await?;
        self.submit_and_track_intent(signed_request).await
    }

    // Provider searcher mode - searches for and bids on requests
    async fn run_provider_searcher(&self) -> Result<()> {
        let searcher = self.searcher.as_ref()
            .ok_or_else(|| Error::InvalidMode("Searcher not configured".into()))?;

        let mut request_stream = searcher.subscribe_to_requests().await?;
        while let Some(request) = request_stream.next().await {
            if searcher.should_bid(&request)? {
                self.bid_execute_and_resolve(request).await?;
            }
        }
        Ok(())
    }

    // Provider offering mode - creates new offers
    async fn run_provider_offering(&self) -> Result<()> {
        let offer = self.builder.build_offer()?;
        let signed_offer = self.sign_intent(offer).await?;
        self.submit_and_track_intent(signed_offer).await
    }

    // Common utilities used across modes
    async fn sign_intent<I: Intent>(&self, intent: I) -> Result<I> {
        // ... signing logic
    }

    async fn submit_and_track_intent<I: Intent>(&self, intent: I) -> Result<()> {
        // ... submission and tracking logic
    }

    async fn bid_execute_and_resolve(&self, request: ComputeRequest) -> Result<()> {
        // ... bidding, execution and resolution logic
    }

    async fn accept_and_track_offer(&self, offer: ComputeOffer) -> Result<()> {
        // ... offer acceptance and tracking logic
    }
}*/

// Specialized traits for each mode's capabilities
// pub trait IntentBuilder {
//     type Intent;
//     fn build(&self) -> Result<Self::Intent>;
// }

// #[async_trait]
// pub trait IntentAnalyzer {
//     type Intent;
//     async fn analyze(&self, intent: &Self::Intent) -> Result<()>;
// }

// #[async_trait]
// pub trait IntentAuctionTracker {
//     type Intent;
//     async fn track_auction(&self, intent_id: FixedBytes<32>) -> Result<()>;
// }

// #[async_trait]
// pub trait IntentResolveTracker {
//     type Intent;
//     async fn track_resolve(&self, intent_id: FixedBytes<32>) -> Result<()>;
// }

// #[async_trait]
// pub trait IntentSearcher {
//     type Intent;
//     async fn search(&self) -> Result<Vec<Self::Intent>>;
// }

// #[async_trait]
// pub trait IntentStreamer {
//     type IntentStream;
//     async fn stream(&self) -> Result<Self::IntentStream>;
// }

// #[async_trait]
// pub trait IntentResolver {
//     type Intent;
//     async fn resolve_intent(&self, intent: &Self::Intent) -> Result<()>;
// }

/*impl<T, P, N, S> TaralliClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub fn new(config: ClientConfig<T, P, N, S>) -> Result<Self> {
        match &config.role {
            ClientRole::Requester(mode) => match mode {
                RequesterMode::Requesting { request_config } => {
                    Ok(Self::RequesterRequesting(RequesterRequestingClient::new(
                        config,
                        request_config.clone(),
                    )?))
                }
                RequesterMode::Searcher { search_config, acceptance_config } => {
                    Ok(Self::RequesterSearching(RequesterSearchingClient::new(
                        config,
                        search_config.clone(),
                        acceptance_config.clone(),
                    )?))
                }
            },
            ClientRole::Provider(mode) => match mode {
                ProviderMode::Offering {
                    offer_config,
                    worker_config,
                    resolver_config,
                } => {
                    Ok(Self::ProviderOffering(ProviderOfferingClient::new(
                        config,
                        offer_config.clone(),
                        worker_config.clone(),
                        resolver_config.clone(),
                    )?))
                }
                ProviderMode::Searcher {
                    search_config,
                    worker_config,
                    resolver_config,
                    ..
                } => {
                    Ok(Self::ProviderSearching(ProviderSearchingClient::new(
                        config,
                        search_config.clone(),
                        worker_config.clone(),
                        resolver_config.clone(),
                    )?))
                }
            }
        }
    }

    pub async fn run(&self) -> Result<()> {
        match self {
            Self::RequesterRequesting(client) => client.run().await,
            Self::RequesterSearching(client) => client.run().await,
            Self::ProviderOffering(client) => client.run().await,
            Self::ProviderSearching(client) => client.run().await,
        }
    }
}*/

/*// Core client enum to handle different client configurations
pub enum TaralliClient<T, P, N, S, I, C> 
where 
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    RequesterRequesting(RequesterRequestingClient<T, P, N, S>),
    RequesterSearching(RequesterSearchingClient<T, P, N, S, I, C>),
    ProviderOffering(ProviderOfferingClient<T, P, N, S>),
    ProviderStreaming(ProviderStreamingClient<T, P, N, S, I, C>),
    ProviderSearching(ProviderSearchingClient<T, P, N, S, I, C>),
}

// Base client implementation with shared functionality
impl<T, P, N, S, I, C> TaralliClient<T, P, N, S, I, C>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub fn new(config: ClientConfig<T, P, N, S>) -> Result<Self> {
        match &config.mode {
            ClientMode::Requester(mode) => match mode {
                RequesterMode::Requesting { request_config } => {
                    Ok(Self::RequesterRequesting(RequesterRequestingClient::new(
                        // config,
                        // request_config.clone(),
                    )?))
                }
                RequesterMode::Searching { search_config, offer_acceptance_config } => {
                    Ok(Self::RequesterSearching(RequesterSearchingClient::new(
                        config,
                        search_config.clone(),
                        offer_acceptance_config.clone(),
                    )?))
                }
            },
            ClientMode::Provider(mode) => match mode {
                ProviderMode::Offering { offer_config, worker_config, resolver_config } => {
                    Ok(Self::ProviderOffering(ProviderOfferingClient::new(
                        config,
                        offer_config.clone(),
                        worker_config.clone(),
                        resolver_config.clone(),
                    )?))
                }
                ProviderMode::Streaming {bidding_config, worker_config, resolver_config } => {
                    Ok(Self::ProviderStreaming(ProviderStreamingClient::new(
                        config,

                    )))
                }
                ProviderMode::Searching { search_config, bidding_config, worker_config, resolver_config } => {
                    Err(ClientError::ProviderSearchingUnimplemented) // TODO
                },
            }
        }
    }

    // Provider Streaming specific methods
    pub async fn run_streaming(&self) -> Result<()> {
        match self {
            Self::ProviderStreaming(client) => client.run().await,
            _ => Err(ClientError::InvalidMode("Client not in streaming mode".into()))
        }
    }

    // Requester Requesting specific methods
    pub async fn sign_request(&self, request: ComputeRequest<SystemParams>) -> Result<SignedComputeRequest> {
        match self {
            Self::RequesterRequesting(client) => client.sign_request(request).await,
            _ => Err(ClientError::InvalidMode("Client not in requester requesting mode".into()))
        }
    }

    pub async fn submit_and_track_request(&self, signed_request: SignedComputeRequest) -> Result<()> {
        match self {
            Self::RequesterRequesting(client) => client.submit_and_track(signed_request).await,
            _ => Err(ClientError::InvalidMode("Client not in requester requesting mode".into()))
        }
    }

    pub async fn sign_offer(&self, offer: ComputeOffer<SystemParams>) -> Result<SignedComputeOffer> {
        match self {
            Self::ProviderOffering(client) => client.sign_offer(offer).await,
            _ => Err(ClientError::InvalidMode("Client not in provider offering mode".into()))
        }
    }

    pub async fn submit_and_track_offer(&self, signed_offer: SignedComputeOffer) -> Result<()> {
        match self {
            Self::ProviderOffering(client) => client.submit_and_track(signed_offer).await,
            _ => Err(ClientError::InvalidMode("Client not in provider offering mode".into()))
        }
    }

    pub async fn search_offers(&self, params: SearchParams) -> Result<Vec<ComputeOffer<SystemParams>>> {
        match self {
            Self::RequesterSearching(client) => client.search(params).await,
            _ => Err(ClientError::InvalidMode("Client not in requester searching mode".into()))
        }
    }
}*/