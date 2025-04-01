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

impl VariableDefinitionT {
    pub fn validate(&self) -> Result<(), InvalidVariableDefinitionError> {
        if self.key.is_empty() {
            return Err(InvalidVariableDefinitionError::UnnamedVariable);
        }
        if self.key.to_string().chars().count() > 1023 {
            return Err(InvalidVariableDefinitionError::InvalidLength(
                self.key.to_string(),
            ));
        }
        if self.key.ends_with('.') {
            return Err(InvalidVariableDefinitionError::TrailingDot(
                self.key.to_string(),
            ));
        }
        if !adheres_to_name_schema(&self.key) {
            return Err(InvalidVariableDefinitionError::InvalidCharacters(
                self.key.to_string(),
            ));
        }
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
        Ok(())
    }
}

pub fn adheres_to_name_schema(txt: &str) -> bool {
    // Only a-z,A-Z,0-9,_,. are allowed
    // The original pattern checking the length
    // ^(?=.{1,1023}$)[a-z]([a-z0-9-]{0,61}[a-z0-9])?(\/[a-z]([a-z0-9-]{0,61}[a-z0-9])?)*$
    // is not working because look-ahead and look-behind are not supported from Regex-crade.
    // So we check the length seperately outside this function.
    let pattern =
        Regex::new(r"^[a-zA-Z_]([a-zA-Z0-9_]{0,62})?(\.[a-zA-Z_]([a-zA-Z0-9_]{0,62})?)*$").unwrap();

    pattern.is_match(txt)
}
