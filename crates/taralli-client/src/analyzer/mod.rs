use async_trait::async_trait;

use crate::error::Result;

pub mod offer;
pub mod request;

/// core analyzer trait
#[async_trait]
pub trait IntentAnalyzer {
    type Intent;
    async fn analyze(&self, latest_ts: u64, intent: &Self::Intent) -> Result<()>;
}
