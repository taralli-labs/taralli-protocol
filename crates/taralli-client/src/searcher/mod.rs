use async_trait::async_trait;

use crate::error::Result;

pub mod offer;

/// core searcher trait used across all compute intent markets
#[async_trait]
pub trait IntentSearcher {
    type Intent;
    async fn search(&self) -> Result<Self::Intent>;
}
