use crate::provider::{provider_builder::AddVariablesError, VariableBuilder};

use super::ProviderBuilder;

#[test]
fn test_add_variables() {
    // Prepare
    let provider = ProviderBuilder::new();

    let var1 = VariableBuilder::new(0, "test_var_1")
        .initial_value(true)
        .build()
        .expect("the variable should build");

    let var2 = VariableBuilder::new(1, "test_var_2")
        .initial_value(true)
        .build()
        .expect("the variable should build");

    // Act
    let result = provider.add_variables(vec![var1, var2]);

    // Assert
    result.expect("This should work!");
}

#[test]
fn test_duplicated_variable_ids_1() {
    // Prepare
    let var_id = 0;
    let provider = ProviderBuilder::new();

    let var1 = VariableBuilder::new(var_id, "test_var_1")
        .initial_value(true)
        .build()
        .expect("the variable should build");

    let var2 = VariableBuilder::new(var_id, "test_var_2")
        .initial_value(true)
        .build()
        .expect("the variable should build");

    // Act
    let error = provider.add_variables(vec![var1, var2]).unwrap_err();

    // Assert
    assert_eq!(error, AddVariablesError::DuplicatedId(var_id));
}

#[test]
fn test_duplicated_variable_ids_2() {
    // Prepare
    let var_id = 0;
    let provider = ProviderBuilder::new();

    let var1 = VariableBuilder::new(var_id, "test_var_1")
        .initial_value(true)
        .build()
        .expect("the variable should build");

    let var2 = VariableBuilder::new(var_id, "test_var_2")
        .initial_value(true)
        .build()
        .expect("the variable should build");

    // Act
    let error = provider
        .add_variables(vec![var1])
        .expect("this should work")
        .add_variables(vec![var2])
        .unwrap_err();

    // Assert
    assert_eq!(error, AddVariablesError::DuplicatedId(var_id));
}

#[test]
fn test_duplicated_variable_names_1() {
    // Prepare
    let var_name = "test_var_1";
    let provider = ProviderBuilder::new();

    let var1 = VariableBuilder::new(0, var_name)
        .initial_value(true)
        .build()
        .expect("the variable should build");

    let var2 = VariableBuilder::new(1, var_name)
        .initial_value(true)
        .build()
        .expect("the variable should build");

    // Act
    let error = provider.add_variables(vec![var1, var2]).unwrap_err();

    // Assert
    assert_eq!(
        error,
        AddVariablesError::DuplicatedKey(var_name.to_string())
    );
}

#[test]
fn test_duplicated_variable_names_2() {
    // Prepare
    let var_name = "test_var_1";
    let provider = ProviderBuilder::new();

    let var1 = VariableBuilder::new(0, var_name)
        .initial_value(true)
        .build()
        .expect("the variable should build");

    let var2 = VariableBuilder::new(1, var_name)
        .initial_value(true)
        .build()
        .expect("the variable should build");

    // Act
    let error = provider
        .add_variables(vec![var1])
        .expect("this should work")
        .add_variables(vec![var2])
        .unwrap_err();

    // Assert
    assert_eq!(
        error,
        AddVariablesError::DuplicatedKey(var_name.to_string())
    );
}
