//! Contains some utils for oauth2 handling
use base64::{prelude::BASE64_STANDARD, Engine};
use hyper::header::AUTHORIZATION;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

use crate::env_file_parser::read_and_parse_env_file;

/// Contains OAuth2 client credentials
#[derive(Clone, Debug)]
pub struct OAuth2Credentials {
    pub client_name: String,
    pub client_id: String,
    pub client_secret: String,
}

impl OAuth2Credentials {
    /// Creates new [OAuth2Credentials] from the provided client name and credentials file.
    ///
    /// The file must be a key=value INI like file, with the keys being `CLIENT_ID` and `CLIENT_SECRET`.
    pub async fn from_env_file(
        client_name: impl Into<String>,
        credentials_file: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let env_vars = read_and_parse_env_file(credentials_file).await?;

        let client_name = client_name.into();
        let client_id = env_vars
            .get("CLIENT_ID")
            .ok_or(anyhow::anyhow!("Can't get CLIENT_ID"))?
            .clone();
        let client_secret = env_vars
            .get("CLIENT_SECRET")
            .ok_or(anyhow::anyhow!("Can't get CLIENT_SECRET"))?
            .clone();

        Ok(Self {
            client_name,
            client_id,
            client_secret,
        })
    }

    /// Executes the oauth2 client credentials flow to request an access token
    pub async fn request_token<T: AsRef<str>, T2: AsRef<str>>(
        &self,
        token_endpoint: T,
        scope: T2,
    ) -> anyhow::Result<TokenResponse> {
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

        let response_text = http_response.text().await?;
        let json_body = serde_json::from_str(&response_text).map_err(|e| {
            anyhow::anyhow!(format!(
                "Failed to convert response body to json: {e} (Response was: {response_text}"
            ))
        })?;

        Ok(json_body)
    }
}

/// The reponse of the client credentials flow
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u128,
    pub scope: String,
    pub token_type: String,
}
