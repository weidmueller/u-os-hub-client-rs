//! Contains a variable with definition and value
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::{Hash, Hasher},
};

use crate::generated::weidmueller::ucontrol::hub::{
    VariableAccessType, VariableDefinitionT, VariableQuality, VariableT,
};

pub mod value;
use value::{TimestampValue, VariableValue};

/// Holds information about a variable (definition and value).
///
/// Warning: If you initialise this struct directly, there is no validation.
/// Instead you should use the [`VariableBuilder`](crate::provider::VariableBuilder) struct.
#[derive(Clone, Debug, PartialEq)]
pub struct Variable {
    /// [`VariableValue`] of a variable. You should only change the value and not the enum discriminant.
    pub value: VariableValue,
    /// Variable access type. True = Readonly, False = ReadWrite
    pub read_only: bool,
    /// Key of the variable.
    pub key: String,
    /// Id for the access without definition.
    pub id: u32,
    /// Experimental marker
    pub experimental: bool,
    /// Latest value change (will be returned on variable read request).
    pub last_value_change: TimestampValue,
}

impl From<&Variable> for VariableT {
    fn from(val: &Variable) -> Self {
        VariableT {
            quality: VariableQuality::GOOD,
            id: val.id,
            value: (&val.value).into(),
            timestamp: Some(val.last_value_change.into()),
            ..Default::default()
        }
    }
}

impl From<&Variable> for VariableDefinitionT {
    fn from(variable: &Variable) -> VariableDefinitionT {
        VariableDefinitionT {
            key: variable.key.clone(),
            id: variable.id,
            data_type: match variable.value {
                VariableValue::Int(_) => {
                    crate::generated::weidmueller::ucontrol::hub::VariableDataType::INT64
                }
                VariableValue::Boolean(_) => {
                    crate::generated::weidmueller::ucontrol::hub::VariableDataType::BOOLEAN
                }
                VariableValue::String(_) => {
                    crate::generated::weidmueller::ucontrol::hub::VariableDataType::STRING
                }
                VariableValue::Float64(_) => {
                    crate::generated::weidmueller::ucontrol::hub::VariableDataType::FLOAT64
                }
                VariableValue::Duration(_) => {
                    crate::generated::weidmueller::ucontrol::hub::VariableDataType::DURATION
                }
                VariableValue::Timestamp(_) => {
                    crate::generated::weidmueller::ucontrol::hub::VariableDataType::TIMESTAMP
                }
            },
            access_type: match variable.read_only {
                true => VariableAccessType::READ_ONLY,
                false => VariableAccessType::READ_WRITE,
            },
            experimental: variable.experimental,
        }
    }
}

// TODO: Find a better location for this (Should it be a trait?)
/// Calculates a hash over multiple variables (without value)
///
/// This can be used as a fingerprint for the provider definition.
pub fn calc_variables_hash(variables: &BTreeMap<u32, Variable>) -> u64 {
    let mut hasher = DefaultHasher::default();
    variables.iter().for_each(|(_, variable)| {
        variable.key.hash(&mut hasher);
        variable.read_only.hash(&mut hasher);
        variable.id.hash(&mut hasher);
        variable.experimental.hash(&mut hasher);
        std::mem::discriminant(&variable.value).hash(&mut hasher);
    });
    hasher.finish()
}
