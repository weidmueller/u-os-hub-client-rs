use std::time::Duration;

use futures::StreamExt;
use serial_test::serial;
use tokio::time::timeout;
use u_os_hub_client::{
    generated::weidmueller::ucontrol::hub::root_as_read_provider_definition_query_response,
    nats_subjects,
    payload_builders::{build_write_variables_command, VariableUpdate},
    provider::{ProviderBuilder, VariableBuilder},
};

use crate::utils::{self, fake_registry::FakeRegistry};

const PROVIDER_ID: &str = "write_variable_test";

#[tokio::test]
#[serial]
async fn test_write_variable_command() {
    // Prepare
    let _fake_registry = FakeRegistry::new().await;
    let auth_nats_con = utils::create_auth_con(PROVIDER_ID).await;
    let test_nats_client = auth_nats_con.get_client().clone();

    let mut def_changed_subscription = test_nats_client
        .subscribe(nats_subjects::provider_changed_event(PROVIDER_ID))
        .await
        .unwrap();

    let provider_builder = ProviderBuilder::new();
    let mut var1 = VariableBuilder::new(0, "my_folder.my_variable_1_rw")
        .read_write()
        .initial_value(true)
        .build()
        .expect("variable should build");

    let provider = provider_builder
        .add_variables(vec![var1.clone()])
        .expect("Variables should be added")
        .register_with_existing_connection(auth_nats_con)
        .await
        .expect("provider should register");

    let mut subscription_to_write_cmd = provider
        .subscribe_to_write_command(vec![var1.clone()])
        .await
        .expect("should work");

    let timeout_result = timeout(Duration::from_secs(1), def_changed_subscription.next()).await;

    let fingerprint = if let Ok(Some(msg)) = timeout_result {
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
    let var1_state = var1.get_mut_state();
    var1_state.set_value(false);
    let write_cmd_payload = build_write_variables_command(
        vec![VariableUpdate {
            id: var1_state.get_id(),
            value: var1_state.get_value().into(),
        }],
        fingerprint,
    );

    test_nats_client
        .publish(
            nats_subjects::write_variables_command(PROVIDER_ID),
            write_cmd_payload,
        )
        .await
        .expect("should publish write command");

    // assert
    if let Ok(Some(write_commands)) =
        timeout(Duration::from_secs(1), subscription_to_write_cmd.recv()).await
    {
        let first_write_command = write_commands
            .first()
            .expect("One variable write command should received")
            .clone();

        assert_eq!(first_write_command.id, var1.get_definition().id);
        assert_eq!(&first_write_command.value, var1.get_state().get_value());
    } else {
        panic!("should received write command")
    }
}
