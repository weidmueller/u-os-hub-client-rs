//! Contains the value of a variable.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::generated::weidmueller::ucontrol::hub::{
    DurationT, TimestampT, VariableValueBooleanT, VariableValueDurationT, VariableValueFloat64T,
    VariableValueInt64T, VariableValueStringT, VariableValueT, VariableValueTimestampT,
};

// TODO: We could try to use generics. The datatype shoudn't be changeable so we could move this check to compile time.
/// The value of a variable.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Boolean(bool),
    String(String),
    Float64(f64),
    Duration(Duration),
    Timestamp(SystemTime),
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

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float64(value)
    }
}

impl From<Duration> for Value {
    fn from(value: Duration) -> Self {
        Value::Duration(value)
    }
}

impl From<SystemTime> for Value {
    fn from(value: SystemTime) -> Self {
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
                let secs = Duration::from_secs(value.seconds as u64);
                let nanos = Duration::from_nanos(value.nanos as u64);

                Value::Duration(secs + nanos)
            }
            VariableValueT::Float64(v) => Value::Float64(v.value),
            VariableValueT::Int64(v) => Value::Int(v.value),
            VariableValueT::String(v) => Value::String(v.value?),
            VariableValueT::Timestamp(v) => {
                let value = v.value?;
                let secs = Duration::from_secs(value.seconds as u64);
                let nanos = Duration::from_nanos(value.nanos as u64);

                Value::Timestamp(UNIX_EPOCH + secs + nanos)
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
                    value: Some(DurationT {
                        seconds: val.as_secs() as i64,
                        nanos: val.subsec_nanos() as i32,
                    }),
                };
                VariableValueT::Duration(Box::new(val_t))
            }
            Value::Timestamp(val) => {
                let mut val_t = VariableValueTimestampT::default();
                let mut time_t = TimestampT::default();
                //TODO: Support timestamps before unix epoch
                let value_since_unix = val
                    .duration_since(UNIX_EPOCH)
                    .expect("should get duration since unix epoch (you can only use timestamps after this epoch)");
                time_t.seconds = value_since_unix.as_secs() as i64;
                time_t.nanos = value_since_unix.subsec_nanos() as i32;
                val_t.value = Some(time_t);
                VariableValueT::Timestamp(Box::new(val_t))
            }
        }
    }
}
