// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

use futures::StreamExt;
use serial_test::serial;
use tokio::time::timeout;
use u_os_hub_client::{
    generated::weidmueller::ucontrol::hub::root_as_read_provider_definition_query_response,
    nats_subjects::{self, get_provider_name_from_subject},
    provider::{
        provider_definition_validator::InvalidProviderDefinitionError, AddVariablesError,
        ProviderBuilder, VariableBuilder,
    },
};

use crate::utils::{self, fake_registry::FakeRegistry};

const PROVIDER_ID: &str = "add_variable_test";

#[tokio::test]
#[serial]
async fn test_add_variables() {
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

    // Register definition can be ignored (we wanna test the definition after adding a variable)
    let _ = timeout(Duration::from_secs(1), def_changed_subscription.next())
        .await
        .expect("Provider definition should be published");

    // act
    let var3 = VariableBuilder::new(2, "my_folder.my_variable_3")
        .initial_value(true)
        .build()
        .expect("variable should build");

    provider
        .add_variables(vec![var3.clone()])
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
        assert_eq!(provider_definition.fingerprint, 15085505095296877768);

        let recv_var_defs = provider_definition
            .variable_definitions
            .expect("there should be variables");

        let recv_var1 = recv_var_defs.first().expect("should should be there");
        let recv_var2 = recv_var_defs.get(1).expect("should should be there");
        let recv_var3 = recv_var_defs.last().expect("should should be there");

        assert_eq!(recv_var1, &(&var1).into());
        assert_eq!(recv_var2, &(&var2).into());
        assert_eq!(recv_var3, &(&var3).into());
    } else {
        panic!("definition changed message should have been sended")
    }
    drop(provider);
}

#[tokio::test]
#[serial]
async fn test_add_variables_fail_on_duplicates() {
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

    // Register definition can be ignored (we wanna the definition after adding a variable)
    let _ = timeout(Duration::from_secs(1), def_changed_subscription.next())
        .await
        .expect("Provider definition should be published");

    // act
    // Variable with duplicated id
    let var_duplicated_id = VariableBuilder::new(1, "my_folder.my_variable_3")
        .initial_value(true)
        .build()
        .expect("variable should build");
    let result_duplicated_id = provider
        .add_variables(vec![var_duplicated_id.clone()])
        .await;

    // Variable with duplicated key
    let var_duplicated_key = VariableBuilder::new(2, "my_folder.my_variable_2")
        .initial_value(true)
        .build()
        .expect("variable should build");
    let result_duplicated_key = provider
        .add_variables(vec![var_duplicated_key.clone()])
        .await;

    // assert
    if let Err(AddVariablesError::InvalidMergedVariableList(
        InvalidProviderDefinitionError::DuplicateId(id),
    )) = result_duplicated_id
    {
        assert_eq!(id, 1);
    } else {
        panic!("Adding variables with duplicated ids should fail")
    }

    if let Err(AddVariablesError::InvalidMergedVariableList(
        InvalidProviderDefinitionError::DuplicatePath(key),
    )) = result_duplicated_key
    {
        assert_eq!(key, "my_folder.my_variable_2");
    } else {
        panic!("Adding variables with duplicated keys should fail")
    }

    drop(provider);
}
