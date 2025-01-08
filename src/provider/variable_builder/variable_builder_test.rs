use rstest::rstest;

use crate::{provider::VariableBuildError, variable::value::Value};

use super::VariableBuilder;

#[test]
fn test_missing_value_error() {
    // Prepare
    let my_var = VariableBuilder::new(0, "my-var");

    // Act
    let result = my_var.build();

    // Assert
    assert_eq!(result, Err(VariableBuildError::MissingValue));
}

#[rstest]
#[case("Test", false)]
#[case("test", true)]
#[case("my-folder.my-var-1", true)]
#[case("my-folder.my_var_1", false)]
#[case("MY-folder.my-var-1", false)]
#[case(
    "this-is-a-very-long-variable-key-this-is-not-allowed-but-why-this-looks-beautiful-or-not",
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
