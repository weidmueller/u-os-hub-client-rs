//! Collection of types that are used in the consumer side of the library.
//! These types abstract the low level flatbuffer types and provide a more user-friendly interface.

use thiserror::Error;

use crate::{
    generated::weidmueller::ucontrol::hub::{
        State, VariableAccessType, VariableDataType, VariableDefinitionT, VariableT,
    },
    variable::{self, value::TimestampValue},
};

use crate::generated::weidmueller::ucontrol::hub::VariableQuality as FbVariableQuality;

/// Represents a variable ID on the hub.
pub type VariableID = u32;

//TODO: how to handle type conversion errors? should that really cause an error, or do we want to have placeholder values?

/// Errors for data hub type conversions
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Failed to convert low level flatbuffer value")]
    FlatbufferDataTypeConversionFailure,
}

/// The state of the Data Hub registry
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RegistryState {
    /// Registry is running
    Running,
    /// Registry is stopping
    Stopping,
    /// Registry state is not specified
    Unspecified,
}

impl TryFrom<State> for RegistryState {
    type Error = Error;

    fn try_from(value: State) -> std::result::Result<Self, Self::Error> {
        match value {
            State::RUNNING => Ok(RegistryState::Running),
            State::STOPPING => Ok(RegistryState::Stopping),
            State::UNSPECIFIED => Ok(RegistryState::Unspecified),
            _ => Err(Error::FlatbufferDataTypeConversionFailure),
        }
    }
}

/// The quality of a variable
///
/// This is set by the provider and indicates the quality of a variable value.
/// The `Uncertain` states are modelled after the OPC UA specification.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VariableQuality {
    /// Indicates that the value is not usable
    BadOrUndefined,
    /// Indicates that the value is good and can be used without restrictions
    Good,
    /// Variable quality is uncertain without any specific reason, but may still be usable e.g. for display or other non critical purposes
    Uncertain,
    /// The variable has stopped updating and contains its last usable value
    UncertainLastUsableValue,
    /// Variable has not been written yet and still contains a default initial value
    UncertainInitialValue,
}

impl TryFrom<FbVariableQuality> for VariableQuality {
    type Error = Error;

    fn try_from(value: FbVariableQuality) -> Result<Self, Self::Error> {
        match value {
            FbVariableQuality::BAD => Ok(VariableQuality::BadOrUndefined),
            FbVariableQuality::GOOD => Ok(VariableQuality::Good),
            FbVariableQuality::UNCERTAIN => Ok(VariableQuality::Uncertain),
            FbVariableQuality::UNCERTAIN_LAST_USABLE_VALUE => {
                Ok(VariableQuality::UncertainLastUsableValue)
            }
            FbVariableQuality::UNCERTAIN_INITIAL_VALUE => {
                Ok(VariableQuality::UncertainInitialValue)
            }
            _ => Err(Error::FlatbufferDataTypeConversionFailure),
        }
    }
}

/// The type of the variable
#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum VariableType {
    Float64,
    Int64,
    String,
    Timestamp,
    Duration,
    Boolean,
}

impl TryFrom<VariableDataType> for VariableType {
    type Error = Error;

    fn try_from(value: VariableDataType) -> Result<Self, Self::Error> {
        match value {
            VariableDataType::FLOAT64 => Ok(VariableType::Float64),
            VariableDataType::INT64 => Ok(VariableType::Int64),
            VariableDataType::STRING => Ok(VariableType::String),
            VariableDataType::TIMESTAMP => Ok(VariableType::Timestamp),
            VariableDataType::DURATION => Ok(VariableType::Duration),
            VariableDataType::BOOLEAN => Ok(VariableType::Boolean),
            _ => Err(Error::FlatbufferDataTypeConversionFailure),
        }
    }
}

/// The definition of a variable
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VariableDefinition {
    /// The unique id of the variable
    pub id: VariableID,
    /// The variable key used to access the variable via the high level API
    pub key: String,
    /// Data type of the variable
    pub data_type: VariableType,
    /// Whether the variable is read-only or not
    pub read_only: bool,
    /// Experimantal variables are hidden in the data hub GUI
    pub experimental: bool,
}

impl TryFrom<VariableDefinitionT> for VariableDefinition {
    type Error = Error;

    fn try_from(ll_var_def: VariableDefinitionT) -> Result<Self, Self::Error> {
        let mapped_data_type = ll_var_def.data_type.try_into()?;

        Ok(VariableDefinition {
            id: ll_var_def.id,
            key: ll_var_def.key,
            data_type: mapped_data_type,
            read_only: (ll_var_def.access_type != VariableAccessType::READ_WRITE),
            experimental: ll_var_def.experimental,
        })
    }
}

/// The state of a variable
#[derive(Debug, Clone, PartialEq)]
pub struct VariableState {
    /// The modification timestamp of the variable.
    ///
    /// Variables can have their own timestamp or inherit the variable list timestamp.
    pub timestamp: TimestampValue,
    /// Current value of the variable
    pub value: variable::value::VariableValue,
    /// The quality of the variable
    pub quality: VariableQuality,
}

impl VariableState {
    /// Creates a new variable state from a low level variable and a fallback timestamp.
    ///
    /// If the low level variable has a timestamp, it will be used. Otherwise, `fallback_timestamp` will be used.
    pub(super) fn new(
        ll_var: VariableT,
        fallback_timestamp: TimestampValue,
    ) -> Result<Self, Error> {
        let mapped_ts = if let Some(ts) = ll_var.timestamp {
            ts.into()
        } else {
            fallback_timestamp
        };

        let mapped_value = Option::<variable::value::VariableValue>::from(ll_var.value)
            .ok_or(Error::FlatbufferDataTypeConversionFailure)?;

        Ok(VariableState {
            timestamp: mapped_ts,
            value: mapped_value,
            quality: ll_var.quality.try_into()?,
        })
    }
}
