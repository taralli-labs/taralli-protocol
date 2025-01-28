use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, EventSource};
use std::pin::Pin;
use taralli_primitives::{request::ComputeRequest, systems::ProvingSystemParams};
use url::Url;

use crate::{
    config::ApiConfig,
    error::{ProviderError, Result},
};

pub struct ProviderApi {
    server_url: Url,
}

// type alias for SSE stream returned by the protocol server
pub type RequestStream =
    Pin<Box<dyn Stream<Item = Result<ComputeRequest<ProvingSystemParams>>> + Send>>;

impl ProviderApi {
    pub fn new(config: ApiConfig) -> Self {
        Self {
            server_url: config.server_url,
        }
    }

    pub fn subscribe_to_markets(&self) -> Result<RequestStream> {
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
                    match serde_json::from_str::<ComputeRequest<ProvingSystemParams>>(&message.data)
                    {
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
