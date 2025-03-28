use taralli_primitives::{env::Environment, systems::SystemIdMask};
use tokio::sync::broadcast::{self, Receiver};

use crate::error::{Result, ServerError};

#[derive(Clone)]
/// A wrapper type for the message that is broadcasted to all subscribers.
/// content: The serialized compute request, with system information being compressed.
/// `subscribed_to`: The system id that the compute request is related to. See `systems` macro in primitives.
pub struct BroadcastedMessage {
    pub content: Vec<u8>,
    pub subscribed_to: SystemIdMask,
}

// Generic over a Message type M
// Todo: Remove generic and use only Vec<u8> when removing propagation of Request<ProvingSystemParams> through SSE.
pub struct SubscriptionManager<M = BroadcastedMessage>
where
    M: Clone,
{
    sender: broadcast::Sender<M>,
}

impl<M> SubscriptionManager<M>
where
    M: Clone,
{
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    #[must_use]
    pub fn buffer_len(&self) -> usize {
        self.sender.len()
    }

    #[must_use]
    pub fn add_subscription(&self) -> Receiver<M> {
        self.sender.subscribe()
    }

    #[must_use]
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
        let lag = std::env::var("SERVER_SUBSCRIPTION_LAG");
        match Environment::from_env_var() {
            Environment::Production => Self::new(
                lag.expect("Must specify SERVER_SUBSCRIPTION_LAG in production")
                    .parse::<usize>()
                    .expect("Failed to parse SERVER_SUBSCRIPTION_LAG"),
            ),
            // For development, any "bigger" value is acceptable.
            // Some tests rely on having at least 1-2-10 of lag.
            Environment::Development => Self::new(
                lag.unwrap_or_else(|_| "100".to_string())
                    .parse::<usize>()
                    .unwrap_or(100),
            ),
        }
    }
}
