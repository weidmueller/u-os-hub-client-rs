//! Contains the variable builder. It's a helper to create variables.

use thiserror::Error;

use crate::{
    dh_types::{
        TimestampValue, VariableDefinition, VariableID, VariableQuality, VariableType,
        VariableValue,
    },
    provider::provider_types::VariableState,
    variable::Variable,
};

use super::variable_definition_validator::validate_variable_key;

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
    /// This is optional. By default, the timestamp is set to [`TimestampValue::now()`] when the variable is built.
    ///
    /// If timestamp is set to `None`, the variable will inherit its timestamp from the variable list
    /// when a consumer receives a variable update or reads the variable list explicitly.
    /// This can be a useful optimization if you have a very large variable list,
    /// as this reduces the payload size for the data hub.
    /// However, be aware that if the variable timestamp is `None`, consumers will always receive the timestamp of
    /// reading the variable list instead of the timestamp of the last value update, which may not be what the consumer expects, so
    /// use this optimization with caution.
    pub fn initial_timestamp(mut self, timestamp: Option<TimestampValue>) -> Self {
        self.override_timestamp = Some(timestamp);
        self
    }

    /// Tries to build the variable.
    ///
    /// It will return an error if any of the required fields are missing or if the key is invalid.
    /// Variable keys are validated using the [`validate_variable_key`] function.
    pub fn build(self) -> Result<Variable, VariableBuildError> {
        if validate_variable_key(&self.key).is_err() {
            return Err(VariableBuildError::InvalidVariableName(self.key));
        }

        if let Some(value) = self.value {
            Ok(Variable {
                definition: VariableDefinition {
                    key: self.key,
                    id: self.id,
                    data_type: Self::infer_variable_type_from_value(&value)?,
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

    fn infer_variable_type_from_value(
        value: &VariableValue,
    ) -> Result<VariableType, VariableBuildError> {
        match value {
            VariableValue::Unknown => Err(VariableBuildError::InvalidValue),
            VariableValue::Int(_) => Ok(VariableType::Int64),
            VariableValue::Float64(_) => Ok(VariableType::Float64),
            VariableValue::String(_) => Ok(VariableType::String),
            VariableValue::Boolean(_) => Ok(VariableType::Boolean),
            VariableValue::Timestamp(_) => Ok(VariableType::Timestamp),
            VariableValue::Duration(_) => Ok(VariableType::Duration),
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
    #[error("Invalid value")]
    InvalidValue,
}
