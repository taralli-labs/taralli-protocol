use crate::{
    config::ServerValidationConfigProvider,
    error::{Result, ServerError},
    state::BaseState,
};
use taralli_primitives::{
    alloy::{
        eips::BlockId,
        network::{BlockTransactionsKind, Ethereum},
        providers::Provider,
        transports::Transport,
    },
    intents::ComputeIntent,
    validation::CommonValidationConfig,
};
use tokio::time::timeout;

pub async fn validate_intent<T: Transport + Clone, P: Provider<T> + Clone, I>(
    intent: &I,
    state: &BaseState<T, P>,
) -> Result<()>
where
    I: ComputeIntent + ServerValidationConfigProvider,
    I::Config: CommonValidationConfig,
{
    // TODO: separate this timestamp fetch from the validation execution of the server
    let latest_timestamp = get_latest_timestamp(state.rpc_provider()).await?;
    let validation_timeout_seconds = state.validation_timeout_seconds();
    let config = I::get_config(state.validation_configs());

    timeout(validation_timeout_seconds, async {
        intent.validate(latest_timestamp, &state.market_address(), config)
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
