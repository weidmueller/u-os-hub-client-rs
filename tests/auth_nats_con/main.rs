use std::{collections::HashSet, time::Duration};

use serial_test::serial;
use tokio::time::timeout;
use u_os_hub_client::{
    authenticated_nats_con::{
        AuthenticatedNatsConnection, AuthenticationSettingsBuilder, NatsPermission,
    },
    oauth2::OAuth2Credentials,
};
use utils::{run_with_timeout, NATS_HOSTNAME};

#[path = "../utils/mod.rs"]
mod utils;

#[tokio::test]
#[serial]
async fn test_successful_con() {
    let auth_settings =
        AuthenticationSettingsBuilder::new(NatsPermission::VariableHubProvide).build();

    let con_result = run_with_timeout(AuthenticatedNatsConnection::new(
        NATS_HOSTNAME,
        &auth_settings,
    ))
    .await;

    assert!(con_result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_default_name_and_single_perms() {
    let auth_settings =
        AuthenticationSettingsBuilder::new(NatsPermission::VariableHubProvide).build();

    let con = run_with_timeout(AuthenticatedNatsConnection::new(
        NATS_HOSTNAME,
        &auth_settings,
    ))
    .await
    .unwrap();

    assert!(con.get_client_name() == "_UNAUTHENTICATED");
    assert!(
        con.get_permissions()
            == ([NatsPermission::VariableHubProvide.to_string()]
                .into_iter()
                .collect::<HashSet<_>>())
    );
}

#[tokio::test]
#[serial]
async fn test_custom_name_and_multi_perms() {
    let auth_settings = AuthenticationSettingsBuilder::new(NatsPermission::VariableHubProvide)
        .add_permission(NatsPermission::VariableHubRead)
        .add_permission(NatsPermission::VariableHubReadWrite)
        .with_credentials(OAuth2Credentials {
            client_name: "test_client".to_string(),
            client_secret: "".to_string(),
            client_id: "".to_string(),
        })
        .build();

    let con = run_with_timeout(AuthenticatedNatsConnection::new(
        NATS_HOSTNAME,
        &auth_settings,
    ))
    .await
    .unwrap();

    assert!(con.get_client_name() == "test_client");
    assert!(
        con.get_permissions()
            == ([
                NatsPermission::VariableHubRead.to_string(),
                NatsPermission::VariableHubReadWrite.to_string(),
                NatsPermission::VariableHubProvide.to_string()
            ]
            .into_iter()
            .collect::<HashSet<_>>())
    );
}

#[tokio::test]
#[serial]
async fn test_server_offline() {
    let auth_settings =
        AuthenticationSettingsBuilder::new(NatsPermission::VariableHubProvide).build();

    let con_result = run_with_timeout(AuthenticatedNatsConnection::new(
        "nats://localhost:4223".to_string(),
        &auth_settings,
    ))
    .await;

    assert!(con_result.is_err());
}

#[tokio::test]
#[serial]
async fn test_creds_with_invalid_auth_server() {
    let auth_settings = AuthenticationSettingsBuilder::new(NatsPermission::VariableHubProvide)
        .with_credentials(OAuth2Credentials {
            client_name: "test_client".to_string(),
            client_secret: "somepass".to_string(),
            client_id: "someid".to_string(),
        })
        //doesnt exist in dev container
        .with_custom_oauth2_endpoint("https://127.0.0.1/oauth2/token")
        .build();

    let timeout_result = timeout(
        Duration::from_secs(2),
        AuthenticatedNatsConnection::new(NATS_HOSTNAME, &auth_settings),
    )
    .await;

    //should run into timeout due to async con failure
    assert!(timeout_result.is_err());
}

#[tokio::test]
#[serial]
async fn test_invalid_auth_server_but_no_creds() {
    let auth_settings = AuthenticationSettingsBuilder::new(NatsPermission::VariableHubProvide)
        //doesnt exist in dev container
        .with_custom_oauth2_endpoint("https://127.0.0.1/oauth2/token")
        .build();

    //This should work even with the invalid endpoint, as we didnt supply any credentials
    let con_result = run_with_timeout(AuthenticatedNatsConnection::new(
        NATS_HOSTNAME,
        &auth_settings,
    ))
    .await;

    assert!(con_result.is_ok());
}
