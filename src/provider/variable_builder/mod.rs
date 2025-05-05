//! Contains the variable builder. It's a helper to create variables.

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

use crate::{
    dh_types::{
        TimestampValue, VariableDefinition, VariableID, VariableQuality, VariableType,
        VariableValue,
    },
    provider::provider_types::VariableState,
    variable::Variable,
};

#[cfg(test)]
mod variable_builder_test;

/// Builder for creating a [`Variable`] in a safe and idiomatic way.
///
/// You may create the [`Variable`] struct directly but then you have no validation checks.
pub struct VariableBuilder {
    //definition
    key: String,
    id: u32,
    read_only: bool,
    experimental: bool,
    //state
    value: Option<VariableValue>,
    quality: VariableQuality,
    //Outer option is used to determine if the value was set in the builder,
    //if not a new timestamp will be generated when building the variable
    override_timestamp: Option<Option<TimestampValue>>,
}

impl VariableBuilder {
    /// Create a new variable builder.
    pub fn new(id: VariableID, key: impl Into<String>) -> Self {
        VariableBuilder {
            id,
            key: key.into(),
            read_only: true,
            experimental: false,
            value: None,
            quality: VariableQuality::Good,
            override_timestamp: None,
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

    /// Sets the initial value of the variable.
    ///
    /// You must set a value before calling [`Self::build`].
    /// During building, the type of the variable is inferred from the value.
    ///
    /// If the initial value is meant to be a temporary placeholder, consider using [`Self::initial_quality`]
    /// to set the quality to [`VariableQuality::UncertainInitialValue`].
    pub fn initial_value(mut self, value: impl Into<VariableValue>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Sets the initial quality of the variable
    ///
    /// This is optional. By default, the quality is set to [`VariableQuality::Good`].
    pub fn initial_quality(mut self, quality: VariableQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Sets the initial timestamp of the variable
    ///
    /// This is optional. By default, the timestamp is set to [`TimestampValue::now()`].
    pub fn initial_timestamp(mut self, timestamp: Option<TimestampValue>) -> Self {
        self.override_timestamp = Some(timestamp);
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
                definition: VariableDefinition {
                    key: self.key,
                    id: self.id,
                    data_type: Self::infer_variable_type_from_value(&value),
                    read_only: self.read_only,
                    experimental: self.experimental,
                },
                state: VariableState {
                    id: self.id,
                    value,
                    quality: self.quality,
                    timestamp: if let Some(override_timestamp) = self.override_timestamp {
                        override_timestamp
                    } else {
                        Some(TimestampValue::now())
                    },
                },
            })
        } else {
            Err(VariableBuildError::MissingValue)
        }
    }

    fn infer_variable_type_from_value(value: &VariableValue) -> VariableType {
        match value {
            VariableValue::Int(_) => VariableType::Int64,
            VariableValue::Float64(_) => VariableType::Float64,
            VariableValue::String(_) => VariableType::String,
            VariableValue::Boolean(_) => VariableType::Boolean,
            VariableValue::Timestamp(_) => VariableType::Timestamp,
            VariableValue::Duration(_) => VariableType::Duration,
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
