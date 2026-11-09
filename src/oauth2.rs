// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

//! Provides utilities for `OAuth2` handling
use base64::{Engine, prelude::BASE64_STANDARD};
use hyper::header::AUTHORIZATION;
use rustls::{
    ClientConfig, DigitallySignedStruct, Error as RustlsError, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, WebPkiSupportedAlgorithms},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path, sync::Arc};
use thiserror::Error;
use tracing::debug;

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
    #[error("Failed to configure TLS: {0}")]
    Tls(#[from] RustlsError),
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
    ///
    /// Uses the configured TLS crypto provider or a request-local `ring` fallback.
    pub async fn request_token<T: AsRef<str>, T2: AsRef<str>>(
        &self,
        token_endpoint: T,
        scope: T2,
    ) -> Result<TokenResponse, OAuth2RequestTokenError> {
        let reqwest_client = request_client()?;

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

/// Creates and returns a configured `reqwest::Client` for `OAuth2` client credentials requests.
///
/// If a global `CryptoProvider` is available, the client will bypass certificate validation.
/// Otherwise, it will use a request-local `ring` provider with a custom certificate verifier.
///
/// Certificate validation is bypassed because u-OS apps currently have no way to access
/// the web server's certificate for proper validation.
/// Certificate validation will be enabled once a suitable mechanism is available.
/// This is currently not critical because the endpoint is local to the device.
///
/// # Errors
/// Returns an `OAuth2RequestTokenError` if the client cannot be built or if TLS configuration fails.
fn request_client() -> Result<reqwest::Client, OAuth2RequestTokenError> {
    let builder = reqwest::ClientBuilder::new();

    // The target endpoint is local to the device.
    // TODO: Enable certificate validation once u-OS apps can obtain the web server certificate.
    if CryptoProvider::get_default().is_some() {
        return Ok(builder.tls_danger_accept_invalid_certs(true).build()?);
    }

    debug!(
        "No global CryptoProvider found, using ring default provider for the Client Credentials request"
    );
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = AcceptInvalidCertificates {
        signature_algorithms: provider.signature_verification_algorithms,
    };

    // Preconfigured TLS bypasses reqwest's tls_danger_accept_invalid_certs setting.
    let tls_config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();

    Ok(builder.tls_backend_preconfigured(tls_config).build()?)
}

#[derive(Debug)]
struct AcceptInvalidCertificates {
    signature_algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for AcceptInvalidCertificates {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.signature_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.signature_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.signature_algorithms.supported_schemes()
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
