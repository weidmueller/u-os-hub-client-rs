//! This module provides a low-level API for interacting with a variable hub provider via NATS.

use std::sync::{Arc, RwLock};

use bytes::Bytes;
use flatbuffers::FlatBufferBuilder;
use futures::{Stream, StreamExt};
use rustc_hash::FxHashMap;
use thiserror::Error;
use tokio::task::JoinHandle;
use tracing::{error, warn};

use crate::{generated::weidmueller::ucontrol::hub::*, nats_subjects};

use super::{
    nats_consumer::NatsConsumer,
    variable_key::{VariableKey, VariableKeyHash},
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("NATS Flush Error: {0}")]
    NatsFlush(#[from] async_nats::client::FlushError),
    #[error("NATS Request Error: {0}")]
    NatsRequest(#[from] async_nats::RequestError),
    #[error("NATS Publish Error: {0}")]
    NatsPublish(#[from] async_nats::PublishError),
    #[error("NATS Subscribe Error: {0}")]
    NatsSub(#[from] async_nats::SubscribeError),
    #[error("Invalid payload/deserialization failure: {0}")]
    InvalidPayload(#[from] flatbuffers::InvalidFlatbuffer),
    #[error("Provider fingerprint mismatch. Expected: {expected}, Actual: {actual}")]
    ProviderFingerprintMismatch { expected: u64, actual: u64 },
    #[error("Variable ID {0} is unknown")]
    InvalidVariableId(VariableID),
    #[error("Variable with key '{0}' not found")]
    InvalidVariableKey(String),
    #[error("Variable with key '{0}' does not allow writing")]
    NotPermitted(String),
    #[error("Invalid variable value type")]
    InvalidValueType,
    #[error("The provider '{0}' is currently offline or has invalid state")]
    ProviderOfflineOrInvalid(String),
}

/// Shared state for the connected provider.
///
/// We use an RwLock here as provider definition changes (and therefor write operations) should be quite rare,
/// and the RwLock allows parallel reads without contention.
pub(super) type SharedState = Arc<RwLock<ConnectedNatsProviderState>>;

pub type Result<T> = std::result::Result<T, Error>;

/// Represents a variable ID on the hub.
pub type VariableID = u32;

/// Internal state of the provider connection.
///
/// This is used to cache the provider definition and variable definitions.
/// It is updated by the internal event loop when new provider definition events are received.
///
/// We use FxHash for lookup tables because rusts default hash algorithm is quite slow for integer keys. See FxHash crate docs for more info.
#[derive(Debug)]
pub(super) struct ConnectedNatsProviderState {
    /// The current fingerprint of the provider definition.
    /// Will be set to `None` if the provider is offline or the definition is invalid.
    pub(super) cur_fingerprint: Option<u64>,
    ///Maps from variable ID to variable definition
    pub(super) cur_variable_defs: FxHashMap<VariableID, VariableDefinitionT>,
    ///Maps from hashed variable key to variable ID
    pub(super) var_mapping: FxHashMap<VariableKeyHash, VariableID>,
}

/// Represents a connection to a data hub provider.
/// This is used to read and write variables from/to the provider.
///
/// Internally uses an event loop to constantly synch with the provider definition.
/// This allows proper error handling when interating with the provider.
pub struct ConnectedNatsProvider {
    provider_id: String,
    consumer: Arc<NatsConsumer>,
    event_loop_task: JoinHandle<()>,
    state: SharedState,
}

impl ConnectedNatsProvider {
    /// Tries to connect to a provider.
    ///
    /// This will request the provider definition from the registry and build mapping tables for fast access to variable IDs and keys.
    /// Internally, this will start an event loop that will update the provider definition when it changes.
    ///
    /// This method may fail if there is an issue with the nats connection, the hub registry is offline
    /// or something goes wrong while deserializing flatbuffer payloads.
    pub async fn new(consumer: Arc<NatsConsumer>, provider_id: impl Into<String>) -> Result<Self> {
        let provider_id: String = provider_id.into();

        //Initialize state
        let nats_client = consumer.get_nats_con().get_client();

        //This will automatically fail if the provider ID doesnt exist, so we dont need additional error handling
        let provider_def_read_resp =
            Self::read_provider_definition_internal(nats_client, &provider_id)
                .await
                .map_err(|_| Error::ProviderOfflineOrInvalid(provider_id.clone()))?;

        let provider_def = provider_def_read_resp
            .provider_definition
            .ok_or(Error::ProviderOfflineOrInvalid(provider_id.clone()))?;

        if provider_def.state != ProviderDefinitionState::OK {
            return Err(Error::ProviderOfflineOrInvalid(provider_id.clone()));
        }

        let cur_fingerprint = provider_def.fingerprint;
        let cur_variable_defs = Self::variable_defs_from_provider_def(*provider_def);
        let var_mapping = Self::key_mapping_from_variable_defs(&cur_variable_defs);

        let state = Arc::new(RwLock::new(ConnectedNatsProviderState {
            cur_fingerprint: Some(cur_fingerprint),
            cur_variable_defs,
            var_mapping,
        }));

        //Start internal event loop
        let event_stream =
            Self::subscribe_provider_definition_internal(nats_client, &provider_id).await?;

        let event_loop_task = tokio::spawn(Self::internal_event_loop(event_stream, state.clone()));

        //Create new instance
        let instance = Self {
            provider_id,
            consumer,
            state,
            event_loop_task,
        };

        Ok(instance)
    }

    /// Gets the linked nats consumer.
    pub fn get_consumer(&self) -> &Arc<NatsConsumer> {
        &self.consumer
    }

    /// Returns the provider ID.
    pub fn get_provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Returns the cached provider fingerprint.
    /// If the provider is offline or invalid, this will return None.
    ///
    /// The cached value will be updated internally once the provider definition changes.
    pub fn get_fingerprint(&self) -> Option<u64> {
        self.state.read().unwrap().cur_fingerprint
    }

    /// Returns if the provider is online and has a valid definition.
    pub fn is_online(&self) -> bool {
        self.get_fingerprint().is_some()
    }

    /// Returns the cached list of all currently registered variable IDs for this provider.
    ///
    /// The cached value will be updated internally once the provider definition changes.
    pub fn get_variable_ids(&self) -> Vec<VariableID> {
        self.state
            .read()
            .unwrap()
            .cur_variable_defs
            .keys()
            .copied()
            .collect()
    }

    /// Returns the cached variable definition for this variable ID.
    ///
    /// Will fail if the variable ID is unknown.
    /// The cached value will be updated internally once the provider definition changes.
    pub fn get_variable_definition(&self, id: VariableID) -> Result<VariableDefinitionT> {
        self.state
            .read()
            .unwrap()
            .cur_variable_defs
            .get(&id)
            .ok_or(Error::InvalidVariableId(id))
            .cloned()
    }

    /// Returns a cached list of all variable IDs and their corresponding definition for this provider.
    ///
    /// The cached value will be updated internally once the provider definition changes.
    pub fn get_all_variable_definitions(&self) -> FxHashMap<VariableID, VariableDefinitionT> {
        self.state.read().unwrap().cur_variable_defs.clone()
    }

    /// Converts a variable key to a variable ID.
    ///
    /// Please note that variable IDs and their mapping to keys may change at any time if the provider definition changes.
    ///
    /// Will fail if the variable key is unknown.
    pub fn variable_id_from_key<'a>(&self, key: impl Into<VariableKey<'a>>) -> Result<VariableID> {
        let key: VariableKey = key.into();

        let state = self.state.read().unwrap();
        let id = *state
            .var_mapping
            .get(&key.key_hash)
            .ok_or(Error::InvalidVariableKey(key.to_string()))?;
        Ok(id)
    }

    /// Returns the variable key string belonging to the specified variable ID.
    ///
    /// Will fail if the variable ID is unknown.
    pub fn variable_key_from_id(&self, id: VariableID) -> Result<String> {
        let state = self.state.read().unwrap();

        let key = state
            .cur_variable_defs
            .get(&id)
            .ok_or(Error::InvalidVariableId(id))?
            .key
            .clone();
        Ok(key)
    }

    /// Checks if the specified variable ID exists on the provider
    pub fn is_variable_id_valid(&self, id: VariableID) -> bool {
        self.state
            .read()
            .unwrap()
            .cur_variable_defs
            //lookup should be very quick as we only lookup an integer in a hashset
            .contains_key(&id)
    }

    /// Reads the provider definition from the registry.
    ///
    /// This method may fail if there is an issue with the nats connection, the hub registry is offline
    /// or something goes wrong while deserializing flatbuffer payloads.
    pub async fn read_provider_definition(&self) -> Result<ReadProviderDefinitionQueryResponseT> {
        self.check_online()?;
        Self::read_provider_definition_internal(self.get_nats_client(), &self.provider_id).await
    }

    /// Subscribes to changes to the provider definition on the hub registry and returns a stream of change events.
    ///
    /// The event may contain an empty provider definition if the provider was removed from the registry.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection or the provider is no longer available.
    ///
    /// While subscribed, this method will try to deserialize the flatbuffer payloads that are received via NATS.
    /// If the flatbuffer deserialization fails, the stream will yield an error value but the subscription will not be cancelled.
    pub async fn subscribe_provider_definition(
        &self,
    ) -> Result<impl Stream<Item = Result<ProviderDefinitionChangedEventT>>> {
        Self::subscribe_provider_definition_internal(self.get_nats_client(), &self.provider_id)
            .await
    }

    /// Reads the current state of the provided variable IDs from the provider.
    ///
    /// Will check if all supplied variable IDs are still valid before sending the read request.
    ///
    /// This method may fail if there is an issue with the nats connection, the provider is unavailable
    /// or something goes wrong while deserializing flatbuffer payloads.
    pub async fn read_variables(
        &self,
        variable_ids: &ReadVariablesQueryRequestT,
    ) -> Result<ReadVariablesQueryResponseT> {
        self.check_online()?;

        //check if all variable IDs are valid
        if let Some(var_ids) = &variable_ids.ids {
            for var_id in var_ids {
                if !self.is_variable_id_valid(*var_id) {
                    return Err(Error::InvalidVariableId(*var_id));
                }
            }
        }

        self.read_variables_unchecked(variable_ids).await
    }

    /// Same as `read_variables`, but does not check if the variable IDs are valid
    /// before sending the request.
    ///
    /// This can be more performant, but the user must ensure that all variable IDs
    /// in the request are still valid for the provider.
    pub async fn read_variables_unchecked(
        &self,
        variable_ids: &ReadVariablesQueryRequestT,
    ) -> Result<ReadVariablesQueryResponseT> {
        //Create flatbuffer read request payload
        let mut builder = FlatBufferBuilder::new();
        let offset = variable_ids.pack(&mut builder);
        builder.finish(offset, None);

        //use collapse to avoid copying vector
        let (all_bytes, data_start_offset) = builder.collapse();
        let request_bytes = Bytes::from(all_bytes).slice(data_start_offset..);

        //Send read request
        let reply = self
            .get_nats_client()
            .request(
                nats_subjects::read_variables_query(&self.provider_id),
                request_bytes,
            )
            .await?;

        //Deserialize reply
        let reply_payload = flatbuffers::root::<ReadVariablesQueryResponse>(&reply.payload)?;
        Ok(reply_payload.unpack())
    }

    /// Subscribes to changes of the variable states of the provider and returns a stream of change events.
    ///
    /// Does not support filtering, as the provider will always send all changed variables in this event.
    /// If filtering is desired, it must be implemented on the receiver side.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection or the provider is no longer available.
    ///
    /// While subscribed, this method will try to deserialize the flatbuffer payloads that are received via NATS.
    /// If the flatbuffer deserialization fails, the stream will yield an error value but the subscription will not be cancelled.
    pub async fn subscribe_variables(
        &self,
    ) -> Result<impl Stream<Item = Result<VariablesChangedEventT>>> {
        let subscription = self
            .get_nats_client()
            .subscribe(nats_subjects::vars_changed_event(&self.provider_id))
            .await?;

        let result_stream = subscription.map(|message| -> Result<VariablesChangedEventT> {
            let payload = flatbuffers::root::<VariablesChangedEvent>(&message.payload)?;
            Ok(payload.unpack())
        });

        Ok(result_stream)
    }

    /// Same as `write_variables`, but does not check variable IDs, permissions or value types.
    ///
    /// This can be more performant, but the user must ensure that the write command is sound,
    /// otherwise it may lead to unexpected behavior of the provider.
    ///
    /// In order to offer at least a minimum of safety, this method will perform cheap checks:
    ///
    /// - Provider must be online
    /// - The current provider fingerprint must match the one in the write command
    pub async fn write_variables_unchecked(
        &self,
        write_command: &WriteVariablesCommandT,
    ) -> Result<()> {
        let Some(cur_fingerprint) = self.get_fingerprint() else {
            return Err(Error::ProviderOfflineOrInvalid(self.provider_id.clone()));
        };

        if cur_fingerprint != write_command.variables.provider_definition_fingerprint {
            return Err(Error::ProviderFingerprintMismatch {
                expected: write_command.variables.provider_definition_fingerprint,
                actual: cur_fingerprint,
            });
        }

        //Create flatbuffer read request payload
        let mut builder = FlatBufferBuilder::new();
        let offset = write_command.pack(&mut builder);
        builder.finish(offset, None);

        //use collapse to avoid copying vector
        let (all_bytes, data_start_offset) = builder.collapse();
        let request_bytes = Bytes::from(all_bytes).slice(data_start_offset..);

        //Send write command
        self.get_nats_client()
            .publish(
                nats_subjects::write_variables_command(&self.provider_id),
                request_bytes,
            )
            .await?;

        //Flush to ensure command is sent
        self.get_nats_client().flush().await?;

        Ok(())
    }

    /// Sends a variable write command to the provider.
    /// Note that the provider decides if the write is accepted or not, however, the provider will not reply to the write command.
    ///
    /// This triggers a nats publish and flush command.
    ///
    /// This method will check if all supplied variable IDs are still valid before sending the write command.
    /// It will also check if the variable IDs are writable and if the value types match the variable definitions.
    ///
    /// This method may fail if there is an issue with the nats connection or the provider is unavailable.
    pub async fn write_variables(&self, write_command: &WriteVariablesCommandT) -> Result<()> {
        self.check_online()?;

        self.check_write_command(write_command)?;
        self.write_variables_unchecked(write_command).await
    }

    pub(super) async fn read_provider_definition_internal(
        nats_client: &async_nats::Client,
        provider_id: &str,
    ) -> Result<ReadProviderDefinitionQueryResponseT> {
        //Create flatbuffer read request payload
        let mut builder: FlatBufferBuilder<'_> = FlatBufferBuilder::new();
        let request_payload = ReadProviderDefinitionQueryRequestT {};
        let offset = request_payload.pack(&mut builder);
        builder.finish(offset, None);

        //use collapse to avoid copying vector
        let (all_bytes, data_start_offset) = builder.collapse();
        let request_bytes = Bytes::from(all_bytes).slice(data_start_offset..);

        let reply = nats_client
            .request(
                nats_subjects::registry_provider_definition_read_query(provider_id),
                request_bytes,
            )
            .await?;

        let payload = flatbuffers::root::<ReadProviderDefinitionQueryResponse>(&reply.payload)?;
        Ok(payload.unpack())
    }

    pub(super) async fn subscribe_provider_definition_internal(
        nats_client: &async_nats::Client,
        provider_id: &str,
    ) -> Result<impl Stream<Item = Result<ProviderDefinitionChangedEventT>>> {
        let subscription = nats_client
            .subscribe(nats_subjects::registry_provider_definition_changed_event(
                provider_id,
            ))
            .await?;

        let result_stream =
            subscription.map(|message| -> Result<ProviderDefinitionChangedEventT> {
                let payload =
                    flatbuffers::root::<ProviderDefinitionChangedEvent>(&message.payload)?;
                Ok(payload.unpack())
            });

        Ok(result_stream)
    }

    fn get_nats_client(&self) -> &async_nats::Client {
        self.consumer.get_nats_con().get_client()
    }

    /// Converts a provider definition to a hash map of variable IDs and their definitions for quick lookups.
    fn variable_defs_from_provider_def(
        provider_def: ProviderDefinitionT,
    ) -> FxHashMap<VariableID, VariableDefinitionT> {
        provider_def
            .variable_definitions
            .unwrap_or_default()
            .into_iter()
            .map(|var_def| (var_def.id, var_def))
            .collect()
    }

    /// Generates a mapping from hashed variable keys to variable IDs from an existing variable mapping table
    fn key_mapping_from_variable_defs(
        var_defs: &FxHashMap<VariableID, VariableDefinitionT>,
    ) -> FxHashMap<VariableKeyHash, VariableID> {
        var_defs
            .iter()
            .map(|(id, var_def)| (VariableKey::from(&var_def.key).key_hash, *id))
            .collect()
    }

    /// Constantly updates the internal provider definition so methods can do proper error checking,
    /// e.g. when setting a variable that doesnt exist
    async fn internal_event_loop(
        event_stream: impl Stream<Item = Result<ProviderDefinitionChangedEventT>>,
        state: SharedState,
    ) {
        tokio::pin!(event_stream);

        while let Some(event) = event_stream.next().await {
            match event {
                Ok(event) => {
                    if let Some(provider_def) = event.provider_definition {
                        if provider_def.state == ProviderDefinitionState::OK {
                            //New valid provider definition received
                            //Update variable mapping and fingerprint
                            let new_fingerprint = provider_def.fingerprint;
                            let cur_variable_defs =
                                Self::variable_defs_from_provider_def(*provider_def);
                            let var_mapping =
                                Self::key_mapping_from_variable_defs(&cur_variable_defs);

                            let mut writeable_state = state.write().unwrap();
                            writeable_state.cur_fingerprint = Some(new_fingerprint);
                            writeable_state.cur_variable_defs = cur_variable_defs;
                            writeable_state.var_mapping = var_mapping;
                        } else {
                            //Provider definition is invalid
                            let mut writeable_state = state.write().unwrap();
                            writeable_state.cur_fingerprint = None;
                        }
                    } else {
                        //empty payload means provider was removed
                        //we keep mappings unchanged and do not clear the internal state, so user
                        //can still use mapping methods while provider is offline
                        let mut writeable_state = state.write().unwrap();
                        writeable_state.cur_fingerprint = None;
                    }
                }
                Err(e) => {
                    warn!("Error while processing provider definition changed event: {e:?}");
                }
            }
        }

        //This should never happen?
        error!("Provider definition events ended");
    }

    /// Returns an error if the value type of the variable does not match the variable definition.
    fn check_variable_value_type(
        value_type: &VariableValueT,
        var_def: VariableDataType,
    ) -> Result<()> {
        match (value_type, var_def) {
            (VariableValueT::Float64(_), VariableDataType::FLOAT64) => Ok(()),
            (VariableValueT::Int64(_), VariableDataType::INT64) => Ok(()),
            (VariableValueT::String(_), VariableDataType::STRING) => Ok(()),
            (VariableValueT::Timestamp(_), VariableDataType::TIMESTAMP) => Ok(()),
            (VariableValueT::Boolean(_), VariableDataType::BOOLEAN) => Ok(()),
            (VariableValueT::Duration(_), VariableDataType::DURATION) => Ok(()),
            _ => Err(Error::InvalidValueType),
        }
    }

    /// Checks if the write command is valid.
    ///
    /// This includes checking if the variable IDs are valid, if the variable is writable and if the value type matches the variable definition.
    fn check_write_command(&self, write_command: &WriteVariablesCommandT) -> Result<()> {
        //check variable IDs, permissions and types
        let var_defs = &self.state.read().unwrap().cur_variable_defs;

        let written_vars = &write_command.variables.items;
        if let Some(written_vars) = written_vars {
            for var in written_vars {
                //lookup variable def by id
                let var_def = var_defs
                    .get(&var.id)
                    .ok_or(Error::InvalidVariableId(var.id))?;

                //check if var has write permission
                if var_def.access_type != VariableAccessType::READ_WRITE {
                    return Err(Error::NotPermitted(var_def.key.clone()));
                }

                //check if value type matches var def
                Self::check_variable_value_type(&var.value, var_def.data_type)?;
            }
        }

        Ok(())
    }

    pub(super) fn check_online(&self) -> Result<()> {
        if !self.is_online() {
            return Err(Error::ProviderOfflineOrInvalid(self.provider_id.clone()));
        }
        Ok(())
    }

    pub(super) fn get_state(&self) -> &SharedState {
        &self.state
    }
}

/// Used to stop the internal status update task
impl Drop for ConnectedNatsProvider {
    fn drop(&mut self) {
        self.event_loop_task.abort();
    }
}
