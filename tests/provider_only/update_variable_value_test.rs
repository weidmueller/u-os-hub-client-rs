use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use serial_test::serial;
use tokio::time::timeout;
use u_os_hub_client::{
    authenticated_nats_con::NatsPermission,
    consumer::{connected_nats_provider::ConnectedNatsProvider, nats_consumer::NatsConsumer},
    provider::{ProviderOptions, UpdateVariableValuesError, VariableBuilder},
    variable::value::Value,
};

use crate::utils::{self, fake_registry::FakeRegistry};

const PROVIDER_ID: &str = "update_variable_value_test";

#[tokio::test]
#[serial]
async fn test_update_variable_value() {
    // Prepare
    let _fake_registry = FakeRegistry::new().await;
    let auth_nats_con = utils::create_auth_con(PROVIDER_ID).await;

    let provider_builder = ProviderOptions::new();
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .value(Value::Boolean(true))
        .build()
        .expect("variable should build");

    let mut var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .value(Value::String("Test_it".to_string()))
        .build()
        .expect("variable should build");

    let provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register(auth_nats_con)
        .await
        .expect("provider should register");

    // act

    var2.value = Value::String("Test_String123".to_string());
    provider
        .update_variable_values(vec![var2.clone()])
        .await
        .expect("writing a variable with the same type should work");

    var2.value = Value::Int(2);
    let result = provider.update_variable_values(vec![var2]).await;

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

    let provider_builder = ProviderOptions::new();
    let mut var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .value(Value::Boolean(true))
        .build()
        .expect("variable should build");

    let mut var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .value(Value::String("Test_it".to_string()))
        .build()
        .expect("variable should build");

    let provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register(auth_nats_con)
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
        var1.value = Value::Boolean(false);
        var2.value = Value::String("Test_String123".to_string());
        provider
            .update_variable_values(vec![var1.clone(), var2.clone()])
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
        var2.value = Value::String("Test_String123".to_string());
        provider
            .update_variable_values(vec![var2.clone()])
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
            .value(123.into())
            .build()
            .unwrap();
        provider.add_variables(&[new_var.clone()]).await.unwrap();

        var2.value = Value::String("Test_String1234".to_string());
        provider
            .update_variable_values(vec![var2.clone()])
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
