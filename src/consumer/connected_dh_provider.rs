//! This module provides a high-level API for interacting with a variable hub provider
//! by abstacting the low-level API details via easy to use rust types.

use std::{collections::HashSet, sync::Arc};

use futures::{Stream, StreamExt};
use thiserror::Error;
use tracing::error;

use crate::{
    dh_types::{VariableDefinition, VariableID},
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionState, ReadVariablesQueryRequestT, TimestampT, VariableListT,
        VariableQuality, VariableT, VariablesChangedEventT, WriteVariablesCommandT,
    },
    variable,
};

use super::{
    connected_nats_provider::{self, ConnectedNatsProvider, ConnectedNatsProviderState},
    consumer_types::{self, VariableState},
    dh_consumer::{self, DataHubConsumer},
    variable_key::VariableKey,
};

/// Error type for the connected data hub provider.
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("{0}")]
    Consumer(#[from] dh_consumer::Error),
    #[error("{0}")]
    TypeConversion(#[from] consumer_types::Error),
    #[error("{0}")]
    LowLevelApi(#[from] connected_nats_provider::Error),
}

/// Result type for the connected data hub provider.
pub type Result<T> = core::result::Result<T, Error>;

/// A value that can be converted into a [`VariableKey`].
///
/// This allows to e.g. use strings in place of [`VariableKey`] and perform automatic conversion.
/// However, for maximum performance, it is recommended to always only create a variable key once and then reuse it for multiple calls.
///
/// See documentation of [`VariableKey`] for more details.
pub trait VariableKeyLike<'a>: Into<VariableKey<'a>> + Copy {}
impl<'a> VariableKeyLike<'a> for VariableKey<'a> {}
impl<'a> VariableKeyLike<'a> for &'a str {}
impl<'a> VariableKeyLike<'a> for &'a String {}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a change in the provider state.
pub enum ProviderEvent {
    /// The provider has changed and has a new, valid definiton.
    DefinitionChanged(Vec<VariableDefinition>),
    /// The provider is offline.
    Offline,
    /// The provider is online but has an invalid definition.
    Invalid,
}

/// Represents a connection to a data hub provider.
/// This is used to read and write variables from/to the provider.
///
/// In contrary to the low level API, this API is designed to be easy to use and mostly
/// uses variable keys instead of variable IDs for all its operations.
///
/// This comes at the cost of higher overhead due to having to perform key lookups,
/// type conversions and some additional copying internally.
///
/// Most methods accept variable keys as input, which are automatically converted to variable IDs internally.
/// This allows you to use strings or other types that implement the [`VariableKeyLike`] trait.
/// However, for maximum performance it is recommended to always create a variable key once and then reuse it for multiple calls.
/// See documentation of [`VariableKey`] for more details.
pub struct DataHubProviderConnection {
    connected_provider: ConnectedNatsProvider,
}

impl DataHubProviderConnection {
    /// Tries to connect to the specified provider.
    ///
    /// If `wait_for_provider` is set to true, the function will wait until the provider is available,
    /// otherwise it will fail if the provider is not currently available on the registry.
    ///
    /// Please note that a provider may be available before all of its variables are present.
    /// Use [`Self::wait_until_variable_keys_are_available()`] to wait for all desired variable keys to be available.
    ///
    /// This method may fail if there is an issue with the nats connection
    /// or something goes wrong while deserializing flatbuffer payloads.
    pub async fn new(
        consumer: Arc<DataHubConsumer>,
        provider_id: impl Into<String>,
        wait_for_provider: bool,
    ) -> Result<Self> {
        let provider_id = provider_id.into();

        if wait_for_provider {
            consumer.wait_for_provider(&provider_id).await?;
        }

        //create ll provider connection
        let connected_provider =
            ConnectedNatsProvider::new(consumer.get_nats_consumer().clone(), provider_id).await?;

        Ok(Self { connected_provider })
    }

    /// Waits until all specified variable keys are available from the provider.
    ///
    /// Some providers will register before all their variables are available and add them later.
    /// This method can be used to ensure that all specified variable keys are available before proceeding with further operations.
    ///
    /// Note that this will block forever if the provider never registers the specified variable keys.
    /// There is no internal timeout, but you may wrap this within a timeout call.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection.
    pub async fn wait_until_variable_keys_are_available(&self, keys: &[&str]) -> Result<()> {
        if keys.is_empty() {
            return Ok(());
        }

        //sub must happen before first check to make sure we dont miss any events
        let mut provider_evt_stream = self.subscribe_provider_events().await?;

        //see if all keys are already present
        let current_var_defs = self.get_all_variable_definitions()?;
        let all_keys_available = keys
            .iter()
            .all(|key| current_var_defs.iter().any(|def| def.key == *key));

        if all_keys_available {
            return Ok(());
        }

        //Wait until the provider offers all requested variable keys
        while let Some(provider_evt) = provider_evt_stream.next().await {
            if let ProviderEvent::DefinitionChanged(var_defs) = &provider_evt {
                let all_keys_available = keys
                    .iter()
                    .all(|key| var_defs.iter().any(|def| def.key == *key));

                if all_keys_available {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Allows access to the low level api
    pub fn get_connected_nats_provider(&self) -> &ConnectedNatsProvider {
        &self.connected_provider
    }

    /// Returns the provider ID.
    pub fn get_provider_id(&self) -> &str {
        self.connected_provider.get_provider_id()
    }

    /// Returns a stream of events for the connected provider.
    /// This allows you to receive events when the provider goes offline or when the provider definition changes.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection.
    ///
    /// If the registry goes offline while subscribed, the subscription will stay active but you will
    /// no longer receive any events until the registry is back online.
    ///
    /// Internally uses the low level api to receive the values.
    /// Each received value from the low level api will be converted to an easy to use rust type.
    /// If the low level api stream returned an error value, this value will be silenly ignored, but the subscription will not be cancelled.
    pub async fn subscribe_provider_events(&self) -> Result<impl Stream<Item = ProviderEvent>> {
        let provider_def_events = self
            .connected_provider
            .subscribe_provider_definition()
            .await?;

        let mapped_events = provider_def_events.filter_map(move |event| async {
            let ll_event = event.ok()?;

            let mapped_event = match ll_event.provider_definition {
                Some(provider_def) => {
                    if provider_def.state == ProviderDefinitionState::OK {
                        let new_variable_defs = provider_def
                            .variable_definitions
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|var_def| var_def.try_into().ok())
                            .collect();
                        ProviderEvent::DefinitionChanged(new_variable_defs)
                    } else {
                        ProviderEvent::Invalid
                    }
                }
                None => ProviderEvent::Offline,
            };

            Some(mapped_event)
        });

        Ok(Box::pin(mapped_events))
    }

    /// Returns the variable key string belonging to the specified variable ID.
    ///
    /// Will fail if the variable ID is unknown.
    #[inline(always)]
    pub fn variable_key_from_id(&self, id: VariableID) -> Result<String> {
        Ok(self.connected_provider.variable_key_from_id(id)?)
    }

    /// Returns the variable ID belonging to the specified variable key.
    ///
    /// Will fail if the variable key is unknown.
    #[inline(always)]
    pub fn variable_id_from_key<'a>(&self, key: impl VariableKeyLike<'a>) -> Result<VariableID> {
        Ok(self.connected_provider.variable_id_from_key(key.into())?)
    }

    /// Returns a cached list of all variable definitions for this provider.
    ///
    /// The cached value will be updated internally once the provider definition changes.
    pub fn get_all_variable_definitions(&self) -> Result<Vec<VariableDefinition>> {
        let var_defs = self.connected_provider.get_all_variable_definitions();

        let result = var_defs
            .into_iter()
            .filter_map(|(_id, var_def)| var_def.try_into().ok())
            .collect();

        Ok(result)
    }

    /// Returns the cached variable definition for this variable.
    ///
    /// Will fail if the variable is unknown.
    /// The cached value will be updated internally once the provider definition changes.
    pub fn get_variable_definition<'a>(
        &self,
        var: impl VariableKeyLike<'a>,
    ) -> Result<VariableDefinition> {
        let id = self.variable_id_from_key(var)?;

        let var_def = self.connected_provider.get_variable_definition(id)?;
        Ok(var_def.try_into()?)
    }

    /// Reads the current state of a single variable from the provider.
    ///
    /// Will check if the supplied variable ID is valid before sending the read request.
    ///
    /// This method may fail if there is an issue with the nats connection, the provider is unavailable
    /// or something goes wrong while deserializing flatbuffer payloads.
    pub async fn read_single_variable<'a>(
        &self,
        var: impl VariableKeyLike<'a>,
    ) -> Result<VariableState> {
        let key = var.into();
        let response_variable_list = self.read_variables(Some(&[key])).await?;

        let state = response_variable_list
            .into_iter()
            .next()
            .ok_or_else(|| connected_nats_provider::Error::InvalidVariableKey(key.to_string()))?
            .1;

        Ok(state)
    }

    /// Reads the current state of all provider variables.
    ///
    /// The `filter` is optional and can be used to only read a subset of variables. If set to None, all variables will be read.
    ///
    /// Will check if the supplied variable ID in the filter are valid before sending the read request.
    ///
    /// This method may fail if there is an issue with the nats connection, the provider is unavailable
    /// or something goes wrong while deserializing flatbuffer payloads.
    pub async fn read_variables<'a>(
        &self,
        filter: Option<&[impl VariableKeyLike<'a>]>,
    ) -> Result<Vec<(VariableID, VariableState)>> {
        self.connected_provider.check_online()?;

        //Build ll api read request
        let var_ids = if let Some(vars) = filter {
            //convert handles to IDs
            let mut ids = Vec::with_capacity(vars.len());

            for var in vars {
                let id = self.variable_id_from_key(*var)?;
                ids.push(id);
            }

            ReadVariablesQueryRequestT { ids: Some(ids) }
        } else {
            //No need to check ids, just forward to LL api
            ReadVariablesQueryRequestT { ids: None }
        };

        //Read low level result
        //Safety: We can use the unchecked method here because we already know that all variable IDs are valid
        let low_level_data = self
            .connected_provider
            .read_variables_unchecked(&var_ids)
            .await?;

        let base_timestamp = low_level_data.variables.base_timestamp.into();

        //map to user friendly data types
        let result = low_level_data
            .variables
            .items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|ll_var| -> Option<(u32, VariableState)> {
                let map_entry = (ll_var.id, VariableState::new(ll_var, base_timestamp).ok()?);

                Some(map_entry)
            })
            .collect();

        Ok(result)
    }

    /// Same as [`Self::subscribe_variables_with_filter`], but only returns changes of a single variable.
    pub async fn subscribe_single_variable<'a>(
        &self,
        var: impl VariableKeyLike<'a>,
    ) -> Result<impl Stream<Item = VariableState>> {
        let mapped_stream = self
            .subscribe_variables_with_filter(Some(vec![var]))
            .await?
            .filter_map(|map| async move { map.into_iter().next().map(|(_id, state)| state) });

        Ok(Box::pin(mapped_stream))
    }

    /// Returns a stream of variable state changes for the connected provider.
    ///
    /// Only returns changes to the provided variable keys by implementing client side filtering.
    /// If `filter_list` is set to None, all changes will be returned.
    ///
    /// This method allows you to subscribe to variable keys that do not yet exist. This allows the provider
    /// to register new variables and the client to receive updates for them without having to resubscribe.
    ///
    /// The subscription will continue even if the provider definition changes while subscribed.
    /// In case a filtered variable key no longer exists, no more updates for this variable
    /// will be yielded by the stream. If the key still exists but got a different ID, the filter will continue to work.
    ///
    /// This method will succeed even if the hub registry is currently offline, but may return an error if there is an issue
    /// with the NATS connection.
    ///
    /// Internally uses the low level api to receive the values.
    /// Each received value from the low level api will be converted to an easy to use rust type.
    /// If the low level api stream returned an error value, this value will be silenly ignored, but the subscription will not be cancelled.
    pub async fn subscribe_variables_with_filter<'a>(
        &self,
        filter_list: Option<Vec<impl VariableKeyLike<'a>>>,
    ) -> Result<impl Stream<Item = Vec<(VariableID, VariableState)>>> {
        let mut last_fp = self.connected_provider.get_fingerprint();
        let state_clone = self.connected_provider.get_state().clone();

        //build initial filter set and remember initial fingerprint
        let mut filter_set = filter_list.as_ref().map(|filter_list| {
            //Safety: Unwrap is ok here as we know that the mutex cant be poisoned during writing
            #[allow(clippy::unwrap_used)]
            Self::build_filter_set(filter_list, &state_clone.read().unwrap())
        });

        //Subscibe to all, unfiltered variable change events
        let low_level_data = self.connected_provider.subscribe_variables().await?;

        //Filter variables and map to user friendly type
        let mapped_stream = low_level_data.filter_map(move |var_changed_evt| {
            if let Some(filter_list) = &filter_list {
                //check if we need to rebuild the filter because provider changed

                //Safety: Unwrap is ok here as we know that the mutex cant be poisoned during writing
                #[allow(clippy::unwrap_used)]
                let readable_state = state_clone.read().unwrap();

                let current_fp = readable_state.cur_fingerprint;
                if current_fp.is_some() && current_fp != last_fp {
                    filter_set = Some(Self::build_filter_set(filter_list, &readable_state));
                    last_fp = current_fp;
                }
            }

            let mapped_and_filtered_vars =
                Self::process_var_changed_evt(&filter_set, var_changed_evt);

            async move { mapped_and_filtered_vars }
        });

        Ok(Box::pin(mapped_stream))
    }

    /// Sends a write command to the provider for a single variable.
    /// Note that the provider decides if the write is accepted or not, however, the provider will not reply to the write command.
    ///
    /// If you want to write multiple variables, use [`Self::write_variables()`] instead,
    /// as this will be more performant for writing multiple values at once.
    ///
    /// You need a connection with [NatsPermission::VariableHubReadWrite](`crate::authenticated_nats_con::NatsPermission::VariableHubReadWrite`) to be able to write variables.
    ///
    /// This method will check if the specified variable is still valid before sending the write command.
    /// It will also check if the variable is writable and if the value type matches the variable definitions.
    ///
    /// This method may fail if there is an issue with the nats connection or the provider is unavailable.
    pub async fn write_single_variable<'a>(
        &self,
        var: impl VariableKeyLike<'a>,
        new_value: impl Into<variable::value::VariableValue>,
    ) -> Result<()> {
        self.write_variables(&[(var, new_value.into())]).await
    }

    /// Same as [`Self::write_single_variable`], but allows to write multiple variables at once.
    ///
    /// This is more efficient than calling [`Self::write_single_variable`] multiple times.
    pub async fn write_variables<'a>(
        &self,
        new_values: &[(impl VariableKeyLike<'a>, variable::value::VariableValue)],
    ) -> Result<()> {
        let provider_definition_fingerprint =
            self.connected_provider.get_fingerprint().ok_or_else(|| {
                connected_nats_provider::Error::ProviderOfflineOrInvalid(
                    self.get_provider_id().to_owned(),
                )
            })?;

        let mut changed_vars = Vec::with_capacity(new_values.len());

        for (var, new_value) in new_values {
            let id = self.variable_id_from_key(*var)?;

            let ll_var = VariableT {
                id,
                value: new_value.into(),
                //TODO: Only to be set by the provider. For now, add default filler. May revise flatbuffer api later
                quality: VariableQuality::BAD,
                timestamp: None,
            };

            changed_vars.push(ll_var);
        }

        //build write command
        let var_list = Box::new(VariableListT {
            provider_definition_fingerprint,
            //TODO: Only to be set by the provider. For now, add default filler. May revise flatbuffer api later
            base_timestamp: TimestampT::default(),
            items: Some(changed_vars),
        });

        let write_command = WriteVariablesCommandT {
            variables: var_list,
        };

        //Send write command - This checks write permissions and variable types internally
        self.connected_provider
            .write_variables(&write_command)
            .await?;

        Ok(())
    }

    /// Builds a hashset of filter IDs for efficient filtering in stream.
    ///
    /// If a key doesnt exist, its id will be skipped and not be inserted into the filter set.
    fn build_filter_set<'a>(
        filter_list: &[impl VariableKeyLike<'a>],
        low_level_state: &ConnectedNatsProviderState,
    ) -> HashSet<VariableID> {
        let mut filter_set = HashSet::with_capacity(filter_list.len());

        for var in filter_list {
            let key: VariableKey = (*var).into();
            let id = low_level_state.var_mapping.get(&key.key_hash);

            if let Some(id) = id {
                filter_set.insert(*id);
            }
        }

        filter_set
    }

    /// Applies the specified filter set to the event and returns a hashmap of variable IDs to variable states.
    fn process_var_changed_evt(
        filter_set: &Option<HashSet<u32>>,
        var_changed_evt: connected_nats_provider::Result<VariablesChangedEventT>,
    ) -> Option<Vec<(VariableID, VariableState)>> {
        let Ok(var_changed_evt) = var_changed_evt else {
            //Low level error while receiving event
            return None;
        };

        let base_timestamp = var_changed_evt.changed_variables.base_timestamp.into();

        //Convert the list of all received variables to a user
        //friendly hashmap of ids -> VariableState
        let received_filtered_vars = var_changed_evt
            .changed_variables
            .items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|ll_var| -> Option<(VariableID, VariableState)> {
                if let Some(filter_set) = &filter_set {
                    if !filter_set.contains(&ll_var.id) {
                        return None;
                    }
                }

                let map_entry = (ll_var.id, VariableState::new(ll_var, base_timestamp).ok()?);

                Some(map_entry)
            })
            .collect();

        Some(received_filtered_vars)
    }
}
