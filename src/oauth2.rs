//! Contains some utils for oauth2 handling
use base64::{prelude::BASE64_STANDARD, Engine};
use hyper::header::AUTHORIZATION;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Contains OAuth2 client credentials
pub struct OAuth2Credentials {
    pub client_id: String,
    pub client_secret: String,
}

impl OAuth2Credentials {
    /// Executes the oauth2 client credentials flow to request an access token
    pub async fn request_token<T: AsRef<str>, T2: AsRef<str>>(
        &self,
        token_endpoint: T,
        scope: T2,
    ) -> Result<TokenResponse, String> {
        let reqwest_client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|x| x.to_string())?;

        let mut params = BTreeMap::new();
        params.insert("grant_type", "client_credentials");
        params.insert("scope", scope.as_ref());

        let credentials = format!(
            "{}:{}",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.client_secret)
        );

        let response: TokenResponse = reqwest_client
            .post(token_endpoint.as_ref())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .header(
                AUTHORIZATION,
                format!("Basic {}", BASE64_STANDARD.encode(credentials)),
            )
            .form(&params)
            .send()
            .await
            .map_err(|x| x.to_string())?
            .json()
            .await
            .map_err(|x| x.to_string())?;

        Ok(response)
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
