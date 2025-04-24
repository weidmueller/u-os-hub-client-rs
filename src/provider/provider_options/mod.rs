//! Contains the provider builder which is need to create a provider.
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

use thiserror::Error;
use tracing::{debug, error};

use crate::{
    authenticated_nats_con::{AuthenticatedNatsConnection, AuthenticationSettings},
    provider::worker::ProviderWorker,
    variable::Variable,
};

use super::Provider;

#[cfg(test)]
mod provider_options_test;

/// The ProviderOptions is used to create a Provider
#[derive(Debug, Clone)]
pub struct ProviderOptions {
    variables: BTreeMap<u32, Variable>,
}

impl Default for ProviderOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderOptions {
    /// Create a new provider builder
    pub fn new() -> Self {
        Self {
            variables: BTreeMap::new(),
        }
    }

    /// Add multiple variable. They will be registerd on [`Self::register()`].
    pub fn add_variables(mut self, vars: Vec<Variable>) -> Result<Self, AddVariablesError> {
        check_for_duplicates(&self.variables, &vars)?;

        let new_variables: BTreeMap<u32, Variable> =
            vars.iter().map(|var| (var.id, var.clone())).collect();
        self.variables.extend(new_variables);
        Ok(self)
    }

    /// Registers the provider on the registry, using the provided nats server address and authentication settings.
    ///
    /// This will create a new [`AuthenticatedNatsConnection`] internally and use it to register the provider.
    pub async fn register(
        self,
        nats_server_address: impl Into<String>,
        auth_settings: &AuthenticationSettings,
    ) -> Result<Provider, ConnectError> {
        let auth_nats_con =
            Arc::new(AuthenticatedNatsConnection::new(nats_server_address, auth_settings).await?);

        self.register_with_existing_connection(auth_nats_con).await
    }

    /// Registers the provider on the registry using an existing nats connection.
    ///
    /// This is useful if you want to use the same [`AuthenticatedNatsConnection`] for multiple clients.
    pub async fn register_with_existing_connection(
        self,
        nats_con: Arc<AuthenticatedNatsConnection>,
    ) -> Result<Provider, ConnectError> {
        debug!(
            "Register `{}` variables at creation time",
            self.variables.len()
        );
        let control_tx = ProviderWorker::new(nats_con, self.variables, true).await?;

        Ok(Provider::new(control_tx))
    }
}

// TODO: Should we move this to a trait?
/// Checks for duplicates (id and key) before adding new variables to a current list.
pub fn check_for_duplicates(
    current_list: &BTreeMap<u32, Variable>,
    variables_to_add: &[Variable],
) -> Result<(), AddVariablesError> {
    let mut new_ids = HashSet::new();
    let mut new_names = HashSet::new();

    // Check new variables
    for new_variable in variables_to_add {
        if !new_ids.insert(new_variable.id) {
            return Err(AddVariablesError::DuplicatedId(new_variable.id));
        }
        if !new_names.insert(new_variable.key.clone()) {
            return Err(AddVariablesError::DuplicatedKey(new_variable.key.clone()));
        }
    }

    // Check merged variables
    for (id, variable) in current_list {
        if new_ids.contains(id) {
            return Err(AddVariablesError::DuplicatedId(variable.id));
        }
        if new_names.contains(&variable.key) {
            return Err(AddVariablesError::DuplicatedKey(variable.key.clone()));
        }
    }
    Ok(())
}

/// Error that can occur when adding a variable.
#[derive(Error, Debug, PartialEq)]
pub enum AddVariablesError {
    /// Indicates a duplicated id
    #[error("Duplicated id `{0}`")]
    DuplicatedId(u32),
    /// Indicates a duplicated key
    #[error("Duplicated key `{0}`")]
    DuplicatedKey(String),
}

/// Error that can occur when connecting the provider
#[derive(Error, Debug)]
pub enum ConnectError {
    /// Indicates an error with the nats connection
    #[error("Nats error: `{0}`")]
    Nats(#[from] async_nats::Error),
    /// Indicates an error with the OAuth2 token request
    #[error("Oauth2 error: `{0}`")]
    OAuth(String),
    /// Indicates an error with the registration at the registry
    #[error("Error while sending the provider definition: `{0}`")]
    UpdateProviderDefinition(UpdateProviderDefinitionError),
    /// Indicates that a connection to the data hub timed out after 5 minutes
    #[error("Connection to the data hub timed out after 5 minutes")]
    Timeout,
}

/// Error that can occur when updating a provider definition to the registry.
#[derive(Error, Debug)]
pub enum UpdateProviderDefinitionError {
    /// Indicates an error with the nats connection
    #[error("Nats error: `{0}`")]
    Nats(async_nats::Error),
    /// Indicates an error with the provider definition
    #[error("Invalid provider definition: `{0}`")]
    InvalidProviderDefinition(String),
}
