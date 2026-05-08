// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

//! Provides utilities for `OAuth2` handling
use base64::{prelude::BASE64_STANDARD, Engine};
use hyper::header::AUTHORIZATION;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};
use thiserror::Error;

use crate::env_file_parser;

/// Error type for creating [`OAuth2Credentials`] from an env file
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum OAuth2CredentialsFromEnvFileError {
    #[error("Failed to read credentials file: {0}")]
    EnvFile(#[from] env_file_parser::Error),
    #[error("Missing credential field: {0}")]
    MissingField(&'static str),
}

/// Error type for `OAuth2` request token operations
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum OAuth2RequestTokenError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Failed to obtain token: {error_response:?}, status code: {http_status_code}")]
    ErrorResponse {
        error_response: Oauth2ErrorResponse,
        http_status_code: u16,
    },
    #[error("Failed to deserialize response: {0}")]
    FailedToDeserializeResponse(#[from] serde_json::Error),
}

/// Contains `OAuth2` client credentials
#[derive(Clone, Debug)]
pub struct OAuth2Credentials {
    pub client_name: String,
    pub client_id: String,
    pub client_secret: String,
}

impl OAuth2Credentials {
    /// Creates new [`OAuth2Credentials`] from the provided client name and credentials file.
    ///
    /// The file must be a .env file which contains key=value pairs, with the keys being `CLIENT_ID` and `CLIENT_SECRET`.
    pub async fn from_env_file(
        client_name: impl Into<String>,
        credentials_file: impl AsRef<Path>,
    ) -> Result<Self, OAuth2CredentialsFromEnvFileError> {
        let env_vars = env_file_parser::read_and_parse_env_file(credentials_file).await?;

        let client_name = client_name.into();
        let client_id = env_vars
            .get("CLIENT_ID")
            .ok_or(OAuth2CredentialsFromEnvFileError::MissingField("CLIENT_ID"))?
            .clone();
        let client_secret = env_vars
            .get("CLIENT_SECRET")
            .ok_or(OAuth2CredentialsFromEnvFileError::MissingField(
                "CLIENT_SECRET",
            ))?
            .clone();

        Ok(Self {
            client_name,
            client_id,
            client_secret,
        })
    }

    /// Executes the `OAuth2` client credentials flow to request an access token
    pub async fn request_token<T: AsRef<str>, T2: AsRef<str>>(
        &self,
        token_endpoint: T,
        scope: T2,
    ) -> Result<TokenResponse, OAuth2RequestTokenError> {
        let reqwest_client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;

        let mut params = BTreeMap::new();
        params.insert("grant_type", "client_credentials");
        params.insert("scope", scope.as_ref());

        let credentials = format!(
            "{}:{}",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.client_secret)
        );

        let http_response = reqwest_client
            .post(token_endpoint.as_ref())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .header(
                AUTHORIZATION,
                format!("Basic {}", BASE64_STANDARD.encode(credentials)),
            )
            .form(&params)
            .send()
            .await?;

        let status = http_response.status();
        let response_text = http_response.text().await?;

        if status.is_success() {
            let token_response = serde_json::from_str(&response_text)
                .map_err(OAuth2RequestTokenError::FailedToDeserializeResponse)?;

            Ok(token_response)
        } else {
            let error_response = serde_json::from_str(&response_text)
                .map_err(OAuth2RequestTokenError::FailedToDeserializeResponse)?;

            Err(OAuth2RequestTokenError::ErrorResponse {
                error_response,
                http_status_code: status.as_u16(),
            })
        }
    }
}

/// The response of the client credentials flow
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u128,
    pub scope: String,
    pub token_type: String,
}

/// Contains details about an error response from the token endpoint
#[derive(Deserialize, Debug)]
pub struct Oauth2ErrorResponse {
    pub error: Option<String>,
    pub error_description: Option<String>,
    pub error_hint: Option<String>,
}
