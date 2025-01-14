use std::time::Duration;

use futures::StreamExt;
use serial_test::serial;
use tokio::time::timeout;
use uc_hub_client::{
    generated::weidmueller::ucontrol::hub::{
        root_as_read_provider_definition_query_response, State,
    },
    provider::{ProviderOptions, VariableBuilder},
    variable::value::Value,
    {
        payload_builders::build_state_changed_event_payload,
        subjects::{
            get_provider_name_from_subject, registry_provider_definition_changed_event,
            REGISTRY_STATE_CHANGED_EVENT_SUBJECT,
        },
    },
};

use crate::utils::create_fake_registry;

const NATS_HOSTNAME: &str = "nats:4222";
const PROVIDER_ID: &str = "register-provider-test";

#[tokio::test]
#[serial]
async fn test_register_provider_with_variables() {
    // Prepare
    let test_nats_client = async_nats::ConnectOptions::new();
    let test_nats_client = test_nats_client.connect(NATS_HOSTNAME).await.unwrap();

    let mut registry_def_changed_subscribtion = test_nats_client
        .subscribe(registry_provider_definition_changed_event(
            PROVIDER_ID.to_string(),
        ))
        .await
        .expect("should subscribe to def changed event from registry");

    // act
    let provider_builder = ProviderOptions::new(PROVIDER_ID);
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .value(Value::Boolean(true))
        .build()
        .expect("variable should build");

    let var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .value(Value::Boolean(true))
        .build()
        .expect("variable should build");

    let _fake_registry = create_fake_registry(test_nats_client.clone(), PROVIDER_ID.to_string());

    let _ = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register_and_connect(NATS_HOSTNAME)
        .await
        .expect("provider should register");

    // assert
    if let Ok(Some(msg)) = timeout(
        Duration::from_secs(1),
        registry_def_changed_subscribtion.next(),
    )
    .await
    {
        let provider_definition = root_as_read_provider_definition_query_response(&msg.payload)
            .unwrap()
            .unpack()
            .provider_definition
            .expect("there should be a provider definition");

        assert_eq!(
            get_provider_name_from_subject(&msg.subject).expect("should be there set"),
            PROVIDER_ID
        );
        assert_eq!(provider_definition.fingerprint, 17906070203590430274);

        let recv_var_defs = provider_definition
            .variable_definitions
            .expect("there should be variables");

        let recv_var1 = recv_var_defs.first().expect("should should be there");
        let recv_var2 = recv_var_defs.last().expect("should should be there");

        assert_eq!(recv_var1, &(&var1).into());
        assert_eq!(recv_var2, &(&var2).into());
    } else {
        panic!("definition changed message should have been sended")
    }
}

#[tokio::test]
#[serial]
async fn test_resend_provider_definition_on_registry_up_event() {
    // Prepare
    let test_nats_client = async_nats::ConnectOptions::new();
    let test_nats_client = test_nats_client.connect(NATS_HOSTNAME).await.unwrap();
    let mut def_changed_subscribtion = test_nats_client
        .subscribe(format!("v1.loc.{}.def.evt.changed", PROVIDER_ID))
        .await
        .unwrap();
    let _fake_registry = create_fake_registry(test_nats_client.clone(), PROVIDER_ID.to_string());

    // act
    let provider_builder = ProviderOptions::new(PROVIDER_ID);
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .value(Value::Boolean(true))
        .build()
        .expect("variable should build");

    let var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .value(Value::Boolean(true))
        .build()
        .expect("variable should build");

    let provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register_and_connect(NATS_HOSTNAME)
        .await
        .expect("provider should register");

    // First definition can be ignored (we wanna test what happes on registry up event)
    let _ = timeout(Duration::from_secs(1), def_changed_subscribtion.next())
        .await
        .expect("Provider definition should be published");

    test_nats_client
        .publish(
            REGISTRY_STATE_CHANGED_EVENT_SUBJECT.to_string(),
            build_state_changed_event_payload(State::RUNNING),
        )
        .await
        .expect("should publish registry up event");

    // assert
    if let Ok(Some(msg)) = timeout(Duration::from_secs(1), def_changed_subscribtion.next()).await {
        let provider_definition = root_as_read_provider_definition_query_response(&msg.payload)
            .unwrap()
            .unpack()
            .provider_definition
            .expect("there should be a provider definition");

        assert_eq!(
            get_provider_name_from_subject(&msg.subject).expect("should be there set"),
            PROVIDER_ID
        );
        assert_eq!(provider_definition.fingerprint, 17906070203590430274);

        let recv_var_defs = provider_definition
            .variable_definitions
            .expect("there should be variables");

        let recv_var1 = recv_var_defs.first().expect("should should be there");
        let recv_var2 = recv_var_defs.last().expect("should should be there");

        assert_eq!(recv_var1, &(&var1).into());
        assert_eq!(recv_var2, &(&var2).into());
    } else {
        panic!("definition changed message should have been sended")
    }
    drop(provider);
}
