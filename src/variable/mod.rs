// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

//! Contains a variable with definition and value
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::{Hash, Hasher},
};

use crate::{
    dh_types::VariableDefinition,
    generated::weidmueller::ucontrol::hub::{VariableDefinitionT, VariableT},
    provider::provider_types::VariableState,
};

/// Holds information about a variable (definition and value).
///
/// Warning: If you initialise this struct directly, there is no validation.
/// Instead you should use the [`VariableBuilder`](crate::provider::VariableBuilder) struct.
#[derive(Clone, Debug, PartialEq)]
pub struct Variable {
    pub(crate) state: VariableState,
    pub(crate) definition: VariableDefinition,
}

impl Variable {
    /// Returns the immutable state of the variable.
    #[inline(always)]
    pub fn get_state(&self) -> &VariableState {
        &self.state
    }

    /// Returns the mutable state of the variable.
    ///
    /// This can be used to change value, quality and timestamp.
    /// See documentation of [`VariableState`] methods for more details.
    #[inline(always)]
    pub fn get_mut_state(&mut self) -> &mut VariableState {
        &mut self.state
    }

    /// Returns the definition of the variable.
    #[inline(always)]
    pub fn get_definition(&self) -> &VariableDefinition {
        &self.definition
    }
}

impl From<&Variable> for VariableT {
    fn from(var: &Variable) -> Self {
        VariableT {
            quality: (*var.state.get_quality()).into(),
            id: var.definition.id,
            value: var.state.get_value().into(),
            timestamp: var.state.get_timestamp().map(|ts| ts.into()),
        }
    }
}

impl From<&Variable> for VariableDefinitionT {
    fn from(var: &Variable) -> VariableDefinitionT {
        let var_def = var.get_definition();

        VariableDefinitionT {
            key: var_def.key.clone(),
            id: var_def.id,
            data_type: var_def.data_type.into(),
            access_type: var_def.access_type.into(),
            experimental: var_def.experimental,
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
        let var_def = variable.get_definition();

        var_def.key.hash(&mut hasher);
        var_def.access_type.hash(&mut hasher);
        var_def.id.hash(&mut hasher);
        var_def.experimental.hash(&mut hasher);

        std::mem::discriminant(variable.state.get_value()).hash(&mut hasher);
    });

    hasher.finish()
}
