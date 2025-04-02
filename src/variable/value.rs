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

                //TODO: this currently underflows for negative values, so we set it to zero in that case.
                //See: https://devops-weidmueller.atlassian.net/browse/UC20-14743
                let secs = Duration::from_secs(u64::try_from(value.seconds).unwrap_or_default());
                let nanos = Duration::from_nanos(u64::try_from(value.nanos).unwrap_or_default());

                Value::Duration(secs + nanos)
            }
            VariableValueT::Float64(v) => Value::Float64(v.value),
            VariableValueT::Int64(v) => Value::Int(v.value),
            VariableValueT::String(v) => Value::String(v.value?),
            VariableValueT::Timestamp(v) => {
                let value = v.value?;

                //Note: Duration can not be negative, but SystemTime can!
                //So we create positive durations via abs,
                //but subtract them from the UNIX_EPOCH if they have a negative source value
                //See: https://devops-weidmueller.atlassian.net/browse/UC20-14743
                let secs = Duration::from_secs(value.seconds.unsigned_abs());
                let nanos = Duration::from_nanos(u64::from(value.nanos.unsigned_abs()));

                let mut system_time = UNIX_EPOCH;

                if value.seconds < 0 {
                    system_time -= secs;
                } else {
                    system_time += secs;
                }

                if value.nanos < 0 {
                    system_time -= nanos;
                } else {
                    system_time += nanos;
                }

                Value::Timestamp(system_time)
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
                        //TODO: this may overflow for very large integers, so we set it to zero in that case.
                        //See: https://devops-weidmueller.atlassian.net/browse/UC20-14743
                        seconds: i64::try_from(val.as_secs()).unwrap_or_default(),
                        nanos: i32::try_from(val.subsec_nanos()).unwrap_or_default(),
                    }),
                };
                VariableValueT::Duration(Box::new(val_t))
            }
            Value::Timestamp(val) => {
                let mut val_t = VariableValueTimestampT::default();
                let mut time_t = TimestampT::default();
                //TODO: Support timestamps before unix epoch
                //For now, we set it to zero in that case.
                //See: https://devops-weidmueller.atlassian.net/browse/UC20-14743
                let value_since_unix = val.duration_since(UNIX_EPOCH).unwrap_or_default();

                //TODO: this may overflow for very large integers, so we set it to zero in that case.
                time_t.seconds = i64::try_from(value_since_unix.as_secs()).unwrap_or_default();
                time_t.nanos = i32::try_from(value_since_unix.subsec_nanos()).unwrap_or_default();
                val_t.value = Some(time_t);
                VariableValueT::Timestamp(Box::new(val_t))
            }
        }
    }
}
