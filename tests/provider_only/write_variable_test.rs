use std::time::Duration;

use futures::StreamExt;
use serial_test::serial;
use tokio::time::timeout;
use uc_hub_client::{
    generated::weidmueller::ucontrol::hub::root_as_read_provider_definition_query_response,
    payload_builders::build_write_variables_command,
    provider::{ProviderOptions, VariableBuilder},
    subjects::write_variables_command_from,
    variable::value::Value,
};

use crate::utils::create_fake_registry;

const NATS_HOSTNAME: &str = "nats:4222";
const PROVIDER_ID: &str = "write_variable_test";

#[tokio::test]
#[serial]
async fn test_write_variable_command() {
    // Prepare
    let test_nats_client = async_nats::ConnectOptions::new();
    let test_nats_client = test_nats_client.connect(NATS_HOSTNAME).await.unwrap();
    let _fake_registry = create_fake_registry(test_nats_client.clone(), PROVIDER_ID.to_string());
    let mut def_changed_subscribtion = test_nats_client
        .subscribe(format!("v1.loc.{}.def.evt.changed", PROVIDER_ID))
        .await
        .unwrap();

    let provider_builder = ProviderOptions::new(PROVIDER_ID);
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1_rw")
        .read_write()
        .value(Value::Boolean(true))
        .build()
        .expect("variable should build");

    let provider = provider_builder
        .add_variables(vec![var1.clone()])
        .expect("Variables should be added")
        .register_and_connect(NATS_HOSTNAME)
        .await
        .expect("provider should register");

    let mut subscribtion_to_write_cmd = provider
        .subscribe_to_write_command(&[var1.clone()])
        .await
        .expect("should work");

    let fingerprint = if let Ok(Some(msg)) =
        timeout(Duration::from_secs(1), def_changed_subscribtion.next()).await
    {
        let provider_definition = root_as_read_provider_definition_query_response(&msg.payload)
            .unwrap()
            .unpack()
            .provider_definition
            .expect("there should be a provider definition");

        provider_definition.fingerprint
    } else {
        panic!("should receive a provider definition from register")
    };
    // act
    let mut var1 = var1.clone();
    var1.value = Value::Boolean(false);
    let write_cmd_payload = build_write_variables_command(vec![var1.clone()], fingerprint);

    test_nats_client
        .publish(write_variables_command_from(PROVIDER_ID), write_cmd_payload)
        .await
        .expect("should publish write command");

    // assert
    if let Ok(Some(vars)) = timeout(Duration::from_secs(1), subscribtion_to_write_cmd.recv()).await
    {
        let variable = vars
            .first()
            .expect("One variable write command should received")
            .clone();

        assert_eq!(variable.id, var1.id);
        assert_eq!(variable.experimental, var1.experimental);
        assert_eq!(variable.key, var1.key);
        assert_eq!(variable.read_only, var1.read_only);
        assert_eq!(variable.value, var1.value);
    } else {
        panic!("should received write command")
    }
}
