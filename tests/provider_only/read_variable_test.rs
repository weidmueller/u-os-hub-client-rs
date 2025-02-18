use serial_test::serial;
use u_os_hub_client::{
    generated::weidmueller::ucontrol::hub::root_as_read_variables_query_response,
    payload_builders::build_read_variables_query_request,
    provider::{ProviderOptions, VariableBuilder},
    subjects::read_variables_query_from,
    variable::value::Value,
};

use crate::utils::create_fake_registry;

const NATS_HOSTNAME: &str = "nats:4222";
const PROVIDER_ID: &str = "read_variable_test";

#[tokio::test]
#[serial]
async fn test_read_all_variables() {
    // Prepare
    let test_nats_client = async_nats::ConnectOptions::new();
    let test_nats_client = test_nats_client.connect(NATS_HOSTNAME).await.unwrap();
    let _fake_registry = create_fake_registry(test_nats_client.clone(), PROVIDER_ID.to_string());

    let provider_builder = ProviderOptions::new(PROVIDER_ID);
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .value(Value::Boolean(true))
        .build()
        .expect("variable should build");

    let var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .value(Value::String("Test_String123".to_string()))
        .build()
        .expect("variable should build");

    let _provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register_and_connect(NATS_HOSTNAME)
        .await
        .expect("provider should register");

    // act
    let result = test_nats_client
        .request(
            read_variables_query_from(PROVIDER_ID),
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
                if let Value::Boolean(value) = var1.value {
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
                if let Value::String(value) = var2.value.clone() {
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
    let test_nats_client = async_nats::ConnectOptions::new();
    let test_nats_client = test_nats_client.connect(NATS_HOSTNAME).await.unwrap();
    let _fake_registry = create_fake_registry(test_nats_client.clone(), PROVIDER_ID.to_string());

    let provider_builder = ProviderOptions::new(PROVIDER_ID);
    let var1 = VariableBuilder::new(0, "my_folder.my_variable_1")
        .value(Value::Boolean(true))
        .build()
        .expect("variable should build");

    let var2 = VariableBuilder::new(1, "my_folder.my_variable_2")
        .value(Value::String("Test_String123".to_string()))
        .build()
        .expect("variable should build");

    let _provider = provider_builder
        .add_variables(vec![var1.clone(), var2.clone()])
        .expect("Variables should be added")
        .register_and_connect(NATS_HOSTNAME)
        .await
        .expect("provider should register");

    // act
    let result = test_nats_client
        .request(
            read_variables_query_from(PROVIDER_ID),
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
                if let Value::String(value) = var2.value.clone() {
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
