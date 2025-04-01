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
