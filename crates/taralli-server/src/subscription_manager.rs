use std::{collections::HashMap, sync::Arc};

use taralli_primitives::systems::ProvingSystemId;
use tokio::sync::{broadcast, RwLock};

use crate::error::{Result, ServerError};

pub struct SubscriptionManager<M>
where
    M: Clone,
{
    pub channels: Arc<RwLock<HashMap<ProvingSystemId, broadcast::Sender<M>>>>,
    pub capacity: usize,
}

impl<M> SubscriptionManager<M>
where
    M: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            capacity,
        }
    }

    /// avoids needing a write lock for first time use of a system
    pub async fn init_channels(&self, ids: &[ProvingSystemId]) {
        let mut map = self.channels.write().await;
        for &id in ids {
            map.entry(id)
                .or_insert_with(|| broadcast::channel(self.capacity).0);
        }
    }

    /// Retrieve (or create) the channel sender for a given ID.
    pub async fn get_or_create_sender(&self, id: ProvingSystemId) -> broadcast::Sender<M> {
        let mut map = self.channels.write().await;
        map.entry(id)
            .or_insert_with(|| broadcast::channel(self.capacity).0)
            .clone()
    }

    /// Subscribe to the channel for a given ID (read only).
    pub async fn subscribe_to_id(&self, id: ProvingSystemId) -> broadcast::Receiver<M> {
        let sender = self.get_or_create_sender(id).await;
        sender.subscribe()
    }

    /// Get multiple Receivers if you want multi-ID SSE.
    pub async fn subscribe_to_ids(&self, ids: &[ProvingSystemId]) -> Vec<broadcast::Receiver<M>> {
        let mut receivers = Vec::with_capacity(ids.len());
        for &id in ids {
            //receivers.push(self.subscribe_to_id(id).await);
            println!("Creating subscriber for system ID: {:?}", id); // Debug
            let sender = self.get_or_create_sender(id).await;
            println!("Sender has {} receivers", sender.receiver_count()); // Debug
            receivers.push(sender.subscribe());
            println!("New receiver count: {}", sender.receiver_count()); // Debug
        }
        receivers
    }

    /// Broadcast an event for a single ID
    pub async fn broadcast(&self, id: ProvingSystemId, event: M) -> Result<usize> {
        let map = self.channels.read().await;
        let sender = match map.get(&id) {
            Some(sender) => sender.clone(),
            None => {
                return Err(ServerError::BroadcastError(format!(
                    "No channel found for ID {:?}",
                    id
                )))
            }
        };
        drop(map);

        let subs = sender.receiver_count();
        if subs == 0 {
            return Err(ServerError::BroadcastError(format!(
                "No subscribers for ID {:?}",
                id
            )));
        }
        sender
            .send(event)
            .map_err(|e| ServerError::BroadcastError(e.to_string()))
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
