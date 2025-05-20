use rstest::rstest;

use crate::{dh_types::VariableAccessType, provider::VariableBuildError};

use super::VariableBuilder;

#[test]
fn test_missing_value_error() {
    // Prepare
    let my_var = VariableBuilder::new(0, "my_var");

    // Act
    let result = my_var.build();

    // Assert
    assert_eq!(result, Err(VariableBuildError::MissingValue));
}

#[test]
fn test_invalid_access_type_error() {
    // Prepare
    let my_var = VariableBuilder::new(0, "my_var").access_type(VariableAccessType::Unknown(0));

    // Act
    let result = my_var.build();

    // Assert
    assert_eq!(result, Err(VariableBuildError::InvalidAccessType));
}

#[rstest]
#[case("Test", true)]
#[case("teSt", true)]
#[case("tesT", true)]
#[case("test", true)]
#[case("_test", true)]
#[case("_tEst", true)]
#[case("_test_", true)]
#[case("my-folder.my-var-1", false)]
#[case("my-folder.my_var_1", false)]
#[case("my_folder.my_var_1", true)]
#[case("my_folder.my_var_1_", true)]
#[case("_my_Folder.my_var_1", true)]
#[case("_my_Folder.my_var_1_", true)]
#[case("MY_folder.my_var_1", true)]
#[case("MY_folder.my_var_1__", true)]
#[case("MY_folder.my_var!1", false)]
#[case(
    "this_is_a_very_long_variable_key_this_is_not_allowed_but_why_this_looks_beautiful_or_not",
    false
)]
fn test_key_validation(#[case] key: String, #[case] valid: bool) {
    // Prepare

    use crate::dh_types::{VariableAccessType, VariableValue};
    let my_var = VariableBuilder::new(0, &key).initial_value(true);

    // Act
    let result = my_var.build();

    // Assert
    match result {
        Ok(var) => {
            assert!(valid, "the variable should be valid");
            assert_eq!(var.definition.id, 0);
            assert_eq!(var.definition.key, key);
            assert_eq!(var.definition.access_type, VariableAccessType::ReadOnly);
            assert_eq!(var.state.value, VariableValue::Boolean(true));
        }
        Err(e) => {
            assert!(!valid, "the variable should not be valid");
            assert_eq!(e, VariableBuildError::InvalidVariableName(key.clone()));
        }
    }
}
