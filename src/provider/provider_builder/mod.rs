// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

//! Contains the provider builder which is need to create a provider.
use std::{collections::BTreeMap, sync::Arc};

use thiserror::Error;
use tracing::{debug, error};

use crate::{
    authenticated_nats_con::{AuthenticatedNatsConnection, AuthenticationSettings},
    dh_types::VariableID,
    generated::weidmueller::ucontrol::hub::ProviderDefinitionT,
    provider::worker::ProviderWorker,
    variable::Variable,
};

use super::{provider_definition_validator::InvalidProviderDefinitionError, Provider};

#[cfg(test)]
mod provider_builder_test;

/// Used to create a [`Provider`] instance.
#[derive(Debug, Clone)]
pub struct ProviderBuilder {
    variables: BTreeMap<u32, Variable>,
}

impl Default for ProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderBuilder {
    /// Creates a new instance with an empty list of variables.
    pub fn new() -> Self {
        Self {
            variables: BTreeMap::new(),
        }
    }

    /// Adds multiple variables to the provider builder.
    ///
    /// These variables will be available immediately after the provider is registered.
    /// You can change the variables later by using the [`Provider`] handle after registration.
    pub fn add_variables(mut self, vars: Vec<Variable>) -> Result<Self, AddVariablesError> {
        validate_var_list(&self.variables, &vars)?;

        let new_variables: BTreeMap<VariableID, Variable> = vars
            .into_iter()
            .map(|var| (var.definition.id, var))
            .collect();

        self.variables.extend(new_variables);

        Ok(self)
    }

    /// Registers the provider on the registry, using the provided nats server address and authentication settings.
    ///
    /// Returns a [`Provider`] handle which can be used to add/remove and modify variables even after the provider has been registered.
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

    /// Same as [`Self::register()`], but uses an existing connection.
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

/// Checks if a potential merge of [`existing_variables`] and [`new_variables`] would still be a valid provider definition.
///
/// Uses the original registry validator to validate the merged variable list.
pub(crate) fn validate_var_list(
    existing_variables: &BTreeMap<u32, Variable>,
    new_variables: &[Variable],
) -> Result<(), InvalidProviderDefinitionError> {
    let merged_var_list = [
        new_variables
            .iter()
            .map(|var| var.into())
            .collect::<Vec<_>>()
            .as_slice(),
        existing_variables
            .values()
            .map(|dh_var| dh_var.into())
            .collect::<Vec<_>>()
            .as_slice(),
    ]
    .concat();

    let tmp_provider_def = ProviderDefinitionT {
        variable_definitions: Some(merged_var_list),
        ..Default::default() //Other fields are irrelevant for validation
    };

    tmp_provider_def.validate()
}

/// Error that can occur when adding a variable.
#[derive(Error, Debug, PartialEq)]
pub enum AddVariablesError {
    /// Something went wrong while validating the merged variable list
    #[error("Invalid merged variable list: `{0}`")]
    InvalidMergedVariableList(#[from] InvalidProviderDefinitionError),
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
