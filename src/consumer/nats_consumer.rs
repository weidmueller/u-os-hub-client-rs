//! This module provides a low-level API for interacting with the variable hub registry via NATS.

use std::sync::Arc;

use bytes::Bytes;
use flatbuffers::FlatBufferBuilder;
use futures::{Stream, StreamExt};
use thiserror::Error;

use crate::{
    authenticated_nats_con::AuthenticatedNatsConnection,
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionState, ProvidersChangedEvent, ProvidersChangedEventT,
        ReadProviderDefinitionQueryRequestT, ReadProvidersQueryResponse,
        ReadProvidersQueryResponseT, StateChangedEvent, StateChangedEventT,
    },
    nats_subjects,
};

use super::connected_nats_provider::{self, ConnectedNatsProvider};

#[derive(Error, Debug)]
pub enum Error {
    #[error("NATS Request Error: {0}")]
    NatsRequest(#[from] async_nats::RequestError),
    #[error("NATS Subscribe Error: {0}")]
    NatsSub(#[from] async_nats::SubscribeError),
    #[error("Invalid payload/deserialization failure: {0}")]
    InvalidPayload(#[from] flatbuffers::InvalidFlatbuffer),
    #[error("Failed to wait for provider: {0}")]
    FailedToWaitForProvider(#[from] connected_nats_provider::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Low Level API for nats and data hub registry abstraction.
/// Primarily handles flatbuffer serialization and abstracts nats paths.
pub struct NatsConsumer {
    nats_con: Arc<AuthenticatedNatsConnection>,
}

impl NatsConsumer {
    /// Creates a consumer object using an existing nats connection.
    pub async fn new(nats_con: Arc<AuthenticatedNatsConnection>) -> Result<Self> {
        Ok(Self { nats_con })
    }

    /// Gets the nats connection object.
    pub fn get_nats_con(&self) -> &Arc<AuthenticatedNatsConnection> {
        &self.nats_con
    }

    /// Subscribes to changes to registry state and returns a stream of change events.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection.
    ///
    /// While subscribed, this method will try to deserialize the flatbuffer payloads that are received via NATS.
    /// If the flatbuffer deserialization fails, the stream will yield an error value but the subscription will not be cancelled.
    pub async fn subscribe_registry_state(
        &self,
    ) -> Result<impl Stream<Item = Result<StateChangedEventT>>> {
        let subscription = self
            .nats_con
            .get_client()
            .subscribe(nats_subjects::registry_state_changed_event())
            .await?;

        let result_stream = subscription.map(|message| -> Result<StateChangedEventT> {
            let payload = flatbuffers::root::<StateChangedEvent>(&message.payload)?;
            Ok(payload.unpack())
        });

        Ok(result_stream)
    }

    /// Returns a list of all registered provider IDs.
    ///
    /// This method issues a NATS request and will fail if the hub registry is currently offline.
    /// It may also return an error if the flatbuffer deserialization fails.
    pub async fn read_provider_ids(&self) -> Result<ReadProvidersQueryResponseT> {
        //Create flatbuffer read request payload
        let mut builder = FlatBufferBuilder::new();
        let request_payload = ReadProviderDefinitionQueryRequestT {};
        let offset = request_payload.pack(&mut builder);
        builder.finish(offset, None);

        //use collapse to avoid copying vector
        let (all_bytes, data_start_offset) = builder.collapse();
        let request_bytes = Bytes::from(all_bytes).slice(data_start_offset..);

        let reply = self
            .nats_con
            .get_client()
            .request(
                nats_subjects::registry_providers_read_query(),
                request_bytes,
            )
            .await?;

        let payload = flatbuffers::root::<ReadProvidersQueryResponse>(&reply.payload)?;

        Ok(payload.unpack())
    }

    /// Subscribes to changes to the registered providers on the hub registry and returns a stream of change events.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection.
    ///
    /// While subscribed, this method will try to deserialize the flatbuffer payloads that are received via NATS.
    /// If the flatbuffer deserialization fails, the stream will yield an error value but the subscription will not be cancelled.
    pub async fn subscribe_provider_ids(
        &self,
    ) -> Result<impl Stream<Item = Result<ProvidersChangedEventT>>> {
        let subscription = self
            .nats_con
            .get_client()
            .subscribe(nats_subjects::registry_providers_changed_event())
            .await?;

        let result_stream = subscription.map(|message| -> Result<ProvidersChangedEventT> {
            let payload = flatbuffers::root::<ProvidersChangedEvent>(&message.payload)?;
            Ok(payload.unpack())
        });

        Ok(result_stream)
    }

    /// Waits until the specified provider ID is available on the registry and contains a valid definition.
    ///
    /// There is no internal timeout, but you may wrap this within a tokio timeout call.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection.
    pub async fn wait_for_provider(&self, provider_id: &str) -> Result<()> {
        let nats_client = self.get_nats_con().get_client();

        //Must sub before reading, otherwise we could miss a registration
        let mut provider_def_evt_stream =
            ConnectedNatsProvider::subscribe_provider_definition_internal(nats_client, provider_id)
                .await?;

        //Get current provider definition
        let provider_def_resp =
            ConnectedNatsProvider::read_provider_definition_internal(nats_client, provider_id)
                .await;

        //Check if provider is already valid
        if let Ok(provider_def_resp) = &provider_def_resp {
            if let Some(provider_def) = &provider_def_resp.provider_definition {
                if provider_def.state == ProviderDefinitionState::OK {
                    return Ok(());
                }
            }
        }

        //process provider def changed events until we find a valid one
        while let Some(provider_def_evt) = provider_def_evt_stream.next().await {
            if let Ok(provider_def_evt) = &provider_def_evt {
                if let Some(provider_def) = &provider_def_evt.provider_definition {
                    if provider_def.state == ProviderDefinitionState::OK {
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }
}
