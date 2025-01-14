use rstest::rstest;

use crate::{provider::VariableBuildError, variable::value::Value};

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
    let my_var = VariableBuilder::new(0, &key).value(Value::Boolean(true));

    // Act
    let result = my_var.build();

    // Assert
    match result {
        Ok(var) => {
            if !valid {
                panic!("the variable should be invalid")
            } else {
                assert_eq!(var.id, 0);
                assert_eq!(var.key, key);
                assert!(var.read_only);
                assert_eq!(var.value, Value::Boolean(true));
            }
        }
        Err(e) => {
            if valid {
                panic!("the variable should be valid")
            }
            assert_eq!(e, VariableBuildError::InvalidVariableName(key.clone()));
        }
    }
}
