//! Contains the variable builder. It's a helper to create variables.

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

use crate::variable::{
    value::{TimestampValue, VariableValue},
    Variable,
};

#[cfg(test)]
mod variable_builder_test;

/// Builder for creating a [`Variable`] in a safe and idiomatic way.
///
/// You may create the [`Variable`] struct directly but then you have no validation checks.
pub struct VariableBuilder {
    key: String,
    id: u32,
    read_only: bool,
    value: Option<VariableValue>,
    experimental: bool,
}

impl VariableBuilder {
    /// Create a new variable builder.
    pub fn new(id: u32, key: &str) -> Self {
        VariableBuilder {
            key: key.to_string(),
            read_only: true,
            value: None,
            id,
            experimental: false,
        }
    }

    /// Sets the variable to read write (optional)
    pub fn read_write(mut self) -> Self {
        self.read_only = false;
        self
    }

    /// Marks the variable as experimental (optional)
    ///
    /// Experimental means that the variable is not stable/reliable and it is hidden in the user interface.
    pub fn experimental(mut self) -> Self {
        self.experimental = true;
        self
    }

    /// Sets the initial value of the variable
    pub fn value(mut self, value: impl Into<VariableValue>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Tries to build the variable.
    ///
    /// It will return an error if any of the required fields are missing or if the key is invalid:
    /// Valid keys fit this regex: "^[a-zA-Z_]([a-zA-Z0-9_]{0,62})?(\.[a-zA-Z_]([a-zA-Z0-9_]{0,62})?)*$"
    pub fn build(self) -> Result<Variable, VariableBuildError> {
        static KEY_PATTERN: Lazy<Regex> = Lazy::new(|| {
            //Safety: This should actually be a hard error because an invalid regex would be a developer error
            #[allow(clippy::expect_used)]
            Regex::new(r"^[a-zA-Z_]([a-zA-Z0-9_]{0,62})?(\.[a-zA-Z_]([a-zA-Z0-9_]{0,62})?)*$")
                .expect("this regex should be valid")
        });

        if !KEY_PATTERN.is_match(self.key.as_str()) {
            return Err(VariableBuildError::InvalidVariableName(self.key));
        }

        if let Some(value) = self.value {
            Ok(Variable {
                value,
                read_only: self.read_only,
                key: self.key,
                id: self.id,
                experimental: self.experimental,
                last_value_change: TimestampValue::now(),
            })
        } else {
            Err(VariableBuildError::MissingValue)
        }
    }
}

/// The error indicating what part of the variable building process failed
/// will only contain the first error encountered
#[derive(Error, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum VariableBuildError {
    #[error("Invalid variable name `{0}`")]
    InvalidVariableName(String),
    #[error("Missing value")]
    MissingValue,
}
