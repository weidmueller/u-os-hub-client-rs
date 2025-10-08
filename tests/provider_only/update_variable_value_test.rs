// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use serial_test::serial;
use tokio::time::timeout;
use u_os_hub_client::{
    authenticated_nats_con::NatsPermission,
    consumer::{
        connected_dh_provider::DataHubProviderConnection,
        connected_nats_provider::ConnectedNatsProvider, dh_consumer::DataHubConsumer,
        nats_consumer::NatsConsumer, variable_key::VariableKey,
    },
    dh_types::{DurationValue, TimestampValue, VariableQuality, VariableValue},
    provider::{ProviderBuilder, UpdateVariableValuesError, VariableBuilder},
};

use crate::utils::{self, fake_registry::FakeRegistry};

const PROVIDER_ID: &str = "update_variable_value_test";

#[tokio::test]
#[serial]
async fn test_update_variable_value() {
    // Prepare
    let _fake_registry = FakeRegistry::new().await;
    let auth_nats_con = utils::create_auth_con(PROVIDER_ID).await;

    let provider_builder = ProviderBuilder::new();
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .initial_value(true)
        .build()
        .expect("variable should build");

    let mut var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .initial_value("Test_it")
        .build()
        .expect("variable should build");

    let provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register_with_existing_connection(auth_nats_con)
        .await
        .expect("provider should register");

    // act

    var2.get_mut_state().set_value("Test_String123");
    provider
        .update_variable_states(vec![var2.get_state().clone()])
        .await
        .expect("writing a variable with the same type should work");

    var2.get_mut_state().set_value(2);
    let result = provider
        .update_variable_states(vec![var2.get_state().clone()])
        .await;

    // assert
    if let Err(UpdateVariableValuesError::TypeMismatch(key)) = result {
        assert_eq!(key, "my_folder.my_variable_2");
    } else {
        panic!("Writing another value type on a variable should fail");
    }
}

#[tokio::test]
#[serial]
async fn test_update_variable_fingerprint() {
    // Prepare
    let _fake_registry = FakeRegistry::new().await;
    let auth_nats_con = utils::create_auth_con(PROVIDER_ID).await;

    let auth_nats_con_consumer =
        utils::create_auth_con_with_perm("test-consumer", NatsPermission::VariableHubRead).await;
    let nats_consumer = Arc::new(
        NatsConsumer::new(auth_nats_con_consumer)
            .await
            .expect("consumer should be created"),
    );

    let provider_builder = ProviderBuilder::new();
    let mut var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .initial_value(true)
        .build()
        .expect("variable should build");

    let mut var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .initial_value("Test_it")
        .build()
        .expect("variable should build");

    let provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register_with_existing_connection(auth_nats_con)
        .await
        .expect("provider should register");

    //record initial fingerprint of provider
    nats_consumer.wait_for_provider(PROVIDER_ID).await.unwrap();
    let connected_nats_provider = ConnectedNatsProvider::new(nats_consumer, PROVIDER_ID)
        .await
        .unwrap();
    let initial_fp = connected_nats_provider
        .read_provider_definition()
        .await
        .unwrap()
        .provider_definition
        .unwrap()
        .fingerprint;

    let mut var_change_sub = connected_nats_provider.subscribe_variables().await.unwrap();

    //update all vars - fingerprint should match initial fingerprint
    {
        var1.get_mut_state().set_value(false);
        var2.get_mut_state().set_value("Test_String123");
        provider
            .update_variable_states(vec![var1.get_state().clone(), var2.get_state().clone()])
            .await
            .expect("writing a variable with the same type should work");

        let var_changed_event = timeout(Duration::from_secs(1), var_change_sub.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(var_changed_event.changed_variables.items.unwrap().len(), 2);

        let var_changed_event_fp = var_changed_event
            .changed_variables
            .provider_definition_fingerprint;
        assert_eq!(var_changed_event_fp, initial_fp);
    }

    //update single var - fingerprint should match initial fingerprint
    {
        var1.get_mut_state().set_value("Test_String123");
        provider
            .update_variable_states(vec![var2.get_state().clone()])
            .await
            .expect("writing a variable with the same type should work");

        let var_changed_event = timeout(Duration::from_secs(1), var_change_sub.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(var_changed_event.changed_variables.items.unwrap().len(), 1);

        let var_changed_event_fp = var_changed_event
            .changed_variables
            .provider_definition_fingerprint;
        assert_eq!(var_changed_event_fp, initial_fp);
    }

    //change vars - fingerprint should change
    {
        let new_var = VariableBuilder::new(2, "my_folder.my_variable_3")
            .initial_value(123)
            .build()
            .unwrap();
        provider.add_variables(vec![new_var.clone()]).await.unwrap();

        var1.get_mut_state().set_value("Test_String1234");
        provider
            .update_variable_states(vec![var2.get_state().clone()])
            .await
            .expect("writing a variable with the same type should work");

        let var_changed_event = timeout(Duration::from_secs(1), var_change_sub.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(var_changed_event.changed_variables.items.unwrap().len(), 1);

        let var_changed_event_fp = var_changed_event
            .changed_variables
            .provider_definition_fingerprint;
        assert_ne!(var_changed_event_fp, initial_fp);
    }
}

/// Tests some advanced cases of the variable state and consumer interaction.
#[tokio::test]
#[serial]
async fn test_quality_and_timestamp() {
    let _fake_registry = FakeRegistry::new().await;

    // Create the provider
    let auth_nats_con = utils::create_auth_con(PROVIDER_ID).await;

    let provider_var_id = 10;
    let provider_var_key = "test_var";

    let provider_builder = ProviderBuilder::new();
    let mut provider_var = VariableBuilder::new(provider_var_id, provider_var_key)
        .initial_value(-1)
        .initial_quality(VariableQuality::Uncertain)
        .initial_timestamp(None)
        .build()
        .unwrap();

    let provider = provider_builder
        .add_variables(vec![provider_var.clone()])
        .unwrap()
        .register_with_existing_connection(auth_nats_con)
        .await
        .unwrap();

    // Create a consumer and connect to the provider
    let consumer_nats_con =
        utils::create_auth_con_with_perm("test-consumer", NatsPermission::VariableHubRead).await;

    let consumer = Arc::new(
        DataHubConsumer::from_existing_connection(consumer_nats_con)
            .await
            .unwrap(),
    );

    let dh_provider_con = timeout(
        Duration::from_secs(1),
        DataHubProviderConnection::new(consumer, PROVIDER_ID, true),
    )
    .await
    .unwrap()
    .unwrap();

    //subscribe to variable changes
    let mut change_stream = dh_provider_con
        .subscribe_variables_with_filter(Option::<Vec<VariableKey>>::None)
        .await
        .unwrap();

    //read initial state of variable, see if timestamp inheritance works and state is correct
    let var_state = dh_provider_con
        .read_single_variable(provider_var_key)
        .await
        .unwrap();

    assert_eq!(var_state.value, VariableValue::Int(-1));
    assert_eq!(var_state.quality, VariableQuality::Uncertain);
    //Even though the provider variable has no timestamp, the consumer should inherit the current time from the var list
    assert!((TimestampValue::now() - var_state.timestamp) < DurationValue::seconds(1));

    //Change the variable state on the provider
    let provider_var_state = provider_var.get_mut_state();
    provider_var_state.set_all(
        3,
        VariableQuality::UncertainLastUsableValue,
        Some(TimestampValue::from_unix_timestamp(123456).unwrap()),
    );
    provider
        .update_variable_states(vec![provider_var_state.clone()])
        .await
        .unwrap();

    //Wait for changed event on consumer
    let var_changed_event = timeout(Duration::from_secs(1), change_stream.next())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(var_changed_event.len(), 1);
    let changed_var = var_changed_event.first().unwrap();
    assert_eq!(changed_var.0, provider_var_id);

    //Check if the variable state is correct
    let var_state = &changed_var.1;
    assert_eq!(var_state.value, VariableValue::Int(3));
    assert_eq!(var_state.quality, VariableQuality::UncertainLastUsableValue);
    //Variable should no longer inherit the timestamp from the variable list
    assert_eq!(
        var_state.timestamp,
        TimestampValue::from_unix_timestamp(123456).unwrap()
    );

    //Change the variable state on the provider once more
    let provider_var_state = provider_var.get_mut_state();
    provider_var_state.set_value(30);
    provider_var_state.set_quality(VariableQuality::Good);
    provider
        .update_variable_states(vec![provider_var_state.clone()])
        .await
        .unwrap();

    //Wait for changed event on consumer
    let var_changed_event = timeout(Duration::from_secs(1), change_stream.next())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(var_changed_event.len(), 1);
    let changed_var = var_changed_event.first().unwrap();
    assert_eq!(changed_var.0, provider_var_id);

    //Check if the variable state is correct
    let var_state = &changed_var.1;
    assert_eq!(var_state.value, VariableValue::Int(30));
    assert_eq!(var_state.quality, VariableQuality::Good);
    //Variable timestamp should be recent
    assert!((TimestampValue::now() - var_state.timestamp) < DurationValue::seconds(1));
}
