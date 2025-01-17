use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, EventSource};
use std::pin::Pin;
use taralli_primitives::{taralli_systems::id::ProvingSystemParams, Request};
use url::Url;

use crate::{
    config::ApiConfig,
    error::{ProviderError, Result},
};

pub struct ProviderApi {
    server_url: Url,
}

type StreamResult =
    Result<Pin<Box<dyn Stream<Item = Result<Request<ProvingSystemParams>>> + Send>>>;

impl ProviderApi {
    pub fn new(config: ApiConfig) -> Self {
        Self {
            server_url: config.server_url,
        }
    }

    pub fn subscribe_to_markets(&self) -> StreamResult {
        // Attempt to join the URL, log error if it fails
        let url = self
            .server_url
            .join("/subscribe")
            .map_err(|e| ProviderError::ServerSubscriptionError(e.to_string()))?;

        // Attempt to create the EventSource, log error if it fails
        let event_source = EventSource::get(url);

        Ok(Box::pin(event_source.filter_map(|event| async move {
            match event {
                Ok(Event::Message(message)) => {
                    match serde_json::from_str::<Request<ProvingSystemParams>>(&message.data) {
                        Ok(proof_request) => Some(Ok(proof_request)),
                        Err(e) => Some(Err(ProviderError::RequestParsingError(format!(
                            "Failed to parse proof request from incoming event: {}",
                            e
                        )))),
                    }
                }
                Ok(Event::Open) => {
                    // Log when the connection is successfully established (optional)
                    tracing::debug!("Connected to /subscribe endpoint");
                    None
                }
                Err(e) => Some(Err(ProviderError::RequestParsingError(format!(
                    "EventSource encountered an error: {}",
                    e
                )))),
            }
        })))
    }
}
