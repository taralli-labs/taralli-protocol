use std::any::TypeId;

use crate::{
    config::{ServerConfigProvider, ServerValidationConfigs}, error::{Result, ServerError}, state::BaseState
};
use taralli_primitives::{alloy::{
    eips::BlockId,
    network::{BlockTransactionsKind, Ethereum},
    providers::Provider,
    transports::Transport,
}, intents::{ComputeIntent, ComputeOffer, ComputeRequest}, systems::SystemParams, validation::CommonValidationConfig};
use taralli_primitives::validation::Validate;
use tokio::time::timeout;

// Helper trait to get config from ValidationConfigs
pub trait ValidationConfigProvider {
    fn get_config_for_type_id(&self, type_id: TypeId) -> Option<&dyn CommonValidationConfig>;
}

// Implement for your ValidationConfigs struct
impl ValidationConfigProvider for ServerValidationConfigs {
    fn get_config_for_type_id(&self, type_id: TypeId) -> Option<&dyn CommonValidationConfig> {
        match type_id {
            t if t == TypeId::of::<ComputeRequest<SystemParams>>() => Some(&self.request),
            t if t == TypeId::of::<ComputeOffer<SystemParams>>() => Some(&self.offer),
            _ => None
        }
    }
}

pub async fn validate_intent<T: Transport + Clone, P: Provider<T> + Clone, I>(
    intent: &I,
    state: &BaseState<T, P>,
) -> Result<()> 
where
    I: ComputeIntent + Validate + ServerConfigProvider,
    I::Config: CommonValidationConfig
{
    // TODO: separate this timestamp fetch from the validation execution of the server
    let latest_timestamp = get_latest_timestamp(state.rpc_provider()).await?;
    let validation_timeout_seconds = state.validation_timeout_seconds();
    //let intent_validation_config = I::Config::from_meta(app_state.validation_config());

    let config = I::get_config(state.validation_configs());

    timeout(validation_timeout_seconds, async {
        intent.validate(
            latest_timestamp,
            &state.market_address(),
            config
        )
    })
    .await
    .map_err(|_| ServerError::ValidationTimeout(validation_timeout_seconds.as_secs()))?
    .map_err(ServerError::from)
}

async fn get_latest_timestamp<P: Provider<T, Ethereum> + Clone, T: Transport + Clone>(
    provider: P,
) -> Result<u64> {
    provider
        .get_block(BlockId::latest(), BlockTransactionsKind::Hashes)
        .await
        .map_err(|_| ServerError::FetchLatestBlockTimestampError)?
        .map(|block| block.header.timestamp)
        .ok_or(ServerError::FetchLatestBlockTimestampError)
}
