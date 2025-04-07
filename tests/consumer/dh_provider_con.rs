use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::StreamExt;
use serial_test::serial;
use tokio::time::timeout;
use u_os_hub_client::{
    authenticated_nats_con::{
        AuthenticationSettings, AuthenticationSettingsBuilder, NatsPermission,
    },
    consumer::{
        connected_dh_provider::{self, ConnectedDataHubProvider, ProviderEvent},
        connected_nats_provider::{self},
        dh_consumer::DataHubConsumer,
        dh_types::{ConsumerVariableDefinition, ConsumerVariableQuality, ConsumerVariableType},
    },
    oauth2::OAuth2Credentials,
    prelude::consumer::{ConsumerVariableState, VariableID, VariableKey},
    variable::value::Value,
};

use crate::{
    dummy_provider::{self, DummyProvider, PROVIDER_ID},
    utils::{fake_registry::FakeRegistry, run_with_timeout, NATS_HOSTNAME},
};

/// Things that can go wrong while we are connected to a provider:
///
/// - Provider definiton changes -> internal var definitions / mappings should get updated.
///     subscriptions should continue to run, but filters must be updated internally.
///     if variable key -> id mappings change, the subscriptions should filter based on the new key -> id mapping
///     if the var key no longer exists, stream should no longer yield values
///
/// - Provider goes offline -> subscriptions should stay in tact but not yield new values,
///     read and write variable commands fail,
///     mapping methods continue to work and use latest cached state.
///
/// - Provider comes back online -> same actions as if provider changes
///
/// - Registry goes offline -> we cant react to this, as the consumer doesnt get registry down events.

const CONSUMER_ID: &str = "test_consumer";

fn consumer_auth_settings(perms: NatsPermission) -> AuthenticationSettings {
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
async fn registry_offline() {
    run_with_timeout(async move {
        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubRead);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        //Should not be able to connect to a provider
        let con_result = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, false).await;
        assert!(con_result.is_err());

        //Should start waiting if flag is enabled
        let timeout_res = timeout(
            Duration::from_millis(100),
            ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true),
        )
        .await;
        assert!(timeout_res.is_err());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn provider_offline() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubRead);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let _dummy_provider = DummyProvider::new_with_delay(Duration::from_millis(100))
            .await
            .unwrap();

        //Should not be able to connect to a provider if wait is set to false
        let con_result = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, false).await;
        assert!(con_result.is_err());

        //this should work, but block until the provider appears
        let dh_provider_con = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true)
            .await
            .unwrap();

        //should have all var defs and states available
        let var_defs = dh_provider_con.get_all_variable_definitions().unwrap();
        assert_eq!(var_defs.len(), 4);

        let var_states = dh_provider_con
            .read_variables(Option::<&[VariableKey]>::None)
            .await
            .unwrap();
        assert_eq!(var_states.len(), 4);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn wait_for_variable_keys() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubRead);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let _dummy_provider = DummyProvider::new().await.unwrap();
        let dh_provider_con = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true)
            .await
            .unwrap();

        //empty list should return instantly
        assert!(dh_provider_con
            .wait_until_variable_keys_are_available(&[])
            .await
            .is_ok());

        //full list should return instantly
        assert!(dh_provider_con
            .wait_until_variable_keys_are_available(&[
                "my_folder.ro_float",
                "my_folder.rw_string",
                "my_folder.rw_int",
                "my_folder.ro_int",
            ])
            .await
            .is_ok());

        //partial list should return instantly
        assert!(dh_provider_con
            .wait_until_variable_keys_are_available(&["my_folder.ro_float", "my_folder.ro_int",])
            .await
            .is_ok());

        //should run into timeout for invalid keys
        let timeout_res = timeout(
            Duration::from_millis(100),
            dh_provider_con.wait_until_variable_keys_are_available(&["doesntexist"]),
        )
        .await;
        assert!(timeout_res.is_err());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn read_var_defs() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubRead);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let _dummy_provider = DummyProvider::new().await.unwrap();
        let dh_provider_con = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true)
            .await
            .unwrap();

        //invalid ids and keys should not work
        assert!(dh_provider_con.variable_key_from_id(999).is_err());
        assert!(dh_provider_con
            .get_variable_definition("unknown_key")
            .is_err());
        let invalid_key = VariableKey::from("unknown_key");
        assert!(dh_provider_con
            .get_variable_definition(invalid_key)
            .is_err());

        //read definitions one by one
        let ro_float = dh_provider_con.variable_key_from_id(100).unwrap();
        let ro_float_def = dh_provider_con.get_variable_definition(&ro_float).unwrap();
        assert_eq!(
            ro_float_def,
            ConsumerVariableDefinition {
                id: 100,
                key: "my_folder.ro_float".to_string(),
                read_only: true,
                experimental: true,
                data_type: ConsumerVariableType::Float64,
            }
        );
        let rw_string = dh_provider_con.variable_key_from_id(200).unwrap();
        let rw_string_def = dh_provider_con.get_variable_definition(&rw_string).unwrap();
        assert_eq!(
            rw_string_def,
            ConsumerVariableDefinition {
                id: 200,
                key: "my_folder.rw_string".to_string(),
                read_only: false,
                experimental: false,
                data_type: ConsumerVariableType::String,
            }
        );
        let rw_int = dh_provider_con.variable_key_from_id(300).unwrap();
        let rw_int_def = dh_provider_con.get_variable_definition(&rw_int).unwrap();
        assert_eq!(
            rw_int_def,
            ConsumerVariableDefinition {
                id: 300,
                key: "my_folder.rw_int".to_string(),
                read_only: false,
                experimental: false,
                data_type: ConsumerVariableType::Int64,
            }
        );
        let ro_int = dh_provider_con.variable_key_from_id(400).unwrap();
        let ro_int_def = dh_provider_con.get_variable_definition(&ro_int).unwrap();
        assert_eq!(
            ro_int_def,
            ConsumerVariableDefinition {
                id: 400,
                key: "my_folder.ro_int".to_string(),
                read_only: true,
                experimental: false,
                data_type: ConsumerVariableType::Int64,
            }
        );

        //Read all variable definitions at once
        let var_defs = dh_provider_con.get_all_variable_definitions().unwrap();
        assert_eq!(var_defs.len(), 4);

        assert!(var_defs.contains(&ConsumerVariableDefinition {
            id: 100,
            key: "my_folder.ro_float".to_string(),
            read_only: true,
            experimental: true,
            data_type: ConsumerVariableType::Float64,
        }));
        assert!(var_defs.contains(&ConsumerVariableDefinition {
            id: 200,
            key: "my_folder.rw_string".to_string(),
            read_only: false,
            experimental: false,
            data_type: ConsumerVariableType::String,
        }));
        assert!(var_defs.contains(&ConsumerVariableDefinition {
            id: 300,
            key: "my_folder.rw_int".to_string(),
            read_only: false,
            experimental: false,
            data_type: ConsumerVariableType::Int64,
        }));
        assert!(var_defs.contains(&ConsumerVariableDefinition {
            id: 400,
            key: "my_folder.ro_int".to_string(),
            read_only: true,
            experimental: false,
            data_type: ConsumerVariableType::Int64,
        }));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn read_var_state() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubRead);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let _dummy_provider = DummyProvider::new().await.unwrap();
        let dh_provider_con = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true)
            .await
            .unwrap();

        //read all at once
        let var_states = dh_provider_con
            .read_variables(Option::<&[VariableKey]>::None)
            .await
            .unwrap();

        assert_eq!(var_states.len(), 4);

        assert!(dh_provider_con.variable_id_from_key("unknown_key").is_err());

        let ro_float_id = dh_provider_con
            .variable_id_from_key("my_folder.ro_float")
            .unwrap();
        let ro_int_id = dh_provider_con
            .variable_id_from_key("my_folder.ro_int")
            .unwrap();
        let rw_string_id = dh_provider_con
            .variable_id_from_key("my_folder.rw_string")
            .unwrap();
        let rw_int_id = dh_provider_con
            .variable_id_from_key("my_folder.rw_int")
            .unwrap();

        let var_states: HashMap<VariableID, ConsumerVariableState> =
            var_states.into_iter().collect();

        let state = var_states.get(&ro_float_id).unwrap();
        assert_eq!(state.value, Value::Float64(123.0));
        assert_eq!(state.quality, ConsumerVariableQuality::Good);

        let state = var_states.get(&ro_int_id).unwrap();
        assert_eq!(state.value, Value::Int(0));
        assert_eq!(state.quality, ConsumerVariableQuality::Good);

        let state = var_states.get(&rw_string_id).unwrap();
        assert_eq!(state.value, Value::String("write me!".to_owned()));
        assert_eq!(state.quality, ConsumerVariableQuality::Good);

        let state = var_states.get(&rw_int_id).unwrap();
        assert_eq!(state.value, Value::Int(1000));
        assert_eq!(state.quality, ConsumerVariableQuality::Good);

        //read one by one via keys
        for def in dh_provider_con.get_all_variable_definitions().unwrap() {
            let var_key: &str = &def.key;
            let state = dh_provider_con.read_single_variable(var_key).await.unwrap();

            match var_key {
                "my_folder.ro_float" => {
                    assert_eq!(state.value, Value::Float64(123.0));
                    assert_eq!(state.quality, ConsumerVariableQuality::Good);
                }
                "my_folder.ro_int" => {
                    assert_eq!(state.value, Value::Int(0));
                    assert_eq!(state.quality, ConsumerVariableQuality::Good);
                }
                "my_folder.rw_string" => {
                    assert_eq!(state.value, Value::String("write me!".to_owned()));
                    assert_eq!(state.quality, ConsumerVariableQuality::Good);
                }
                "my_folder.rw_int" => {
                    assert_eq!(state.value, Value::Int(1000));
                    assert_eq!(state.quality, ConsumerVariableQuality::Good);
                }
                _ => panic!("Unexpected variable key: {var_key}"),
            }
        }
    })
    .await;
}

#[tokio::test]
#[serial]
async fn subscribe_variables() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubRead);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let dummy_provider = DummyProvider::new().await.unwrap();
        let dh_provider_con = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true)
            .await
            .unwrap();

        //The subscribe method allows to filter for keys that do not yet exist
        let mut change_stream = dh_provider_con
            .subscribe_variables_with_filter(Some(vec!["my_folder.ro_float", "my_folder.ro_int2"]))
            .await
            .unwrap();

        //wait for the some events
        let change_evt = change_stream.next().await.unwrap();
        assert_eq!(change_evt.len(), 1);
        let changed_var = &change_evt.first().unwrap().1;
        assert_eq!(changed_var.value, 0.0.into());
        assert_eq!(changed_var.quality, ConsumerVariableQuality::Good);

        let change_evt = change_stream.next().await.unwrap();
        assert_eq!(change_evt.len(), 1);
        let changed_var = &change_evt.first().unwrap().1;
        assert_eq!(changed_var.value, 123.0.into());
        assert_eq!(changed_var.quality, ConsumerVariableQuality::Good);

        //this will register a new variable and change IDs of existing vars
        dummy_provider.change_variables();

        //should now receive events for the old variable and the new variable,
        //even though the id of the old variable has changed and the new variable didnt exist during the initial subscription call.
        let change_evt = change_stream.next().await.unwrap();
        assert_eq!(change_evt.len(), 2);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn write_variables() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubReadWrite);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let _dummy_provider = DummyProvider::new().await.unwrap();
        let dh_provider_con = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true)
            .await
            .unwrap();

        //subscribe to variable changes
        let mut change_stream = dh_provider_con
            .subscribe_variables_with_filter(Option::<Vec<VariableKey>>::None)
            .await
            .unwrap();

        //Try to write invalid value type
        assert!(dh_provider_con
            .write_single_variable("my_folder.rw_string", 123)
            .await
            .is_err());

        //Try to write to RO variable
        assert!(dh_provider_con
            .write_single_variable("my_folder.ro_float", 999.9)
            .await
            .is_err());

        //Try to write non existant variable
        assert!(dh_provider_con
            .write_single_variable("doesntexist", Value::String("wtf?".to_owned()))
            .await
            .is_err());

        //write single variable
        dh_provider_con
            .write_single_variable("my_folder.rw_string", "Hello World!")
            .await
            .unwrap();

        //write should have triggered a change event
        let change_evt: HashMap<_, _> = change_stream.next().await.unwrap().into_iter().collect();
        assert_eq!(change_evt.len(), 1);
        assert_eq!(change_evt.get(&200).unwrap().value, "Hello World!".into());

        //Write multiple variables at once
        let var_changes = [
            ("my_folder.rw_string", "Multi write!!!".into()),
            ("my_folder.rw_int", 123.into()),
        ];
        dh_provider_con.write_variables(&var_changes).await.unwrap();

        //write should have triggered a change event
        let change_evt: HashMap<_, _> = change_stream.next().await.unwrap().into_iter().collect();
        assert_eq!(change_evt.len(), 2);
        assert_eq!(change_evt.get(&200).unwrap().value, "Multi write!!!".into());
        assert_eq!(change_evt.get(&300).unwrap().value, 123.into());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn write_with_insufficient_nats_permissions() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubRead);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let _dummy_provider = DummyProvider::new().await.unwrap();
        let dh_provider_con = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true)
            .await
            .unwrap();

        //try to write variable with RO nats permissions
        assert!(dh_provider_con
            .write_single_variable("my_folder.rw_string", "Hello World!")
            .await
            .is_err());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn change_var_defs() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubRead);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let dummy_provider = DummyProvider::new().await.unwrap();
        let dh_provider_con = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true)
            .await
            .unwrap();

        let mut provider_events_sub = dh_provider_con.subscribe_provider_events().await.unwrap();

        //Read original var defs
        let var_defs = dh_provider_con.get_all_variable_definitions().unwrap();
        assert_eq!(var_defs.len(), 4);

        //create keys and try to reuse them after provider changed
        let ro_float_key = VariableKey::from("my_folder.ro_float");
        let ro_int_key = VariableKey::from("my_folder.ro_int");
        let rw_int_key = VariableKey::from("my_folder.rw_int");

        let ro_float_id = dh_provider_con.variable_id_from_key(ro_float_key).unwrap();
        assert_eq!(ro_float_id, 100);

        let ro_int_id = dh_provider_con.variable_id_from_key(ro_int_key).unwrap();
        assert_eq!(ro_int_id, 400);

        let rw_int_id = dh_provider_con.variable_id_from_key(rw_int_key).unwrap();
        assert_eq!(rw_int_id, 300);

        //Start subscription of single variables
        let mut ro_float_sub = dh_provider_con
            .subscribe_single_variable(ro_float_key)
            .await
            .unwrap();
        let mut ro_int_sub = dh_provider_con
            .subscribe_single_variable(ro_int_key)
            .await
            .unwrap();

        //should receive updates
        ro_float_sub.next().await.unwrap();
        ro_int_sub.next().await.unwrap();

        //trigger a change of the provider def
        dummy_provider.change_variables();

        //wait for changed event from registry (must wait for 2 events - one to clear vars and one for new vars)
        if let ProviderEvent::DefinitionChanged(event_var_defs) =
            provider_events_sub.next().await.unwrap()
        {
            assert!(event_var_defs.is_empty());
        } else {
            panic!("Expected ProviderEvent::DefinitionChanged");
        };

        let var_defs = dh_provider_con.get_all_variable_definitions().unwrap();
        assert!(var_defs.is_empty());

        if let ProviderEvent::DefinitionChanged(event_var_defs) =
            provider_events_sub.next().await.unwrap()
        {
            assert_eq!(event_var_defs.len(), 5);
        } else {
            panic!("Expected ProviderEvent::DefinitionChanged");
        };

        //read new var defs
        let var_defs = dh_provider_con.get_all_variable_definitions().unwrap();
        assert_eq!(var_defs.len(), 5);

        let ro_float_id = dh_provider_con.variable_id_from_key(ro_float_key).unwrap();
        assert_eq!(ro_float_id, 10);

        //rw_int key no longer exists
        assert!(dh_provider_con.variable_id_from_key(rw_int_key).is_err());

        //read entire definiton of new, fifth variable
        let rw_int3 = dh_provider_con.variable_key_from_id(50).unwrap();
        let rw_int3_def = dh_provider_con.get_variable_definition(&rw_int3).unwrap();
        assert_eq!(
            rw_int3_def,
            ConsumerVariableDefinition {
                id: 50,
                key: "my_folder.rw_int3".to_string(),
                read_only: false,
                experimental: false,
                data_type: ConsumerVariableType::Int64,
            }
        );

        //the ro_float should still work, even though variable ID has changed
        ro_float_sub.next().await.unwrap();

        //the ro_int sub should no longer work, as they key no longer exists after the update
        timeout(dummy_provider::VARIABLE_UPDATE_RATE * 2, ro_int_sub.next())
            .await
            .unwrap_err();
    })
    .await;
}

#[tokio::test]
#[serial]
async fn provider_goes_offline() {
    run_with_timeout(async move {
        let _fake_reg = FakeRegistry::new().await;

        let auth_settings = consumer_auth_settings(NatsPermission::VariableHubReadWrite);
        let consumer = Arc::new(
            DataHubConsumer::connect(NATS_HOSTNAME, &auth_settings)
                .await
                .unwrap(),
        );

        let dummy_provider = DummyProvider::new().await.unwrap();
        let dh_provider_con = ConnectedDataHubProvider::new(consumer.clone(), PROVIDER_ID, true)
            .await
            .unwrap();

        let mut provider_events_sub = dh_provider_con.subscribe_provider_events().await.unwrap();

        //Var defs should exist
        let var_defs = dh_provider_con.get_all_variable_definitions().unwrap();
        assert_eq!(var_defs.len(), 4);

        //destroy the provider
        drop(dummy_provider);

        //wait for offline event from registry
        let ProviderEvent::Offline = provider_events_sub.next().await.unwrap() else {
            panic!("Expected ProviderEvent::Offline");
        };

        //var defs should be unchanged
        let var_defs = dh_provider_con.get_all_variable_definitions().unwrap();
        assert_eq!(var_defs.len(), 4);

        //reading and writing vars should fail
        assert!(matches!(
            dh_provider_con
                .read_variables(Option::<&[VariableKey]>::None)
                .await,
            Err(connected_dh_provider::Error::LowLevelApi(
                connected_nats_provider::Error::ProviderOfflineOrInvalid,
            ))
        ),);

        assert!(matches!(
            dh_provider_con
                .write_single_variable("my_folder.rw_string", "Hello World!")
                .await,
            Err(connected_dh_provider::Error::LowLevelApi(
                connected_nats_provider::Error::ProviderOfflineOrInvalid,
            ))
        ),);

        //subscribing to variables should still work
        let mut vars_changed_sub = dh_provider_con
            .subscribe_variables_with_filter(Option::<Vec<VariableKey>>::None)
            .await
            .unwrap();

        //provider comes back online
        let _dummy_provider = DummyProvider::new().await.unwrap();

        //Should get event and update var defs
        if let ProviderEvent::DefinitionChanged(event_var_defs) =
            provider_events_sub.next().await.unwrap()
        {
            assert_eq!(event_var_defs.len(), 4);
        } else {
            panic!("Expected ProviderEvent::DefinitionChanged");
        };

        //read new var defs
        let var_defs = dh_provider_con.get_all_variable_definitions().unwrap();
        assert_eq!(var_defs.len(), 4);

        //reading and writing vars should work again
        assert!(dh_provider_con
            .read_variables(Option::<&[VariableKey]>::None)
            .await
            .is_ok());

        assert!(dh_provider_con
            .write_single_variable("my_folder.rw_string", "Hello World!")
            .await
            .is_ok());

        //the subscription should return values even though it was started while provider was offline
        vars_changed_sub.next().await.unwrap();
    })
    .await;
}
