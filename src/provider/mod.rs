//! This module provides APIs and data types for interacting with the u-OS Data Hub as a provider.
//! Providers are responsible for providing variables to the Data Hub and managing their values.
//! They can publish data and accept write requests from consumers.
//! All providers are managed by the Data Hub registry.
//!
//! The following example demonstrates how to set up a basic provider that updates a variable periodically and accepts write commands from consumers.
//!
//! ```no_run
//!# use std::time::Duration;
//!#
//!# use tokio::{select, task, time::sleep};
//!#
//! use u_os_hub_client::{
//!     authenticated_nats_con::{
//!         AuthenticationSettingsBuilder, NatsPermission, DEFAULT_U_OS_NATS_ADDRESS,
//!     },
//!     oauth2::OAuth2Credentials,
//!     provider::{Provider, ProviderBuilder, VariableBuilder},
//!     variable::value::VariableValue,
//! };
//!
//! async fn provider_example() -> anyhow::Result<()> {
//!     //Configure your nats server authentication
//!     let auth_settings = AuthenticationSettingsBuilder::new(NatsPermission::VariableHubProvide)
//!         .with_credentials(OAuth2Credentials {
//!             //NATS client name of the provider
//!             client_name: "test-provider".to_string(),
//!             //Obtained by the uOS Identity&Access Client GUI
//!             client_id: "<your_oauth_client_id>".to_string(),
//!             client_secret: "<your_oauth_client_secret>".to_string(),
//!         })
//!         .build();
//!
//!     // Create some Variables by using a VariableBuilder
//!     let mut ro_int = VariableBuilder::new(300, "my_folder.ro_int")
//!         .value(1000)
//!         .build()?;
//!
//!     let rw_string = VariableBuilder::new(200, "my_folder.rw_string")
//!         .value("write me!")
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
//!     // Simulate the logic of our provider
//!     // For simplicity, we simply modify one of our variables and accept write commands on the other variable
//!
//!     // Subscribe to write requests of our RW variable
//!     let mut write_command_sub = provider.subscribe_to_write_command(vec![rw_string]).await?;
//!
//!     // Start a periodic timer to update the value of our read-only variable
//!     let mut var_write_timer = tokio::time::interval(Duration::from_secs(1));
//!     var_write_timer.tick().await; //skip first tick
//!
//!     let mut cur_int_val = 1000;
//!
//!     loop {
//!         tokio::select! {
//!             //Increment the value of ro_int periodically
//!             _ = var_write_timer.tick() => {
//!                 ro_int.value = VariableValue::Int(cur_int_val);
//!
//!                 let updated_vars = vec![ro_int.clone()];
//!                 provider.update_variable_values(updated_vars).await?;
//!
//!                 cur_int_val += 1;
//!             },
//!             //For simplicity, simply accept all write commands from consumer clients
//!             //Real providers would probably check the write commands and only accept some of them
//!             Some(written_vars) = write_command_sub.recv() => {
//!                 provider.update_variable_values(written_vars).await?;
//!             }
//!         }
//!     }
//!
//!    Ok(())
//! }
//! ```

pub mod provider_builder;
pub mod provider_definition_validator;

pub mod test_data;

pub mod variable_builder;
pub mod variable_definition_validator;
mod worker;

pub use provider_builder::ProviderBuilder;
use provider_builder::UpdateProviderDefinitionError;
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
    UpdateValues(
        Vec<Variable>,
        oneshot::Sender<Result<(), UpdateVariableValuesError>>,
    ),
    HandleWrite(Message),
    Query(Message),
    Register,
    Unregister,
    Subscribe(
        Vec<Variable>,
        oneshot::Sender<Result<mpsc::Receiver<Vec<Variable>>, SubscribeToWriteCommandError>>,
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
    #[error("Duplicated id `{0}`")]
    DuplicatedId(u32),
    #[error("Duplicated key `{0}`")]
    DuplicatedKey(String),
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
    #[error("There is already an active write subscribtion of the variable `{0}`")]
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

    /// Updates the values of the variables.
    ///
    /// This will fail if a variable does not currently exist on the provider or the value type does not match the variable type.
    /// Will trigger a publish on the NATS layer which notifies all subscribed consumers.
    pub async fn update_variable_values(
        &self,
        variables: Vec<Variable>,
    ) -> Result<(), UpdateVariableValuesError> {
        let (tx, rx) = oneshot::channel();
        self.command_channel
            .send(ProviderCommand::UpdateValues(variables, tx))
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
    ) -> Result<Receiver<Vec<Variable>>, SubscribeToWriteCommandError> {
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
