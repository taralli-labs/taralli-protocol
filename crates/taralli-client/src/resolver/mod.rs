use crate::error::Result;
use async_trait::async_trait;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};

use taralli_primitives::alloy::network::Network;

pub mod offer;
pub mod request;

/// core resolver trait
#[async_trait]
pub trait IntentResolver<N: Network> {
    type Intent;
    async fn resolve_intent(
        &self,
        intent_id: FixedBytes<32>,
        opaque_submission: Bytes,
    ) -> Result<N::ReceiptResponse>;
}
