//! Collection of types that are used in the consumer side of the library.
//! These types abstract the low level flatbuffer types and provide a more user-friendly interface.

use crate::{
    dh_types::{self, VariableQuality},
    generated::weidmueller::ucontrol::hub::{State, VariableT},
    variable::{self, value::TimestampValue},
};

//TODO: how to handle type conversion errors? should that really cause an error, or do we want to have placeholder values?

/// Errors for data hub type conversions
pub type Error = dh_types::Error;

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
