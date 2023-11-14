//! This module provides a high-level API for interacting with the variable hub registry
//! by abstacting the low-level API details via easy to use rust types.

use std::sync::Arc;

use futures::{Stream, StreamExt};
use thiserror::Error;

use crate::authenticated_nats_con::{AuthenticatedNatsConnection, AuthenticationSettings};

use super::nats_consumer::{self, NatsConsumer};

/// Error type for the data hub consumer
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("{0}")]
    NatsConnection(#[from] async_nats::Error),
    #[error("{0}")]
    LowLevelApi(#[from] nats_consumer::Error),
}

/// Result type for the data hub consumer
pub type Result<T> = std::result::Result<T, Error>;

/// A high-level API for interacting with the variable hub registry
/// by abstacting the low-level API details via easy to use rust types.
pub struct DataHubConsumer {
    nats_consumer: Arc<NatsConsumer>,
}

impl DataHubConsumer {
    /// Tries to connect and authenticate to the provided NATS address and initializes the consumer.
    ///
    /// See documentation of [`AuthenticatedNatsConnection`] for more details on the connection process.
    pub async fn connect(
        nats_server_addr: impl Into<String>,
        auth_settings: &AuthenticationSettings,
    ) -> Result<Self> {
        let auth_nats_con =
            Arc::new(AuthenticatedNatsConnection::new(nats_server_addr, auth_settings).await?);

        Self::from_existing_connection(auth_nats_con).await
    }

    /// Creates a new data hub consumer from an existing nats connection.
    ///
    /// This is useful if you want to use the same connection for multiple clients.
    pub async fn from_existing_connection(
        nats_con: Arc<AuthenticatedNatsConnection>,
    ) -> Result<Self> {
        let nats_consumer = Arc::new(NatsConsumer::new(nats_con).await?);
        Ok(Self { nats_consumer })
    }

    /// Allows access to low level api
    pub fn get_nats_consumer(&self) -> &Arc<NatsConsumer> {
        &self.nats_consumer
    }

    /// Returns a list of all registered provider IDs.
    ///
    /// This method issues a NATS request and will fail if the hub registry is currently offline.
    /// It may also return an error if the flatbuffer deserialization fails.
    pub async fn read_provider_ids(&self) -> Result<Vec<String>> {
        let low_level_data = self.nats_consumer.read_provider_ids().await?;

        let mapped_result = low_level_data
            .providers
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|provider| provider.id)
            .collect();

        Ok(mapped_result)
    }

    /// Returns a stream of provider ID changes on the registry.
    /// Each time a provider is added or removed on the registry, the stream will yield a new list of provider IDs.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection.
    ///
    /// Internally uses the low level api to receive the values.
    /// Each received value from the low level api will be converted to an easy to use rust type.
    /// If the low level api stream returned an error value, this value will be silenly ignored, but the subscription will not be cancelled.
    pub async fn subscribe_provider_ids(&self) -> Result<impl Stream<Item = Vec<String>>> {
        let low_level_data = self.nats_consumer.subscribe_provider_ids().await?;

        let mapped_result = low_level_data.filter_map(move |prov_changed_evt| async move {
            //simply ignore invalid events for high level api
            let prov_changed_evt = prov_changed_evt.ok()?;

            prov_changed_evt.providers.items.map(|items| {
                items
                    .into_iter()
                    .map(|provider| provider.id)
                    .collect::<Vec<_>>()
            })
        });

        Ok(Box::pin(mapped_result))
    }

    /// Waits until the specified provider ID is available on the registry and contains a valid definition.
    ///
    /// There is no internal timeout, but you may wrap this within a timeout call.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection.
    pub async fn wait_for_provider(&self, provider_id: &str) -> Result<()> {
        Ok(self
            .get_nats_consumer()
            .wait_for_provider(provider_id)
            .await?)
    }
}
