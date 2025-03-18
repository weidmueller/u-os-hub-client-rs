use serial_test::serial;
use u_os_hub_client::{
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
