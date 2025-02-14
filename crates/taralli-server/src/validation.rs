use crate::app_state::AppState;
use crate::error::{Result, ServerError};
use axum::extract::State;
use std::time::Duration;
use taralli_primitives::alloy::{
    eips::BlockId,
    network::{BlockTransactionsKind, Ethereum},
    providers::Provider,
    transports::Transport,
};
use taralli_primitives::validation::validate_partial_request;
use taralli_primitives::PartialRequest;
use tokio::time::timeout;

pub async fn validate_proof_request<T, P>(
    request: &PartialRequest,
    app_state: &State<AppState<T, P>>,
    timeout_seconds: Duration,
) -> Result<()>
where
    T: Transport + Clone,
    P: Provider<T> + Clone,
{
    // TODO: remove this async process from the validation execution of the server and use input parameter instead
    let latest_timestamp = get_latest_timestamp(app_state.rpc_provider()).await?;

    timeout(timeout_seconds, async {
        validate_partial_request(
            request,
            latest_timestamp,
            &app_state.market_address(),
            app_state.minimum_allowed_proving_time(),
            app_state.maximum_allowed_start_delay(),
            app_state.maximum_allowed_stake(),
        )?;
        Ok(())
    })
    .await
    .map_err(|_| ServerError::ValidationTimeout(timeout_seconds.as_secs()))?
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
