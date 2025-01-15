use tokio::sync::broadcast::{self, Receiver};

use crate::error::{Result, ServerError};

// Generic over a Message type M
pub struct SubscriptionManager<M>
where
    M: Clone,
{
    sender: broadcast::Sender<M>,
}

impl<M> SubscriptionManager<M>
where
    M: Clone,
{
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn add_subscription(&self) -> Receiver<M> {
        self.sender.subscribe()
    }

    pub fn active_subscriptions(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Send an event to all the receivers in the broadcast.
    /// Although this function is just a wrapper around `tokio::sync::broadcast::Sender::send` as of now,
    /// in the future we might want to add custom logic to it.
    pub fn broadcast(&self, event: M) -> Result<usize> {
        let subscriber_count = self.active_subscriptions();
        if subscriber_count == 0 {
            tracing::warn!("Attempted to broadcast event but found no active subscribers");
            return Err(ServerError::NoProvidersAvailable());
        }

        match self.sender.send(event) {
            Ok(recv_count) => {
                tracing::info!("Successfully broadcast event to {} receiver(s)", recv_count);
                Ok(recv_count)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to broadcast event to {} subscribers: {}",
                    subscriber_count,
                    e
                );
                Err(ServerError::BroadcastError(e.to_string()))
            }
        }
    }
}

impl<M> Default for SubscriptionManager<M>
where
    M: Clone,
{
    fn default() -> Self {
        Self::new(1)
    }
}
