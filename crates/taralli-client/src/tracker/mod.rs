use async_trait::async_trait;
use std::time::Duration;
use taralli_primitives::alloy::primitives::FixedBytes;

use crate::error::Result;

pub mod offer;
pub mod request;

#[async_trait]
pub trait IntentAuctionTracker {
    type Intent;
    type BidEvent;
    async fn track_auction(
        &self,
        intent_id: FixedBytes<32>,
        timeout: Duration,
    ) -> Result<Option<Self::BidEvent>>;
}

#[async_trait]
pub trait IntentResolveTracker {
    type Intent;
    type ResolveEvent;
    async fn track_resolve(
        &self,
        intent_id: FixedBytes<32>,
        timeout: Duration,
    ) -> Result<Option<Self::ResolveEvent>>;
}
