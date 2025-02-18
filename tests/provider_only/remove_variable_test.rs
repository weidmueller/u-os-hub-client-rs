use std::time::Duration;

use futures::StreamExt;
use serial_test::serial;
use tokio::time::timeout;
use u_os_hub_client::{
    generated::weidmueller::ucontrol::hub::root_as_read_provider_definition_query_response,
    provider::{ProviderOptions, VariableBuilder},
    subjects::get_provider_name_from_subject,
    variable::value::Value,
};

use crate::utils::create_fake_registry;

const NATS_HOSTNAME: &str = "nats:4222";
const PROVIDER_ID: &str = "delete_variable_test";

#[tokio::test]
#[serial]
async fn test_remove_variables() {
    // Prepare
    let test_nats_client = async_nats::ConnectOptions::new();
    let test_nats_client = test_nats_client.connect(NATS_HOSTNAME).await.unwrap();
    let _fake_registry = create_fake_registry(test_nats_client.clone(), PROVIDER_ID.to_string());

    let mut def_changed_subscribtion = test_nats_client
        .subscribe(format!("v1.loc.{}.def.evt.changed", PROVIDER_ID))
        .await
        .unwrap();

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

    // Register definition can be ignored (we wanna test the definition after removing a variable)
    let _ = timeout(Duration::from_secs(1), def_changed_subscribtion.next())
        .await
        .expect("Provider definition should be published");

    // act
    provider
        .remove_variables(vec![var2])
        .await
        .expect("should add a new variable");

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
        assert_eq!(provider_definition.fingerprint, 7344710243453588040);

        let recv_var_defs = provider_definition
            .variable_definitions
            .expect("there should be variables");

        let recv_var1 = recv_var_defs.first().expect("should should be there");

        // Check if the correct variable was removed
        assert_eq!(recv_var1, &(&var1).into());

        // Only one variable should be left
        assert_eq!(recv_var_defs.len(), 1);
    } else {
        panic!("definition changed message should have been sended")
    }
    drop(provider);
}
