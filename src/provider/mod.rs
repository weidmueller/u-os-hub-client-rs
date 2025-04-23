//! Contains the provider client.

pub mod provider_definition_validator;
pub mod provider_options;

pub mod test_data;

pub mod variable_builder;
pub mod variable_definition_validator;
mod worker;

pub use provider_options::ProviderOptions;
use provider_options::UpdateProviderDefinitionError;
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

/// Manages the provider.
/// It sends commands to an internal worker task
/// so it can be copied for usage in multiple parts of your application.
/// It is created by the [`ProviderOptions`].
#[derive(Clone)]
pub struct Provider {
    command_channel: Sender<ProviderCommand>,
}

impl Provider {
    /// Creates a new Provider
    pub(crate) fn new(command_channel: Sender<ProviderCommand>) -> Self {
        Self { command_channel }
    }

    /// Adds variables to the provider
    pub async fn add_variables(&self, variables: &[Variable]) -> Result<(), AddVariablesError> {
        let (tx, rx) = oneshot::channel();
        self.command_channel
            .send(ProviderCommand::AddVariables(variables.to_vec(), tx))
            .await
            .map_err(|_| AddVariablesError::ProviderThreadCrashed)?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(AddVariablesError::ProviderThreadCrashed),
        }
    }

    /// Removes variables from the provider
    pub async fn remove_variables(
        &self,
        variables: Vec<Variable>,
    ) -> Result<(), RemoveVariablesError> {
        let (tx, rx) = oneshot::channel();

        self.command_channel
            .send(ProviderCommand::RemoveVariables(variables.to_vec(), tx))
            .await
            .map_err(|_| RemoveVariablesError::ProviderThreadCrashed)?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(RemoveVariablesError::ProviderThreadCrashed),
        }
    }

    /// Updates the values of the variables
    /// The data type of a value can't be changed.
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
    /// Readonly variables will be ignored.
    /// You can only open one subscriber per variable to avoid conflicts.
    pub async fn subscribe_to_write_command(
        &self,
        _variables: &[Variable],
    ) -> Result<Receiver<Vec<Variable>>, SubscribeToWriteCommandError> {
        let (tx, rx) = oneshot::channel();
        self.command_channel
            .send(ProviderCommand::Subscribe(_variables.to_vec(), tx))
            .await
            .map_err(|_| SubscribeToWriteCommandError::ProviderThreadCrashed)?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(SubscribeToWriteCommandError::ProviderThreadCrashed),
        }
    }
}
