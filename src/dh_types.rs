//! Collection of types that are used in the library.
//! These types abstract the low level flatbuffer types and provide a more user-friendly interface.

use thiserror::Error;

use crate::generated::weidmueller::ucontrol::hub::{
    DurationT, TimestampT, VariableValueBooleanT, VariableValueDurationT, VariableValueFloat64T,
    VariableValueInt64T, VariableValueStringT, VariableValueT, VariableValueTimestampT,
};
use crate::generated::weidmueller::ucontrol::hub::{
    VariableAccessType, VariableDataType, VariableDefinitionT,
};

use crate::generated::weidmueller::ucontrol::hub::VariableQuality as FbVariableQuality;

/// Represents a variable ID on the hub.
pub type VariableID = u32;

/// Errors for data hub type conversions
//TODO: how to handle type conversion errors? should that really cause an error, or do we want to have placeholder values?
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Failed to convert low level flatbuffer value")]
    FlatbufferDataTypeConversionFailure,
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

impl From<VariableQuality> for FbVariableQuality {
    fn from(value: VariableQuality) -> Self {
        match value {
            VariableQuality::BadOrUndefined => FbVariableQuality::BAD,
            VariableQuality::Good => FbVariableQuality::GOOD,
            VariableQuality::Uncertain => FbVariableQuality::UNCERTAIN,
            VariableQuality::UncertainLastUsableValue => {
                FbVariableQuality::UNCERTAIN_LAST_USABLE_VALUE
            }
            VariableQuality::UncertainInitialValue => FbVariableQuality::UNCERTAIN_INITIAL_VALUE,
        }
    }
}

/// The type of the variable
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

impl From<VariableType> for VariableDataType {
    fn from(value: VariableType) -> Self {
        match value {
            VariableType::Float64 => VariableDataType::FLOAT64,
            VariableType::Int64 => VariableDataType::INT64,
            VariableType::String => VariableDataType::STRING,
            VariableType::Timestamp => VariableDataType::TIMESTAMP,
            VariableType::Duration => VariableDataType::DURATION,
            VariableType::Boolean => VariableDataType::BOOLEAN,
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

/// User friendly duration type that abstracts the low level flatbuffer value
pub type DurationValue = time::Duration;

impl From<DurationT> for DurationValue {
    fn from(value: DurationT) -> Self {
        DurationValue::new(value.seconds, value.nanos)
    }
}

impl From<DurationValue> for DurationT {
    fn from(value: DurationValue) -> Self {
        Self {
            seconds: value.whole_seconds(),
            nanos: value.subsec_nanoseconds(),
        }
    }
}

/// User friendly timestamp type that abstracts the low level flatbuffer value
pub type TimestampValue = time::UtcDateTime;

impl From<TimestampT> for TimestampValue {
    fn from(value: TimestampT) -> Self {
        TimestampValue::UNIX_EPOCH + DurationValue::new(value.seconds, value.nanos)
    }
}

impl From<TimestampValue> for TimestampT {
    fn from(value: TimestampValue) -> Self {
        let duration_since_epoch = value - TimestampValue::UNIX_EPOCH;

        let mut seconds = duration_since_epoch.whole_seconds();
        let mut nanos = duration_since_epoch.subsec_nanoseconds();

        //We want our timestamps on the hub to adhere to the google timestamp definition, which requires
        //nanos to be positive, but time::UtcDateTime can use negative nanos by default.
        //For example, -1.5s before EPOCH is represented as -1 sec and -500_000_000 nanos in time::UtcDateTime, but we want it to
        //be represented as -2 sec and 500_000_000 nanos instead.
        //So if nanos are negative, we subtract 1 from seconds and calculate the remaining positive nanos
        if nanos < 0 {
            seconds -= 1;
            nanos += 1_000_000_000;
        }

        Self { seconds, nanos }
    }
}

/// The value of a variable.
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum VariableValue {
    Int(i64),
    Boolean(bool),
    String(String),
    Float64(f64),
    Duration(DurationValue),
    Timestamp(TimestampValue),
}

impl From<i64> for VariableValue {
    fn from(value: i64) -> Self {
        VariableValue::Int(value)
    }
}

impl From<bool> for VariableValue {
    fn from(value: bool) -> Self {
        VariableValue::Boolean(value)
    }
}

impl From<&str> for VariableValue {
    fn from(value: &str) -> Self {
        VariableValue::String(value.to_string())
    }
}

impl From<String> for VariableValue {
    fn from(value: String) -> Self {
        VariableValue::String(value)
    }
}

impl From<f64> for VariableValue {
    fn from(value: f64) -> Self {
        VariableValue::Float64(value)
    }
}

impl From<DurationValue> for VariableValue {
    fn from(value: DurationValue) -> Self {
        VariableValue::Duration(value)
    }
}

impl From<TimestampValue> for VariableValue {
    fn from(value: TimestampValue) -> Self {
        VariableValue::Timestamp(value)
    }
}

impl From<VariableValueT> for Option<VariableValue> {
    fn from(value: VariableValueT) -> Self {
        Some(match value {
            VariableValueT::NONE => None?,
            VariableValueT::Boolean(v) => VariableValue::Boolean(v.value),
            VariableValueT::Duration(v) => {
                let value = v.value?;
                VariableValue::Duration(value.into())
            }
            VariableValueT::Float64(v) => VariableValue::Float64(v.value),
            VariableValueT::Int64(v) => VariableValue::Int(v.value),
            VariableValueT::String(v) => VariableValue::String(v.value?),
            VariableValueT::Timestamp(v) => {
                let value: TimestampT = v.value?;
                VariableValue::Timestamp(value.into())
            }
        })
    }
}

impl From<&VariableValue> for VariableValueT {
    fn from(value: &VariableValue) -> VariableValueT {
        match value {
            VariableValue::Int(val) => {
                let val_t = VariableValueInt64T { value: *val };
                VariableValueT::Int64(Box::new(val_t))
            }
            VariableValue::Boolean(val) => {
                let val_t = VariableValueBooleanT { value: *val };
                VariableValueT::Boolean(Box::new(val_t))
            }
            VariableValue::String(val) => {
                let val_t = VariableValueStringT {
                    value: Some(val.to_string()),
                };
                VariableValueT::String(Box::new(val_t))
            }
            VariableValue::Float64(val) => {
                let val_t = VariableValueFloat64T { value: *val };
                VariableValueT::Float64(Box::new(val_t))
            }
            VariableValue::Duration(val) => {
                let val_t = VariableValueDurationT {
                    value: Some((*val).into()),
                };

                VariableValueT::Duration(Box::new(val_t))
            }
            VariableValue::Timestamp(val) => {
                let val_t = VariableValueTimestampT {
                    value: Some((*val).into()),
                };

                VariableValueT::Timestamp(Box::new(val_t))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::generated::weidmueller::ucontrol::hub::TimestampT;
    use rstest::rstest;

    #[rstest]
    #[case::pos_seconds_and_nanos(1, 600_000_000, TimestampT{seconds: 1, nanos: 600_000_000})]
    #[case::pos_seconds_neg_nanos(1, -600_000_000, TimestampT{seconds: 0, nanos: 400_000_000})]
    #[case::neg_seconds_and_nanos(-1, 600_000_000, TimestampT{seconds: -1, nanos: 600_000_000})]
    #[case::neg_seconds_neg_nanos(-1, -600_000_000, TimestampT{seconds: -2, nanos: 400_000_000})]
    fn test_timestamp_conversion_always_positive_nanos(
        #[case] seconds_since_epoch: i64,
        #[case] nanos_since_epoch: i32,
        #[case] expected_fb_timestamp: TimestampT,
    ) {
        let dh_timestamp =
            TimestampValue::UNIX_EPOCH + DurationValue::new(seconds_since_epoch, nanos_since_epoch);

        let flatbuffer_timestamp: TimestampT = dh_timestamp.into();
        assert_eq!(flatbuffer_timestamp, expected_fb_timestamp);

        let dh_timestamp_converted: TimestampValue = flatbuffer_timestamp.into();
        assert_eq!(dh_timestamp_converted, dh_timestamp);
    }
}
