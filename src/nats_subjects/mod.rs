// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

//! Contains constants and functions for dealing with the u-OS Data Hub subjects
use const_format::formatcp;

#[cfg(test)]
mod subjects_test;

/// Version prefix in a NATS subject.
pub const VERSION_PREFIX: &str = "v1";
/// Location prefix in a NATS subject.
pub const LOCATION_PREFIX: &str = "loc";

/// Get the subject that reports variable value changes of a provider.
#[inline(always)]
pub fn vars_changed_event(provider_id: &str) -> String {
    format!("{VERSION_PREFIX}.{LOCATION_PREFIX}.{provider_id}.vars.evt.changed")
}

/// Get the subject to read variables from a provider.
#[inline(always)]
pub fn read_variables_query(provider_id: &str) -> String {
    format!("{VERSION_PREFIX}.{LOCATION_PREFIX}.{provider_id}.vars.qry.read")
}

/// Get the subject to read variables from a provider.
#[inline(always)]
pub fn write_variables_command(provider_id: &str) -> String {
    format!("{VERSION_PREFIX}.{LOCATION_PREFIX}.{provider_id}.vars.cmd.write")
}

/// Subject for provider definition changed events.
///
/// The provider will use this subject to notify the registry about a changed definition.
#[inline(always)]
pub fn provider_changed_event(provider_id: &str) -> String {
    format!("{VERSION_PREFIX}.{LOCATION_PREFIX}.{provider_id}.def.evt.changed")
}

/// Subject for reading the provider definition from the registry. Used by hub participants.
#[inline(always)]
pub fn registry_provider_definition_read_query(provider_id: &str) -> String {
    format!("{VERSION_PREFIX}.{LOCATION_PREFIX}.registry.providers.{provider_id}.def.qry.read",)
}

/// The registry will use this subject to notify consumers about a changed definition of the given provider.
///
/// The registry will publish the whole provider definition and not only the changes.
#[inline(always)]
pub fn registry_provider_definition_changed_event(provider_id: &str) -> String {
    format!("{VERSION_PREFIX}.{LOCATION_PREFIX}.registry.providers.{provider_id}.def.evt.changed")
}

/// Subject for reading registered provider ids from the registry.
#[inline(always)]
//Safety: formatcp uses unchecked indexing internally, so this creates a false positive for us.
#[allow(clippy::indexing_slicing)]
pub const fn registry_providers_read_query() -> &'static str {
    formatcp!("{VERSION_PREFIX}.{LOCATION_PREFIX}.registry.providers.qry.read")
}

/// Subject for notifying consumers about a changed provider id list.
#[inline(always)]
//Safety: formatcp uses unchecked indexing internally, so this creates a false positive for us.
#[allow(clippy::indexing_slicing)]
pub const fn registry_providers_changed_event() -> &'static str {
    formatcp!("{VERSION_PREFIX}.{LOCATION_PREFIX}.registry.providers.evt.changed")
}

/// Subject where the registry publishs it's current state
#[inline(always)]
//Safety: formatcp uses unchecked indexing internally, so this creates a false positive for us.
#[allow(clippy::indexing_slicing)]
pub const fn registry_state_changed_event() -> &'static str {
    formatcp!("{VERSION_PREFIX}.{LOCATION_PREFIX}.registry.state.evt.changed")
}

/// Extracts the provider name from a subject.
pub fn get_provider_name_from_subject(subject: &str) -> Option<String> {
    let parts: Vec<&str> = subject.split('.').collect();
    let provider_name_index = if subject.contains("registry") { 4 } else { 2 };
    if parts.len() >= 3 {
        let name = parts.get(provider_name_index)?.to_string();
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    } else {
        None
    }
}

/// Extract the provider id from the subject string.
pub fn get_provider_id_from_subject(subject: &str) -> anyhow::Result<String> {
    let parts: Vec<&str> = subject.split('.').collect();

    let provider_id_index = get_index_of_provider_id(&parts);

    parts
        .get(provider_id_index)
        .map_or(Err(anyhow::anyhow!("NoProviderInSubject")), |id| {
            if !id.is_empty() {
                Ok(id.to_string())
            } else {
                Err(anyhow::anyhow!("NoProviderInSubject"))
            }
        })
}

/// Returns index of the provider id in the subject.
/// if the subject is a registry subject the provider id
/// is in the 4th position; else in 2nd position.
fn get_index_of_provider_id(parts: &[&str]) -> usize {
    if parts.len() >= 4 && parts.get(2) == Some(&"registry") && parts.get(3) == Some(&"providers") {
        // registry subject
        4
    } else {
        // provider subject
        2
    }
}
