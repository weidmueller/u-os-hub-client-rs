use std::collections::HashSet;

use crate::generated::weidmueller::ucontrol::hub::{ProviderDefinitionState, ProviderDefinitionT};
use thiserror::Error;

use super::variable_definition_validator::InvalidVariableDefinitionError;

#[cfg(test)]
mod provider_definition_validator_test;

pub mod test_data;

/// Errors when validating provider definitions.
#[derive(Error, Debug, PartialEq)]
pub enum InvalidProviderDefinitionError {
    /// Provider definition contains duplicate paths.
    #[error("The path `{0}` exists several times")]
    DuplicatePath(String),
    /// Provider definition contains a variable which should be added to another variable.
    /// Variables shall only be added to folders.
    #[error("The path `{0}` is added to a leaf node and not a folder")]
    AddToLeafNode(String),
    /// The definition of a variable is invalid.
    #[error("Invalid variable definition: `{0}`")]
    InvalidVariableDefinition(InvalidVariableDefinitionError),
    /// Found a duplicate id in the provider definition.
    #[error("The id `{0}` is duplicated")]
    DuplicateId(u32),
}

/// Block extends the generated flatbuffers structure 'ProviderDefinitionT' with a validate function.
impl ProviderDefinitionT {
    /// The function validates a provider definition.
    ///
    /// It checks if
    /// - variable definitions are valid
    /// - all variable ids are unique (in the provider definition)
    /// - all variable names are unique (in the provider definition)
    /// - all variables are added to a folder and not to another variable
    pub fn validate(&self) -> Result<(), InvalidProviderDefinitionError> {
        if let Some(variable_definitions) = &self.variable_definitions {
            let mut preverified_paths = HashSet::new();
            let mut preverified_ids = HashSet::new();

            // Validate all variables in one iteration
            for variable in variable_definitions {
                // Check if the variable definition is valid
                variable.validate().map_err(|err| {
                    InvalidProviderDefinitionError::InvalidVariableDefinition(err)
                })?;

                if Self::is_duplicate_id(&mut preverified_ids, &variable.id) {
                    return Err(InvalidProviderDefinitionError::DuplicateId(variable.id));
                }

                if Self::is_duplicate_variable_id(&mut preverified_paths, variable.key.clone()) {
                    return Err(InvalidProviderDefinitionError::DuplicatePath(
                        variable.key.clone(),
                    ));
                }
            }

            // Run after all other checks to find naming collisions regardless of node-order
            for variable in variable_definitions {
                if Self::is_variable_added_to_leaf_node(&preverified_paths, variable.key.clone()) {
                    return Err(InvalidProviderDefinitionError::AddToLeafNode(
                        variable.key.clone(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn is_duplicate_id(ids: &mut HashSet<u32>, id: &u32) -> bool {
        !ids.insert(*id)
    }

    fn is_duplicate_variable_id(paths: &mut HashSet<String>, variable_key: String) -> bool {
        !paths.insert(variable_key)
    }

    fn is_variable_added_to_leaf_node(paths: &HashSet<String>, variable_key: String) -> bool {
        let parent_paths = Self::get_all_parent_paths(&variable_key);
        parent_paths
            .iter()
            .any(|parent_path| paths.contains(parent_path))
    }

    /// Turns a path into a vector of all parent paths.
    ///
    /// # Examples
    /// ```text
    /// assert_eq!(get_all_parent_paths("var1/var2/var3"), vec!["var1", "var1/var2"]);
    /// assert_eq!(get_all_parent_paths("var1"), vec![]);
    /// assert_eq!(get_all_parent_paths(""), vec![]);
    /// ```
    fn get_all_parent_paths(variable_key: &str) -> Vec<String> {
        let path_parts: Vec<&str> = variable_key.split('.').collect();
        (0..path_parts.len() - 1)
            .map(|last_index| path_parts[0..=last_index].to_vec().join("."))
            .collect()
    }

    /// Helper function which tries to construct a valid provider definition from the provider definition.
    pub fn to_valid_provider_definition(
        self,
    ) -> Result<ValidProviderDefinition, InvalidProviderDefinitionError> {
        ValidProviderDefinition::new(self)
    }
}

/// Struct implements a valid provider definition (as a tuple struct).
///
/// A provider definition is given to the new function and validated here.
/// If the provider definition is valid, a valid provider definition is constructed.
/// If not, construction fails.
#[derive(Clone)]
pub struct ValidProviderDefinition(pub ProviderDefinitionT);

impl ValidProviderDefinition {
    pub fn new(
        mut provider_definition: ProviderDefinitionT,
    ) -> Result<ValidProviderDefinition, InvalidProviderDefinitionError> {
        provider_definition.validate()?;
        provider_definition.state = ProviderDefinitionState::OK;
        Ok(ValidProviderDefinition(provider_definition))
    }
}
