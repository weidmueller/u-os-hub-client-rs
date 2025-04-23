//! Collection of types that are used in the consumer side of the library.
//! These types abstract the low level flatbuffer types and provide a more user-friendly interface.

use thiserror::Error;

use crate::{
    generated::weidmueller::ucontrol::hub::{
        State, VariableAccessType, VariableDataType, VariableDefinitionT, VariableQuality,
        VariableT,
    },
    variable::{self, value::DhTimestamp},
};

use super::connected_nats_provider::VariableID;

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
pub enum DhRegistryState {
    /// Registry is running
    Running,
    /// Registry is stopping
    Stopping,
    /// Registry state is not specified
    Unspecified,
}

impl TryFrom<State> for DhRegistryState {
    type Error = Error;

    fn try_from(value: State) -> std::result::Result<Self, Self::Error> {
        match value {
            State::RUNNING => Ok(DhRegistryState::Running),
            State::STOPPING => Ok(DhRegistryState::Stopping),
            State::UNSPECIFIED => Ok(DhRegistryState::Unspecified),
            _ => Err(Error::FlatbufferDataTypeConversionFailure),
        }
    }
}

/// The quality of a variable
///
/// This is set by the provider and indicates the quality of a variable value.
/// The `Uncertain` states are modelled after the OPC UA specification.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConsumerVariableQuality {
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

impl TryFrom<VariableQuality> for ConsumerVariableQuality {
    type Error = Error;

    fn try_from(value: VariableQuality) -> Result<Self, Self::Error> {
        match value {
            VariableQuality::BAD => Ok(ConsumerVariableQuality::BadOrUndefined),
            VariableQuality::GOOD => Ok(ConsumerVariableQuality::Good),
            VariableQuality::UNCERTAIN => Ok(ConsumerVariableQuality::Uncertain),
            VariableQuality::UNCERTAIN_LAST_USABLE_VALUE => {
                Ok(ConsumerVariableQuality::UncertainLastUsableValue)
            }
            VariableQuality::UNCERTAIN_INITIAL_VALUE => {
                Ok(ConsumerVariableQuality::UncertainInitialValue)
            }
            _ => Err(Error::FlatbufferDataTypeConversionFailure),
        }
    }
}

/// The type of the variable
#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum ConsumerVariableType {
    Float64,
    Int64,
    String,
    Timestamp,
    Duration,
    Boolean,
}

impl TryFrom<VariableDataType> for ConsumerVariableType {
    type Error = Error;

    fn try_from(value: VariableDataType) -> Result<Self, Self::Error> {
        match value {
            VariableDataType::FLOAT64 => Ok(ConsumerVariableType::Float64),
            VariableDataType::INT64 => Ok(ConsumerVariableType::Int64),
            VariableDataType::STRING => Ok(ConsumerVariableType::String),
            VariableDataType::TIMESTAMP => Ok(ConsumerVariableType::Timestamp),
            VariableDataType::DURATION => Ok(ConsumerVariableType::Duration),
            VariableDataType::BOOLEAN => Ok(ConsumerVariableType::Boolean),
            _ => Err(Error::FlatbufferDataTypeConversionFailure),
        }
    }
}

/// The definition of a variable
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConsumerVariableDefinition {
    /// The unique id of the variable
    pub id: VariableID,
    /// The variable key used to access the variable via the high level API
    pub key: String,
    /// Data type of the variable
    pub data_type: ConsumerVariableType,
    /// Whether the variable is read-only or not
    pub read_only: bool,
    /// Experimantal variables are hidden in the data hub GUI
    pub experimental: bool,
}

impl TryFrom<VariableDefinitionT> for ConsumerVariableDefinition {
    type Error = Error;

    fn try_from(ll_var_def: VariableDefinitionT) -> Result<Self, Self::Error> {
        let mapped_data_type = ll_var_def.data_type.try_into()?;

        Ok(ConsumerVariableDefinition {
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
pub struct ConsumerVariableState {
    /// The modification timestamp of the variable.
    ///
    /// Variables can have their own timestamp or inherit the variable list timestamp.
    pub timestamp: DhTimestamp,
    /// Current value of the variable
    pub value: variable::value::Value,
    /// The quality of the variable
    pub quality: ConsumerVariableQuality,
}

impl ConsumerVariableState {
    /// Creates a new variable state from a low level variable and a fallback timestamp.
    ///
    /// If the low level variable has a timestamp, it will be used. Otherwise, `fallback_timestamp` will be used.
    pub(super) fn new(ll_var: VariableT, fallback_timestamp: DhTimestamp) -> Result<Self, Error> {
        let mapped_ts = if let Some(ts) = ll_var.timestamp {
            ts.into()
        } else {
            fallback_timestamp
        };

        let mapped_value = Option::<variable::value::Value>::from(ll_var.value)
            .ok_or(Error::FlatbufferDataTypeConversionFailure)?;

        Ok(ConsumerVariableState {
            timestamp: mapped_ts,
            value: mapped_value,
            quality: ll_var.quality.try_into()?,
        })
    }
}
