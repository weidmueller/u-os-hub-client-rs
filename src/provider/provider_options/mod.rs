//! Contains the provider builder which is need to create a provider.
use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};

use async_nats::{Client, ConnectOptions};
use thiserror::Error;
use tokio::time::interval;
use tracing::{debug, warn};

use crate::{provider::worker::ProviderWorker, variable::Variable};

use super::{oauth2::OAuth2ProviderCredentials, Provider};

#[cfg(test)]
mod provider_options_test;

/// The ProviderOptions is used to create a Provider
pub struct ProviderOptions {
    provider_id: String,
    credentials: Option<ProviderCredentials>,
    oauth2_endpoint: Option<String>,
    variables: BTreeMap<u32, Variable>,
}

impl ProviderOptions {
    /// Create a new provider builder
    /// The provider_id must be equal with the application name and the oauth2 client name.
    pub fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            credentials: None,
            variables: BTreeMap::new(),
            oauth2_endpoint: None,
        }
    }

    /// Adds credentials. This is not required for anonymous access.
    pub fn with_credentials(mut self, creds: ProviderCredentials) -> Self {
        self.credentials = Some(creds);
        self
    }

    /// Overwrites the default oauth2 endpoint
    pub fn with_oauth2_endpoint(mut self, endpoint: String) -> Self {
        self.oauth2_endpoint = Some(endpoint);
        self
    }

    /// Add multiple variable. They will be registerd on [`Self::register_and_connect()`].
    pub fn add_variables(mut self, vars: Vec<Variable>) -> Result<Self, AddVariablesError> {
        check_for_duplicates(&self.variables, &vars)?;

        let new_variables: BTreeMap<u32, Variable> =
            vars.iter().map(|var| (var.id, var.clone())).collect();
        self.variables.extend(new_variables);
        Ok(self)
    }

    /// Connect to nats and register the provider to the registry.
    ///
    /// If [`ProviderCredentials::Oauth2`] is used a access token will be fetched first.
    pub async fn register_and_connect(self, nats_hostname: &str) -> Result<Provider, ConnectError> {
        let client = self.fetch_token_connect_to_nats(nats_hostname).await?;

        debug!(
            "Register `{}` variables at creation time",
            self.variables.len()
        );
        let control_tx =
            ProviderWorker::new(client, self.provider_id, self.variables, true).await?;

        Ok(Provider::new(control_tx))
    }

    /// Fetch an access token and connect to nats
    ///
    /// This will be repeated because the provider could starts before the oauth2 provider and the nats server.
    ///
    /// If it fails, it is repeated once every 5 seconds, 20 times in total.
    async fn fetch_token_connect_to_nats(
        &self,
        nats_hostname: &str,
    ) -> Result<Client, ConnectError> {
        let mut retry_interval = interval(Duration::from_secs(5));
        let mut attempts = 0;
        loop {
            // Wait for next try (The first tick completes immediately)
            retry_interval.tick().await;
            match Self::fetch_token_and_connect_to_nats_once(self, nats_hostname).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    warn!(
                        "Connection to u-OS Data Hub failed: {}. Attempt: {}",
                        e,
                        attempts + 1
                    );
                    if attempts >= 19 {
                        return Err(e);
                    }
                    attempts += 1;
                }
            }
        }
    }

    /// Fetch an access token and connect to nats
    async fn fetch_token_and_connect_to_nats_once(
        &self,
        nats_hostname: &str,
    ) -> Result<Client, ConnectError> {
        let client = ConnectOptions::new().name(&self.provider_id);

        let client = match &self.credentials {
            Some(ProviderCredentials::Oauth2(creds)) => client.token(
                creds
                    .request_token(
                        &self
                            .oauth2_endpoint
                            .clone()
                            .unwrap_or("https://127.0.0.1/oauth2/token".to_string()),
                    )
                    .await
                    .map_err(ConnectError::OAuth)?
                    .access_token,
            ),
            Some(ProviderCredentials::Token(token)) => client.token(token.to_string()),
            None => client,
        };

        let client = client
            .custom_inbox_prefix(format!("_INBOX.{}", self.provider_id))
            .connect(nats_hostname)
            .await
            .map_err(|x| ConnectError::Nats(Box::new(x)))?;
        Ok(client)
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

/// The ProviderCredentials enum is used to store the credentials of the provider.
/// Oauth2 is used to store the client credentials direct.
/// Token is used to store a token.
pub enum ProviderCredentials {
    /// Stores oauth2 client credentials
    Oauth2(OAuth2ProviderCredentials),
    /// Stores an access token
    Token(String),
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
    Nats(async_nats::Error),
    /// Indicates an error with the OAuth2 token request
    #[error("Oauth2 error: `{0}`")]
    OAuth(String),
    /// Indicates an error with the registration at the registry
    #[error("Error while sending the provider definition: `{0}`")]
    UpdateProviderDefinition(UpdateProviderDefinitionError),
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
