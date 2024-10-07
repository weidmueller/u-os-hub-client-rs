//! Contains oauth2 utils for providers.

use std::collections::HashMap;

use thiserror::Error;

use crate::oauth2::{OAuth2Credentials, TokenResponse};

/// Credentials for the oauth2 client credentials flow.
pub struct OAuth2ProviderCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl OAuth2ProviderCredentials {
    /// Executes the oauth2 client credentials flow to request an access token.
    /// The access token will be fetched with the `hub.variables.provide` scope.
    pub(super) async fn request_token<T: AsRef<str>>(
        &self,
        token_endpoint: T,
    ) -> Result<TokenResponse, String> {
        let credentials = OAuth2Credentials {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
        };

        credentials
            .request_token(token_endpoint, "hub.variables.provide")
            .await
    }
}

impl TryFrom<HashMap<String, String>> for OAuth2ProviderCredentials {
    type Error = ParsingError;

    fn try_from(value: HashMap<String, String>) -> Result<Self, ParsingError> {
        let client_id = value
            .get("CLIENT_ID")
            .ok_or(ParsingError::MissingKey("CLIENT_ID".to_string()))?
            .clone();

        let client_secret = value
            .get("CLIENT_SECRET")
            .ok_or(ParsingError::MissingKey("CLIENT_SECRET".to_string()))?
            .clone();

        Ok(Self {
            client_id,
            client_secret,
        })
    }
}

/// Parsing error
#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("Missing key `{0}`")]
    MissingKey(String),
}
