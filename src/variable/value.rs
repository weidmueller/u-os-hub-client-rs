//! Contains the value of a variable.
use crate::generated::weidmueller::ucontrol::hub::{
    DurationT, TimestampT, VariableValueBooleanT, VariableValueDurationT, VariableValueFloat64T,
    VariableValueInt64T, VariableValueStringT, VariableValueT, VariableValueTimestampT,
};

/// User friendly duration type that abstracts the low level flatbuffer value
pub type DhDuration = time::Duration;

impl From<DurationT> for DhDuration {
    fn from(value: DurationT) -> Self {
        DhDuration::new(value.seconds, value.nanos)
    }
}

impl From<DhDuration> for DurationT {
    fn from(value: DhDuration) -> Self {
        Self {
            seconds: value.whole_seconds(),
            nanos: value.subsec_nanoseconds(),
        }
    }
}

/// User friendly timestamp type that abstracts the low level flatbuffer value
pub type DhTimestamp = time::UtcDateTime;

impl From<TimestampT> for DhTimestamp {
    fn from(value: TimestampT) -> Self {
        DhTimestamp::UNIX_EPOCH + DhDuration::new(value.seconds, value.nanos)
    }
}

impl From<DhTimestamp> for TimestampT {
    fn from(value: DhTimestamp) -> Self {
        let duration_since_epoch = value - DhTimestamp::UNIX_EPOCH;

        Self {
            seconds: duration_since_epoch.whole_seconds(),
            nanos: duration_since_epoch.subsec_nanoseconds(),
        }
    }
}

// TODO: We could try to use generics. The datatype shoudn't be changeable so we could move this check to compile time.
/// The value of a variable.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Boolean(bool),
    String(String),
    Float64(f64),
    Duration(DhDuration),
    Timestamp(DhTimestamp),
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_string())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float64(value)
    }
}

impl From<DhDuration> for Value {
    fn from(value: DhDuration) -> Self {
        Value::Duration(value)
    }
}

impl From<DhTimestamp> for Value {
    fn from(value: DhTimestamp) -> Self {
        Value::Timestamp(value)
    }
}

impl From<VariableValueT> for Option<Value> {
    fn from(value: VariableValueT) -> Self {
        Some(match value {
            VariableValueT::NONE => None?,
            VariableValueT::Boolean(v) => Value::Boolean(v.value),
            VariableValueT::Duration(v) => {
                let value = v.value?;
                Value::Duration(value.into())
            }
            VariableValueT::Float64(v) => Value::Float64(v.value),
            VariableValueT::Int64(v) => Value::Int(v.value),
            VariableValueT::String(v) => Value::String(v.value?),
            VariableValueT::Timestamp(v) => {
                let value: TimestampT = v.value?;
                Value::Timestamp(value.into())
            }
        })
    }
}

impl From<&Value> for VariableValueT {
    fn from(value: &Value) -> VariableValueT {
        match value {
            Value::Int(val) => {
                let val_t = VariableValueInt64T { value: *val };
                VariableValueT::Int64(Box::new(val_t))
            }
            Value::Boolean(val) => {
                let val_t = VariableValueBooleanT { value: *val };
                VariableValueT::Boolean(Box::new(val_t))
            }
            Value::String(val) => {
                let val_t = VariableValueStringT {
                    value: Some(val.to_string()),
                };
                VariableValueT::String(Box::new(val_t))
            }
            Value::Float64(val) => {
                let val_t = VariableValueFloat64T { value: *val };
                VariableValueT::Float64(Box::new(val_t))
            }
            Value::Duration(val) => {
                let val_t = VariableValueDurationT {
                    value: Some((*val).into()),
                };

                VariableValueT::Duration(Box::new(val_t))
            }
            Value::Timestamp(val) => {
                let val_t = VariableValueTimestampT {
                    value: Some((*val).into()),
                };

                VariableValueT::Timestamp(Box::new(val_t))
            }
        }
    }
}
