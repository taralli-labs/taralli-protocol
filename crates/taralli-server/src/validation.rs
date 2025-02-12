use crate::{
    error::{Result, ServerError},
    state::BaseState,
};
use taralli_primitives::alloy::{
    eips::BlockId,
    network::{BlockTransactionsKind, Ethereum},
    providers::Provider,
    transports::Transport,
};
use taralli_primitives::validation::{FromMetaConfig, Validate};
use tokio::time::timeout;

pub async fn validate_intent<T: Transport + Clone, P: Provider<T> + Clone, I>(
    intent: &I,
    app_state: &BaseState<T, P>,
) -> Result<()>
where
    I: Validate,
    I::Config: FromMetaConfig,
{
    // TODO: separate this timestamp fetch from the validation execution of the server
    let latest_timestamp = get_latest_timestamp(app_state.rpc_provider()).await?;
    let validation_timeout_seconds = app_state.validation_timeout_seconds();
    let intent_validation_config = I::Config::from_meta(app_state.validation_config());

    timeout(validation_timeout_seconds, async {
        intent.validate(
            latest_timestamp,
            &app_state.market_address(),
            &intent_validation_config,
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
