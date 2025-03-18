use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use serial_test::serial;
use u_os_hub_client::{
    authenticated_nats_con::{
        AuthenticatedNatsConnection, AuthenticationSettings, AuthenticationSettingsBuilder,
        Permissions,
    },
    consumer::dh_consumer::DataHubConsumer,
    oauth2::OAuth2Credentials,
};

use crate::{
    dummy_provider::{DummyProvider, PROVIDER_ID},
    utils::{fake_registry::FakeRegistry, run_with_timeout, NATS_HOSTNAME},
};

const CONSUMER_ID: &str = "test_consumer";

fn consumer_auth_settings(perms: Permissions) -> AuthenticationSettings {
    AuthenticationSettingsBuilder::new(perms)
        .with_credentials(OAuth2Credentials {
            client_name: CONSUMER_ID.to_string(),
            client_id: "".to_owned(),
            client_secret: "".to_owned(),
        })
        .build()
}

#[tokio::test]
#[serial]
async fn test_connect() {
    run_with_timeout(async move {
        let auth_settings = consumer_auth_settings(Permissions::Read);
        let con_result = DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings).await;
        assert!(con_result.is_ok());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_reuse_con() {
    run_with_timeout(async move {
        let auth_settings = consumer_auth_settings(Permissions::Read);
        let con = Arc::new(
            AuthenticatedNatsConnection::new(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let con_result = DataHubConsumer::from_existing_connection(con).await;
        assert!(con_result.is_ok());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_invalid_scope() {
    run_with_timeout(async move {
        let auth_settings = consumer_auth_settings(Permissions::Provide);
        let con_result = DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings).await;
        assert!(con_result.is_err());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn registry_offline() {
    run_with_timeout(async move {
        let auth_settings = consumer_auth_settings(Permissions::Read);
        let consumer = DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
            .await
            .unwrap();

        //subscription should work
        let sub_result = consumer.subscribe_provider_ids().await;
        assert!(sub_result.is_ok());

        //reading should fail
        let read_result = consumer.read_provider_ids().await;
        assert!(read_result.is_err());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn monitor_providers() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        //create consumer
        let auth_settings = consumer_auth_settings(Permissions::Read);
        let consumer = DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
            .await
            .unwrap();

        let mut providers_changed_sub = consumer.subscribe_provider_ids().await.unwrap();

        //read ids before registering a provider
        let provider_ids = consumer.read_provider_ids().await.unwrap();
        assert!(provider_ids.is_empty());

        //register a dummy provider after a slight delay
        let dummy_provider = DummyProvider::new_with_delay(Duration::from_millis(100))
            .await
            .unwrap();

        //Wait for the provider to be registered
        consumer.wait_for_provider(PROVIDER_ID).await.unwrap();

        //Should return immediately if the provider is already registered
        consumer.wait_for_provider(PROVIDER_ID).await.unwrap();

        //read ids again
        let provider_ids = consumer.read_provider_ids().await.unwrap();
        assert!(provider_ids.len() == 1);
        assert!(provider_ids[0] == PROVIDER_ID);

        //Double check the last providers changed event and check if the provider id was in the list
        if let Some(providers_changed_evt) = providers_changed_sub.next().await {
            assert!(providers_changed_evt.len() == 1);
            assert!(providers_changed_evt[0] == PROVIDER_ID);
        } else {
            panic!("No providers changed event received after adding a provider");
        }

        //destroy dummy provider to trigger unregister
        drop(dummy_provider);

        //Wait for providers changed event and check if the provider id is no longer in the list
        if let Some(providers_changed_evt) = providers_changed_sub.next().await {
            assert!(providers_changed_evt.is_empty());
        } else {
            panic!("No providers changed event received after removal");
        }

        //list should be empty now
        let provider_ids = consumer.read_provider_ids().await.unwrap();
        assert!(provider_ids.is_empty());
    })
    .await;
}
