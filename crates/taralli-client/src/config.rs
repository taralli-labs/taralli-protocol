use serde::{Deserialize, Serialize};
use taralli_primitives::alloy::{primitives::Address, signers::Signer};
use taralli_primitives::systems::SystemId;
use url::Url;

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct AnalyzerConfig {
//     pub market_address: Address,
//     pub validation_config: RequestValidationConfig,
// }

// impl Default for AnalyzerConfig {
//     fn default() -> Self {
//         Self {
//             market_address: Address::ZERO, // Will be set from config
//             validation_config: RequestValidationConfig::default(),
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct BidderConfig {
//     pub market_address: Address,
//     pub min_bid_delay: u64,
//     pub max_bid_attempts: u32,
// }

// impl Default for BidderConfig {
//     fn default() -> Self {
//         Self {
//             market_address: Address::ZERO, // Will be set from config
//             min_bid_delay: 0,
//             max_bid_attempts: 3,
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ResolverConfig {
//     pub market_address: Address,
//     pub confirmation_blocks: u64,
// }

// impl Default for ResolverConfig {
//     fn default() -> Self {
//         Self {
//             market_address: Address::ZERO, // Will be set from config
//             confirmation_blocks: 1,
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ApiConfig {
//     pub server_url: Url,
//     pub request_timeout: u64,
//     pub max_retries: u32,
// }

// impl Default for ApiConfig {
//     fn default() -> Self {
//         Self {
//             server_url: Url::parse("http://localhost:8080").unwrap(),
//             request_timeout: 30,
//             max_retries: 3,
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct WorkerConfig {
//     pub supported_proving_systems: Vec<SystemId>,
//     pub max_concurrent_jobs: u32,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct RewardTokenConfig {
//     pub address: Address,
//     pub decimal: u8,
// }

// requester client configs
#[derive(Clone)]
pub struct RequesterSearchConfig {}
#[derive(Clone)]
pub struct OfferAcceptanceConfig {}
#[derive(Clone)]
pub struct RequestConfig {}

// provider client configs
#[derive(Clone)]
pub struct ProviderSearchConfig {}
#[derive(Clone)]
pub struct RequestAcceptanceConfig {}
// #[derive(Clone)]
// pub struct StreamConfig {}
#[derive(Clone)]
pub struct OfferConfig{}
#[derive(Clone)]
pub struct WorkerConfig {}
#[derive(Clone)]
pub struct ResolverConfig {}

// Client modes
pub enum ClientMode {
    Requester(RequesterMode),
    Provider(ProviderMode),
}

// Requester-specific modes
pub enum RequesterMode {
    // Search for existing compute offers
    Searching {
        search_config: RequesterSearchConfig,
        offer_acceptance_config: OfferAcceptanceConfig,
    },
    // Create compute requests
    Requesting {
        request_config: RequestConfig,
    },
}

// Provider-specific modes
#[derive(Clone)]
pub enum ProviderMode {
    // Search for compute requests to bid on
    Searching {
        search_config: ProviderSearchConfig,
        bidding_config: RequestAcceptanceConfig,
        worker_config: WorkerConfig,
        resolver_config: ResolverConfig,
    },
    // Process incoming compute requests over ws stream
    Streaming {
        bidding_config: RequestAcceptanceConfig,
        worker_config: WorkerConfig,
        resolver_config: ResolverConfig,
    },
    // Create compute offers
    Offering {
        offer_config: OfferConfig,
        worker_config: WorkerConfig,
        resolver_config: ResolverConfig,
    },
}

/*#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub rpc_provider: P,
    pub signer: S,
    pub server_url: Url,
    pub mode: ClientMode,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N, S> ClientConfig<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub fn new(
        rpc_provider: P,
        signer: S,
        server_url: Url,
        mode: ClientMode
    ) -> Self {
        Self {
            rpc_provider,
            signer,
            server_url,
            mode,
            phantom_data: PhantomData,
        }
    }
}
*/
