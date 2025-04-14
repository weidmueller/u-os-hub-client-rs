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
pub enum Error {
    #[error("Failed to convert low level flatbuffer value")]
    FlatbufferDataTypeConversionFailure,
}

/// The state of the Data Hub registry
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DhRegistryState {
    Running,
    Stopping,
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
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConsumerVariableQuality {
    BadOrUndefined,
    Good,
    Uncertain,
    UncertainLastUsableValue,
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
    pub id: VariableID,
    pub key: String,
    pub data_type: ConsumerVariableType,
    pub read_only: bool,
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
    pub timestamp: DhTimestamp,
    pub value: variable::value::Value,
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
