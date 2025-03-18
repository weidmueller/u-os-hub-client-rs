#![allow(dead_code)]
///As integration tests are compiled into their own crates, this leads to unused warnings they dont use all functions of the utils module
pub mod fake_registry;

use std::{future::IntoFuture, sync::Arc, time::Duration};

use tokio::time::timeout;
use u_os_hub_client::{
    authenticated_nats_con::{
        AuthenticatedNatsConnection, AuthenticationSettingsBuilder, Permissions,
    },
    oauth2::OAuth2Credentials,
};

pub const NATS_HOSTNAME: &str = "nats://localhost:4222";

/// As some nats operations are async and may wait for a long time, we define a default timeout to avoid blocking tests forever.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn create_auth_con(provider_id: &str) -> Arc<AuthenticatedNatsConnection> {
    let auth_settings = AuthenticationSettingsBuilder::new(Permissions::Provide)
        .with_credentials(OAuth2Credentials {
            client_name: provider_id.to_owned(),
            client_id: "".to_owned(),
            client_secret: "".to_owned(),
        })
        .build();

    tracing::info!("Creating AuthenticatedNatsConnection");
    let con = timeout(
        Duration::from_secs(10),
        AuthenticatedNatsConnection::new(NATS_HOSTNAME, &auth_settings),
    )
    .await
    .unwrap()
    .unwrap();

    Arc::new(con)
}

pub async fn run_with_timeout<F>(future: F) -> <F as IntoFuture>::Output
where
    F: IntoFuture,
{
    timeout(DEFAULT_TIMEOUT, future).await.expect("Timeout")
}
