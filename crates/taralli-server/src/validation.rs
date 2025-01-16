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
use taralli_primitives::taralli_systems::traits::ProvingSystemInformation;
use taralli_primitives::validation::{
    validate_amount_constraints, validate_market_address, validate_proving_system_id,
    validate_signature, validate_time_constraints,
};
use taralli_primitives::Request;
use tokio::time::timeout;

pub async fn validate_proof_request<T, P, I>(
    request: &Request<I>,
    app_state: &State<AppState<T, P, Request<I>>>,
    minimum_allowed_proving_time: u32,
    maximum_start_delay: u32,
    maximum_allowed_stake: u128,
    timeout_seconds: Duration,
) -> Result<()>
where
    T: Transport + Clone,
    P: Provider<T> + Clone,
    I: ProvingSystemInformation + Clone,
{
    // TODO: remove this async process from the validation execution of the server and use input parameter instead
    let latest_timestamp = get_latest_timestamp(app_state.rpc_provider()).await?;

    timeout(timeout_seconds, async {
        validate_proving_system_id(request, app_state.proving_system_ids())?;
        validate_market_address(request, app_state.market_address())?;
        validate_amount_constraints(maximum_allowed_stake, request)?;
        validate_time_constraints(
            latest_timestamp,
            minimum_allowed_proving_time,
            maximum_start_delay,
            request,
        )?;
        validate_signature(request)?;
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
