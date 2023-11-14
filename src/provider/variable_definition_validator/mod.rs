//! This module contains the validation logic for variable definitions.

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

use crate::generated::weidmueller::ucontrol::hub::{
    VariableAccessType, VariableDataType, VariableDefinitionT,
};

#[cfg(test)]
mod variable_definition_validator_test;

/// Errors when validating variable definitions.
#[derive(Error, Debug, PartialEq)]
pub enum InvalidVariableDefinitionError {
    /// Unnamed variable found.
    #[error("Unnamed variable")]
    UnnamedVariable,
    /// The variable key contains invalid characters.
    #[error("The variable key of `{0}` contains invalid characters")]
    InvalidCharacters(String),
    /// Trailing dot '.' found in variable key.
    #[error("The variable key of `{0}` contains a trailing '.'")]
    TrailingDot(String),
    /// Unspecified property.
    #[error("Unspecified property `{0}`")]
    UnspecifiedProperty(String),
    /// Invalid length
    #[error("The the variable key length of `{0}` is invalid")]
    InvalidLength(String),
}

/// The regex pattern used to validate variable keys.
/// All variable keys must match this pattern to be valid.
///
/// The pattern will be checked by the variable builder as well as the provider and the uOS Data Hub registry.
pub static VALID_VARIABLE_KEY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    // Only a-z,A-Z,0-9,_,. are allowed
    // The original pattern checking the length
    // ^(?=.{1,1023}$)[a-z]([a-z0-9-]{0,61}[a-z0-9])?(\/[a-z]([a-z0-9-]{0,61}[a-z0-9])?)*$
    // is not working because look-ahead and look-behind are not supported from Regex-crade.
    // So we check the length seperately outside this function.

    //Safety: This should actually be a hard error because an invalid regex would be a developer error
    #[allow(clippy::expect_used)]
    Regex::new(r"^[a-zA-Z_]([a-zA-Z0-9_]{0,62})?(\.[a-zA-Z_]([a-zA-Z0-9_]{0,62})?)*$")
        .expect("this regex should be valid")
});

/// Checks if a variable key is valid.
///
/// Variable keys must match the regex pattern [`VALID_VARIABLE_KEY_PATTERN`] and some additional constrains.
/// The keys will be checked by the variable builder as well as the provider and the uOS Data Hub registry.
pub fn validate_variable_key(key: &str) -> Result<(), InvalidVariableDefinitionError> {
    //Try to do quick O(1) checks first here, as this can speed up the validation time

    if key.is_empty() {
        return Err(InvalidVariableDefinitionError::UnnamedVariable);
    }

    if key.ends_with('.') {
        return Err(InvalidVariableDefinitionError::TrailingDot(key.to_string()));
    }

    //Note: this is O(n), but should be acceptable as the key is usually short
    if key.chars().count() > 1023 {
        return Err(InvalidVariableDefinitionError::InvalidLength(
            key.to_string(),
        ));
    }

    if !VALID_VARIABLE_KEY_PATTERN.is_match(key) {
        return Err(InvalidVariableDefinitionError::InvalidCharacters(
            key.to_string(),
        ));
    }

    Ok(())
}

impl VariableDefinitionT {
    /// Checks if the variable definition is valid.
    ///
    /// Checks if the key adheres to the naming schema and if access_type and data_type are set properly.
    pub fn validate(&self) -> Result<(), InvalidVariableDefinitionError> {
        //Try to do quick O(1) checks first here, as this can speed up the validation time
        if self.access_type == VariableAccessType::UNSPECIFIED {
            return Err(InvalidVariableDefinitionError::UnspecifiedProperty(
                "access_type".to_string(),
            ));
        }
        if self.data_type == VariableDataType::UNSPECIFIED {
            return Err(InvalidVariableDefinitionError::UnspecifiedProperty(
                "data_type".to_string(),
            ));
        }

        validate_variable_key(&self.key)?;

        Ok(())
    }
}
