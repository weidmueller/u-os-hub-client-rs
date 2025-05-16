//! This module provides APIs and data types for interacting with the u-OS Data Hub as a provider.
//! Providers are responsible for providing variables to the Data Hub and managing their values.
//! They can publish data and accept write requests from consumers.
//! All providers are managed by the Data Hub registry.
//!
//! The following example demonstrates how to set up a basic provider that updates a variable periodically and accepts write commands from consumers.
//!
//! ```no_run
//!# use std::{collections::HashMap, time::Duration};
//!#
//! use u_os_hub_client::{
//!     authenticated_nats_con::{
//!         AuthenticationSettingsBuilder, NatsPermission, DEFAULT_U_OS_NATS_ADDRESS,
//!     },
//!     oauth2::OAuth2Credentials,
//!     provider::{Provider, ProviderBuilder, VariableBuilder},
//!     variable::Variable,
//! };
//!
//! /// Demonstrates how to create a provider that serves variables to the Data Hub.
//! async fn provider_init() -> anyhow::Result<()> {
//!     //Configure your nats server authentication
//!     let auth_settings = AuthenticationSettingsBuilder::new(NatsPermission::VariableHubProvide)
//!         .with_credentials(OAuth2Credentials {
//!             //NATS client name of the provider
//!             client_name: "test-provider".to_string(),
//!             //Obtained by the u-OS Identity&Access Client GUI
//!             client_id: "<your_oauth_client_id>".to_string(),
//!             client_secret: "<your_oauth_client_secret>".to_string(),
//!         })
//!         .build();
//!
//!     // Create some Variables by using a VariableBuilder
//!     let ro_int = VariableBuilder::new(300, "my_folder.ro_int")
//!         .initial_value(1000)
//!         .build()?;
//!
//!     let rw_string = VariableBuilder::new(200, "my_folder.rw_string")
//!         .initial_value("write me!")
//!         .read_write()
//!         .build()?;
//!
//!     // Use the ProviderBuilder to create an initial provider definition and register on the Data Hub registry
//!     let provider_builder =
//!         ProviderBuilder::new().add_variables(vec![ro_int.clone(), rw_string.clone()])?;
//!
//!     // Register on the Data Hub registry. This will yield a Provider handle which
//!     // can be used to add/remove and modify variables even after the provider has been registered.
//!     let provider = provider_builder
//!         .register(DEFAULT_U_OS_NATS_ADDRESS, &auth_settings)
//!         .await?;
//!
//!     //Start endless loop for provider logic
//!     provider_logic(provider, ro_int, rw_string).await?;
//!
//!     Ok(())
//! }
//!
//! /// Simulates the logic of our provider.
//! ///
//! /// For simplicity, we simply modify one of our variables and accept write commands on the other variable
//! async fn provider_logic(
//!     provider: Provider,
//!     mut ro_int: Variable,
//!     rw_string: Variable,
//! ) -> anyhow::Result<()> {
//!     // Subscribe to write requests of our RW variables
//!     // We use a hashmap to quickly find the variable by its id later
//!     let mut writable_vars = HashMap::from([(rw_string.get_definition().id, rw_string)]);
//!
//!     let mut write_command_sub = provider
//!         .subscribe_to_write_command(writable_vars.values().cloned().collect())
//!         .await?;
//!
//!     // Start a periodic timer to update the value of our read-only variable
//!     let mut var_write_timer = tokio::time::interval(Duration::from_secs(1));
//!     var_write_timer.tick().await; //skip first tick
//!
//!     let mut cur_int_val = 1000;
//!
//!     loop {
//!         tokio::select! {
//!             // Increment the value of ro_int periodically
//!             _ = var_write_timer.tick() => {
//!                 let ro_int_state = ro_int.get_mut_state();
//!                 ro_int_state.set_value(cur_int_val);
//!
//!                 let updated_states = vec![ro_int_state.clone()];
//!                 if let Err(e) = provider.update_variable_states(updated_states).await {
//!                     eprintln!("Error updating variable states: {e}");
//!                 }
//!
//!                 cur_int_val += 1;
//!             },
//!             // React to write commands of consumers
//!             Some(write_commands) = write_command_sub.recv() => {
//!                 // The logic here is implementation defined
//!                 // In this example, we simply accept all write commands and update the states
//!                 let mut updated_states = Vec::with_capacity(write_commands.len());
//!
//!                 for write_cmd in write_commands {
//!                     let written_var = writable_vars.get_mut(&write_cmd.id);
//!
//!                     if let Some(written_var) = written_var {
//!                         // Update the variable state with the new value. This will automatically update the timestamp
//!                         let written_var_state = written_var.get_mut_state();
//!                         written_var_state.set_value(write_cmd.value);
//!                         updated_states.push(written_var_state.clone());
//!                     }
//!                     else {
//!                         eprintln!("Received write command for unknown variable ID: {}", write_cmd.id);
//!                     }
//!                 }
//!
//!                 // Publish the updated states to the Data Hub
//!                 if let Err(e) = provider.update_variable_states(updated_states).await {
//!                     eprintln!("Error updating variable states: {e}");
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```

pub mod provider_builder;
pub mod provider_definition_validator;
pub mod provider_types;

pub mod test_data;

pub mod variable_builder;
pub mod variable_definition_validator;
mod worker;

pub use provider_builder::ProviderBuilder;
use provider_builder::UpdateProviderDefinitionError;
use provider_definition_validator::InvalidProviderDefinitionError;
use provider_types::{VariableState, VariableWriteCommand};
use thiserror::Error;
pub use variable_builder::{VariableBuildError, VariableBuilder};

use async_nats::Message;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    oneshot,
};

use crate::variable::Variable;

/// Commands that are sent to the worker task
#[derive(Debug)]
pub(crate) enum ProviderCommand {
    AddVariables(
        Vec<Variable>,
        oneshot::Sender<Result<(), AddVariablesError>>,
    ),
    RemoveVariables(
        Vec<Variable>,
        oneshot::Sender<Result<(), RemoveVariablesError>>,
    ),
    UpdateStates(
        Vec<VariableState>,
        oneshot::Sender<Result<(), UpdateVariableValuesError>>,
    ),
    HandleWrite(Message),
    Query(Message),
    Register,
    Unregister,
    Subscribe(
        Vec<Variable>,
        oneshot::Sender<
            Result<mpsc::Receiver<Vec<VariableWriteCommand>>, SubscribeToWriteCommandError>,
        >,
    ),
}

/// Error that can occur when adding a variable
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum AddVariablesError {
    #[error("The background thread crashed. You need to recreate the provider.")]
    ProviderThreadCrashed,
    #[error("Nats error: `{0}`")]
    NatsError(async_nats::Error),
    #[error("Error while sending the provider definition: `{0}`")]
    UpdateProviderDefinition(UpdateProviderDefinitionError), // TODO: Should we do this flatten?
    #[error("Invalid merged variable list: `{0}`")]
    InvalidMergedVariableList(#[from] InvalidProviderDefinitionError),
}

/// Error that can occur when removing a variable
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum RemoveVariablesError {
    #[error("The background thread crashed. You need to recreate the provider.")]
    ProviderThreadCrashed,
    #[error("Nats error: `{0}`")]
    NatsError(async_nats::Error),
    #[error("Error while sending the provider definition: `{0}`")]
    UpdateProviderDefinition(UpdateProviderDefinitionError), // TODO: Should we do this flatten?
}

/// Error that can occur when updating a variable value
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum UpdateVariableValuesError {
    #[error("The background thread crashed. You need to recreate the provider.")]
    ProviderThreadCrashed,
    #[error("Nats error: `{0}`")]
    NatsError(async_nats::Error),
    #[error("Wrong value type on `{0}`")]
    TypeMismatch(String),
    #[error("Can't find variable `{0}`")]
    VariableNotFound(String),
}

/// Error that can occur when subscribing to a write command
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum SubscribeToWriteCommandError {
    #[error("The background thread crashed. You need to recreate the provider.")]
    ProviderThreadCrashed,
    #[error("Can't find variable `{0}`")]
    VariableNotFound(String),
    #[error("There is already an active write subscription of the variable `{0}`")]
    AlreadySubscribed(String),
    #[error("Can't subscribt to write command of read only variabe `{0}`")]
    ReadOnlyVariable(String),
}

/// Represents a provider that is registered on the Data Hub registry.
///
/// Can be used to add/remove variables and update their values.
/// Must be created by a [`ProviderBuilder`].
#[derive(Clone)]
pub struct Provider {
    command_channel: Sender<ProviderCommand>,
}

impl Provider {
    /// Creates a new Provider
    pub(crate) fn new(command_channel: Sender<ProviderCommand>) -> Self {
        Self { command_channel }
    }

    /// Adds variables to the provider.
    ///
    /// This will replace existing variables with the same id.
    ///
    /// Note that this is a rather expensive operation that modifies the provider definition and triggers
    /// a re-registration on the Data Hub registry.
    pub async fn add_variables(&self, variables: Vec<Variable>) -> Result<(), AddVariablesError> {
        let (tx, rx) = oneshot::channel();
        self.command_channel
            .send(ProviderCommand::AddVariables(variables, tx))
            .await
            .map_err(|_| AddVariablesError::ProviderThreadCrashed)?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(AddVariablesError::ProviderThreadCrashed),
        }
    }

    /// Removes variables from the provider.
    ///
    /// Will ignore variable IDs that do currently not exist on the provider.
    ///
    /// Note that this is a rather expensive operation that modifies the provider definition and triggers
    /// a re-registration on the Data Hub registry.
    pub async fn remove_variables(
        &self,
        variables: Vec<Variable>,
    ) -> Result<(), RemoveVariablesError> {
        let (tx, rx) = oneshot::channel();

        self.command_channel
            .send(ProviderCommand::RemoveVariables(variables, tx))
            .await
            .map_err(|_| RemoveVariablesError::ProviderThreadCrashed)?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(RemoveVariablesError::ProviderThreadCrashed),
        }
    }

    /// Allows to updates the states (e.g. value, timestamp, quality) of variables.
    ///
    /// This will fail if a variable does not currently exist on the provider or the value type does not match the variable type.
    /// Will trigger a publish on the NATS layer which notifies all subscribed consumers.
    ///
    /// Note that you can not change a variable definition after adding it to the provider. To do so, remove and re-add the variable.
    pub async fn update_variable_states(
        &self,
        states: Vec<VariableState>,
    ) -> Result<(), UpdateVariableValuesError> {
        let (tx, rx) = oneshot::channel();
        self.command_channel
            .send(ProviderCommand::UpdateStates(states, tx))
            .await
            .map_err(|_| UpdateVariableValuesError::ProviderThreadCrashed)?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(UpdateVariableValuesError::ProviderThreadCrashed),
        }
    }

    /// Subscribes to the write command of multiple variables.
    ///
    /// Readonly variables will be ignored.
    /// You can only open one subscriber per variable to avoid conflicts.
    pub async fn subscribe_to_write_command(
        &self,
        variables: Vec<Variable>,
    ) -> Result<Receiver<Vec<VariableWriteCommand>>, SubscribeToWriteCommandError> {
        let (tx, rx) = oneshot::channel();
        self.command_channel
            .send(ProviderCommand::Subscribe(variables, tx))
            .await
            .map_err(|_| SubscribeToWriteCommandError::ProviderThreadCrashed)?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(SubscribeToWriteCommandError::ProviderThreadCrashed),
        }
    }
}
