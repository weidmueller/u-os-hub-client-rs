use crate::generated::weidmueller::ucontrol::hub::{VariableAccessType, VariableDataType};

use super::*;
use rstest::rstest;

// TODO: discuss if it makes sense to move the VariableDefinitions to the test_data module
#[rstest]
#[case(&VariableDefinitionT {
        key: "test".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Ok(()))]
#[case(&VariableDefinitionT {
        key: "_test".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Ok(()))]
#[case(&VariableDefinitionT {
        key: "test_".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Ok(()))]
#[case(&VariableDefinitionT {
        key: "_folder1.test_".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Ok(()))]
#[case(&VariableDefinitionT {
        key: "VARCAPITAL".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Ok(()))]
#[case(&VariableDefinitionT {
        key: "var_with_underscore".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Ok(()))]
#[case(&VariableDefinitionT {
        key: "".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Err(InvalidVariableDefinitionError::UnnamedVariable))]
#[case(&VariableDefinitionT {
        key: "vari/able".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Err(InvalidVariableDefinitionError::InvalidCharacters("vari/able".to_string())))]
#[case(&VariableDefinitionT {
        key: "vari able".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Err(InvalidVariableDefinitionError::InvalidCharacters("vari able".to_string())))]
#[case(&VariableDefinitionT {
        key: "variable_with🐞".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Err(InvalidVariableDefinitionError::InvalidCharacters("variable_with🐞".to_string())))]
#[case(&VariableDefinitionT {
        key: "variable_with.".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Err(InvalidVariableDefinitionError::TrailingDot("variable_with.".to_string())))]
#[case(&VariableDefinitionT {
        key: "my-variable".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Err(InvalidVariableDefinitionError::InvalidCharacters("my-variable".to_string())))]
#[case(&VariableDefinitionT {
        key: "my_variable".to_string(),
        id: 5,
        data_type: VariableDataType::UNSPECIFIED,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Err(InvalidVariableDefinitionError::UnspecifiedProperty(
        "data_type".to_string(),
    )))]
#[case(&VariableDefinitionT {
        key: "my_variable".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::UNSPECIFIED,
        experimental: false
    }, Err(InvalidVariableDefinitionError::UnspecifiedProperty(
        "access_type".to_string(),
    )))]
#[case::valid_name_with_1023_characters(&VariableDefinitionT {
        key: "abcdefghijklmnop.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwx".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Ok(()))]
#[case::invalid_name_with_1024_characters(&VariableDefinitionT {
        key: "abcdefghijklmnopq.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwx".to_string(),
        id: 5,
        data_type: VariableDataType::BOOLEAN,
        access_type: VariableAccessType::READ_ONLY,
        experimental: false
    }, Err(InvalidVariableDefinitionError::InvalidLength("abcdefghijklmnopq.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwxy.abcdefghijklmnopqrstuvwxyz0123456789_abcdefghijklmnopqrstuvwx".to_string())))]
fn test_validate(
    #[case] variable_definition: &VariableDefinitionT,
    #[case] expected_result: Result<(), InvalidVariableDefinitionError>,
) {
    assert_eq!(variable_definition.validate(), expected_result);
}
