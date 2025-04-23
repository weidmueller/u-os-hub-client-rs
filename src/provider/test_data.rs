//! Contains test data for unit and integration tests.
//! Only needed for development and testing.

// For integration test code, these lints are explicitly allowed.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use crate::generated::weidmueller::ucontrol::hub::{
    ProviderDefinitionState, ProviderDefinitionT, VariableAccessType, VariableDataType,
    VariableDefinitionT,
};

pub fn valid_provider_definition_with_variables() -> ProviderDefinitionT {
    ProviderDefinitionT {
        fingerprint: 1,
        variable_definitions: Some(vec![
            VariableDefinitionT {
                key: "var_boolean".to_string(),
                id: 1,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_string".to_string(),
                id: 2,
                data_type: VariableDataType::STRING,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_int64".to_string(),
                id: 3,
                data_type: VariableDataType::INT64,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "folder1.var_int64".to_string(),
                id: 4,
                data_type: VariableDataType::INT64,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "folder1.var_int64_2".to_string(),
                id: 5,
                data_type: VariableDataType::INT64,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_timestamp".to_string(),
                id: 6,
                data_type: VariableDataType::TIMESTAMP,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_duration".to_string(),
                id: 7,
                data_type: VariableDataType::DURATION,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
        ]),
        state: ProviderDefinitionState::OK,
        ..ProviderDefinitionT::default()
    }
}

pub fn valid_provider_definition_with_read_write_variables() -> ProviderDefinitionT {
    ProviderDefinitionT {
        fingerprint: 1,
        variable_definitions: Some(vec![
            VariableDefinitionT {
                key: "var_boolean".to_string(),
                id: 1,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_string".to_string(),
                id: 2,
                data_type: VariableDataType::STRING,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_int64".to_string(),
                id: 3,
                data_type: VariableDataType::INT64,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "folder1.var_int64".to_string(),
                id: 4,
                data_type: VariableDataType::INT64,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "folder1.var_int64_2".to_string(),
                id: 5,
                data_type: VariableDataType::INT64,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_timestamp".to_string(),
                id: 6,
                data_type: VariableDataType::TIMESTAMP,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_duration".to_string(),
                id: 7,
                data_type: VariableDataType::DURATION,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_float64".to_string(),
                id: 8,
                data_type: VariableDataType::FLOAT64,
                access_type: VariableAccessType::READ_WRITE,
                ..VariableDefinitionT::default()
            },
        ]),
        state: ProviderDefinitionState::OK,
        ..ProviderDefinitionT::default()
    }
}

pub fn invalid_provider_definition_with_unnamed_variable() -> ProviderDefinitionT {
    ProviderDefinitionT {
        fingerprint: 2,
        variable_definitions: Some(vec![
            VariableDefinitionT {
                key: "var_string".to_string(),
                id: 1,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "".to_string(),
                id: 2,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
        ]),
        ..ProviderDefinitionT::default()
    }
}

pub fn invalid_provider_definition_with_duplicate_id() -> ProviderDefinitionT {
    ProviderDefinitionT {
        fingerprint: 3,
        variable_definitions: Some(vec![
            VariableDefinitionT {
                key: "var_string".to_string(),
                id: 1,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_int64".to_string(),
                id: 1,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
        ]),
        ..ProviderDefinitionT::default()
    }
}

pub fn invalid_provider_definition_with_invalid_characters() -> ProviderDefinitionT {
    ProviderDefinitionT {
        fingerprint: 4,
        variable_definitions: Some(vec![
            VariableDefinitionT {
                key: "var_string".to_string(),
                id: 1,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "My🐞variable2".to_string(),
                id: 2,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
        ]),
        ..ProviderDefinitionT::default()
    }
}

pub fn invalid_provider_definition_with_subnode_of_node() -> ProviderDefinitionT {
    ProviderDefinitionT {
        fingerprint: 5,
        variable_definitions: Some(vec![
            VariableDefinitionT {
                key: "var_string".to_string(),
                id: 1,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_string.my_variable2".to_string(),
                id: 2,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
        ]),
        ..ProviderDefinitionT::default()
    }
}

pub fn invalid_provider_definition_with_subsubnode_of_node() -> ProviderDefinitionT {
    ProviderDefinitionT {
        fingerprint: 5,
        variable_definitions: Some(vec![
            VariableDefinitionT {
                key: "var_string".to_string(),
                id: 1,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
            VariableDefinitionT {
                key: "var_string.var_int64.var_int64".to_string(),
                id: 2,
                data_type: VariableDataType::BOOLEAN,
                access_type: VariableAccessType::READ_ONLY,
                ..VariableDefinitionT::default()
            },
        ]),
        ..ProviderDefinitionT::default()
    }
}

pub trait ProviderDefinitionFilter {
    fn filter_variables_by_datatype(self, datatype: VariableDataType) -> Result<Self, String>
    where
        Self: Sized;
}

impl ProviderDefinitionFilter for ProviderDefinitionT {
    fn filter_variables_by_datatype(mut self, datatype: VariableDataType) -> Result<Self, String> {
        if let Some(variable_definitions) = self.variable_definitions {
            let filtered_variable_definitions = variable_definitions
                .into_iter()
                .filter(|definition| definition.data_type == datatype)
                .collect();

            self.variable_definitions = Some(filtered_variable_definitions);
            Ok(self)
        } else {
            Err(format!(
                "Unable to filter variable definitions by datatype {:?}",
                datatype
            ))
        }
    }
}

pub trait ReverseableVarDefinitions {
    fn reverse_nodes(self) -> Self;
}

impl ReverseableVarDefinitions for ProviderDefinitionT {
    fn reverse_nodes(mut self) -> Self {
        let reversed_vars: Vec<VariableDefinitionT> = self
            .variable_definitions
            .unwrap()
            .into_iter()
            .rev()
            .collect();
        self.variable_definitions = Some(reversed_vars);
        self
    }
}
