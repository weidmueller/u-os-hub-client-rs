use serial_test::serial;
use u_os_hub_client::{
    generated::weidmueller::ucontrol::hub::root_as_read_variables_query_response,
    nats_subjects,
    payload_builders::build_read_variables_query_request,
    provider::{ProviderBuilder, VariableBuilder},
    variable::value::VariableValue,
};

use crate::utils::{self, fake_registry::FakeRegistry};

const PROVIDER_ID: &str = "read_variable_test";

#[tokio::test]
#[serial]
async fn test_read_all_variables() {
    // Prepare
    let _fake_registry = FakeRegistry::new().await;
    let auth_nats_con = utils::create_auth_con(PROVIDER_ID).await;
    let test_nats_client = auth_nats_con.get_client().clone();

    let provider_builder = ProviderBuilder::new();
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .value(VariableValue::Boolean(true))
        .build()
        .expect("variable should build");

    let var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .value(VariableValue::String("Test_String123".to_string()))
        .build()
        .expect("variable should build");

    let _provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register_with_existing_connection(auth_nats_con)
        .await
        .expect("provider should register");

    // act
    let result = test_nats_client
        .request(
            nats_subjects::read_variables_query(PROVIDER_ID),
            build_read_variables_query_request(None),
        )
        .await
        .expect("a response should be received");

    // assert
    let unpacked_response = root_as_read_variables_query_response(&result.payload)
        .expect("the response should be parseable")
        .unpack();

    let variables = unpacked_response
        .variables
        .items
        .expect("there should be variables");

    assert_eq!(variables.len(), 2);

    for variable in &variables {
        match variable.id {
            0 => {
                if let VariableValue::Boolean(value) = var1.value {
                    assert_eq!(
                        variable
                            .value
                            .as_boolean()
                            .expect("this should be a boolean")
                            .value,
                        value
                    );
                } else {
                    panic!("this test needs to be adapted to the new the values");
                }
            }
            1 => {
                if let VariableValue::String(value) = var2.value.clone() {
                    assert_eq!(
                        variable
                            .value
                            .as_string()
                            .expect("this should be a string")
                            .value
                            .clone()
                            .expect("this value should be empty"),
                        value
                    );
                } else {
                    panic!("this test needs to be adapted to the new the values");
                }
            }
            _ => {
                panic!("This id was never set!")
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_read_one_variable() {
    // Prepare
    let _fake_registry = FakeRegistry::new().await;
    let auth_nats_con = utils::create_auth_con(PROVIDER_ID).await;
    let test_nats_client = auth_nats_con.get_client().clone();

    let provider_builder = ProviderBuilder::new();
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .value(VariableValue::Boolean(true))
        .build()
        .expect("variable should build");

    let var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .value(VariableValue::String("Test_String123".to_string()))
        .build()
        .expect("variable should build");

    let _provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register_with_existing_connection(auth_nats_con)
        .await
        .expect("provider should register");

    // act
    let result = test_nats_client
        .request(
            nats_subjects::read_variables_query(PROVIDER_ID),
            build_read_variables_query_request(Some(vec![1])),
        )
        .await
        .expect("a response should be received");

    // assert
    let unpacked_response = root_as_read_variables_query_response(&result.payload)
        .expect("the response should be parseable")
        .unpack();

    let variables = unpacked_response
        .variables
        .items
        .expect("there should be variables");

    assert_eq!(variables.len(), 1);

    for variable in &variables {
        match variable.id {
            1 => {
                if let VariableValue::String(value) = var2.value.clone() {
                    assert_eq!(
                        variable
                            .value
                            .as_string()
                            .expect("this should be a string")
                            .value
                            .clone()
                            .expect("this value should be empty"),
                        value
                    );
                } else {
                    panic!("this test needs to be adapted to the new the values");
                }
            }
            _ => {
                panic!("this id was never set!")
            }
        }
    }
}
