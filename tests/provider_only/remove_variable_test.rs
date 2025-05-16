use std::time::Duration;

use futures::StreamExt;
use serial_test::serial;
use tokio::time::timeout;
use u_os_hub_client::{
    generated::weidmueller::ucontrol::hub::root_as_read_provider_definition_query_response,
    nats_subjects::{self, get_provider_name_from_subject},
    provider::{ProviderBuilder, VariableBuilder},
};

use crate::utils::{self, fake_registry::FakeRegistry};

const PROVIDER_ID: &str = "delete_variable_test";

#[tokio::test]
#[serial]
async fn test_remove_variables() {
    // Prepare
    let _fake_registry = FakeRegistry::new().await;
    let auth_nats_con = utils::create_auth_con(PROVIDER_ID).await;
    let test_nats_client = auth_nats_con.get_client();

    let mut def_changed_subscription = test_nats_client
        .subscribe(nats_subjects::provider_changed_event(PROVIDER_ID))
        .await
        .unwrap();

    let provider_builder = ProviderBuilder::new();
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .initial_value(true)
        .build()
        .expect("variable should build");

    let var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .initial_value(true)
        .build()
        .expect("variable should build");

    let provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register_with_existing_connection(auth_nats_con)
        .await
        .expect("provider should register");

    // Register definition can be ignored (we wanna test the definition after removing a variable)
    let _ = timeout(Duration::from_secs(1), def_changed_subscription.next())
        .await
        .expect("Provider definition should be published");

    // act
    provider
        .remove_variables(vec![var2])
        .await
        .expect("should add a new variable");

    // assert
    if let Ok(Some(msg)) = timeout(Duration::from_secs(1), def_changed_subscription.next()).await {
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
