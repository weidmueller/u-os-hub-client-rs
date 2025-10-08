// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

use crate::{
    generated::weidmueller::ucontrol::hub::{
        VariableAccessType, VariableDataType, VariableDefinitionT,
    },
    provider::test_data,
};

use super::*;
use rstest::rstest;
use test_data::ReverseableVarDefinitions;

#[rstest]
#[case::valid_provider_definition_without_nodes(ProviderDefinitionT::default(), Ok(()))]
#[case::valid_provider_definition_with_variables(test_data::valid_provider_definition_with_variables(), Ok(()))]
#[case::invalid_provider_definition_with_unnamed_variable(
    test_data::invalid_provider_definition_with_unnamed_variable(),
    Err(InvalidProviderDefinitionError::InvalidVariableDefinition(
        InvalidVariableDefinitionError::UnnamedVariable
    ))
)]
#[case::invalid_provider_definition_with_duplicate_id(
    test_data::invalid_provider_definition_with_duplicate_id(),
    Err(InvalidProviderDefinitionError::DuplicateId(1))
)]
#[case::invalid_provider_definition_with_invalid_characters(test_data::invalid_provider_definition_with_invalid_characters(), Err(InvalidProviderDefinitionError::InvalidVariableDefinition(InvalidVariableDefinitionError::InvalidCharacters("My🐞variable2".to_string()))))]
#[case::invalid_provider_definition_with_subnode_of_node(test_data::invalid_provider_definition_with_subnode_of_node().reverse_nodes(), Err(InvalidProviderDefinitionError::AddToLeafNode("var_string.my_variable2".to_string())))]
#[case::invalid_provider_definition_with_subnode_of_node_rev(test_data::invalid_provider_definition_with_subnode_of_node().reverse_nodes(), Err(InvalidProviderDefinitionError::AddToLeafNode("var_string.my_variable2".to_string())))]
#[case::invalid_provider_definition_with_subsubnode_of_node(test_data::invalid_provider_definition_with_subsubnode_of_node(), Err(InvalidProviderDefinitionError::AddToLeafNode("var_string.var_int64.var_int64".to_string())))]
#[case::invalid_provider_definition_with_subsubnode_of_node_rev(test_data::invalid_provider_definition_with_subsubnode_of_node().reverse_nodes(), Err(InvalidProviderDefinitionError::AddToLeafNode("var_string.var_int64.var_int64".to_string())))]
fn test_validate(
    #[case] provider_definition: ProviderDefinitionT,
    #[case] expected_result: Result<(), InvalidProviderDefinitionError>,
) {
    assert_eq!(provider_definition.validate(), expected_result);
}

#[rstest]
fn test_performance() {
    //Create a provider definition with 50k variables
    let num_vars = 50_000u32;
    let mut provider_definition = ProviderDefinitionT::default();

    let mut variable_definitions = Vec::with_capacity(num_vars as usize);
    for i in 0..num_vars {
        let variable_definition = VariableDefinitionT {
            id: i,
            key: format!("var_string.my_variable{i}"),
            data_type: VariableDataType::STRING,
            access_type: VariableAccessType::READ_WRITE,
            experimental: false,
        };
        variable_definitions.push(variable_definition);
    }

    provider_definition.variable_definitions = Some(variable_definitions);

    // Measure the time it takes to validate the provider definition
    let start_time = std::time::Instant::now();
    provider_definition.validate().unwrap();
    let duration = start_time.elapsed();
    println!("Validation took: {:?}", duration);

    //Not strictly deterministic (hardware dependent), but this should be reasonable for debug builds
    //Currently only takes about 100ms on my local hardware.
    assert!(duration.as_secs() < 2, "Validation took too long");
}
