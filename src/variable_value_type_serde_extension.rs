// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

use serde::{
    ser::{Error, SerializeStruct},
    Serialize, Serializer,
};

use crate::generated::weidmueller::ucontrol::hub::VariableValueT;

impl Serialize for VariableValueT {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            VariableValueT::NONE => todo!(),
            VariableValueT::Boolean(_) => match self.as_boolean().map(|b| b.value) {
                Some(value) => serializer.serialize_bool(value),
                None => Err(Error::custom("Serialization to boolean value failed.")),
            },
            VariableValueT::Timestamp(_) => {
                let mut timestamp_struct = serializer.serialize_struct("timestamp", 2)?;
                let timestamp = &self
                    .as_timestamp()
                    .ok_or::<S::Error>(Error::custom("Serialization to timestamp value failed."))?;

                let time_value = timestamp
                    .value
                    .clone()
                    .ok_or(Error::custom("Serialization to timestamp value failed."))?;

                timestamp_struct.serialize_field("seconds", &time_value.seconds)?;
                timestamp_struct.serialize_field("nanos", &time_value.nanos)?;

                timestamp_struct.end()
            }
            VariableValueT::Duration(_) => {
                let mut duration_struct = serializer.serialize_struct("duration", 2)?;
                let duration = &self
                    .as_duration()
                    .ok_or::<S::Error>(Error::custom("Serialization to duration value failed."))?;

                let time_value = duration
                    .value
                    .clone()
                    .ok_or(Error::custom("Serialization to duration value failed."))?;

                duration_struct.serialize_field("seconds", &time_value.seconds)?;
                duration_struct.serialize_field("nanos", &time_value.nanos)?;

                duration_struct.end()
            }
            VariableValueT::Float64(_) => match self.as_float_64().map(|d| d.value) {
                Some(value) => serializer.serialize_f64(value),
                None => Err(Error::custom("Serialization to double value failed.")),
            },
            VariableValueT::Int64(_) => match self.as_int_64().map(|i| i.value) {
                Some(value) => serializer.serialize_i64(value),
                None => Err(Error::custom("Serialization to integer value failed.")),
            },
            VariableValueT::String(_) => {
                if let Some(str) = self.as_string().map(|s| s.value.clone()) {
                    match str {
                        Some(str) => serializer.serialize_str(str.as_str()),
                        None => Err(Error::custom("Serialization to string value failed.")),
                    }
                } else {
                    Err(Error::custom("Serialization to string value failed."))
                }
            }
        }
    }
}
