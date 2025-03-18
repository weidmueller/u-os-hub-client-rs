//! Contains constants and functions for dealing with the u-OS Data Hub subjects
use const_format::formatcp;

#[cfg(test)]
mod subjects_test;

/// Version prefix in a NATS subject.
pub const VERSION_PREFIX: &str = "v1";
/// Location prefix in a NATS subject.
pub const LOCATION_PREFIX: &str = "loc";

/// Subject where the registry publishs it's current state
pub const REGISTRY_STATE_CHANGED_EVENT_SUBJECT: &str =
    formatcp!("{VERSION_PREFIX}.{LOCATION_PREFIX}.registry.state.evt.changed");

/// Get the subject to read variables from a provider.
pub fn read_variables_query_from(provider_id: &str) -> String {
    format!("v1.loc.{provider_id}.vars.qry.read")
}

/// Get the subject to read variables from a provider.
pub fn write_variables_command_from(provider_id: &str) -> String {
    format!("v1.loc.{provider_id}.vars.cmd.write")
}

/// The registry will use this subject to notify consumers about a changed definition of the given provider.
///
/// The registry will publish the whole provider definition and not only the changes.
pub fn registry_provider_definition_changed_event(provider: String) -> String {
    format!(
        "{}.{}.{}.{}.{}",
        VERSION_PREFIX, LOCATION_PREFIX, "registry.providers", provider, "def.evt.changed"
    )
}

/// Extracts the provider name from a subject.
pub fn get_provider_name_from_subject(subject: &str) -> Option<String> {
    let parts: Vec<&str> = subject.split('.').collect();
    let provider_name_index = if subject.contains("registry") { 4 } else { 2 };
    if parts.len() >= 3 {
        let name = parts[provider_name_index].to_string();
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
    if parts.len() >= 4 && parts[2] == "registry" && parts[3] == "providers" {
        // registry subject
        4
    } else {
        // provider subject
        2
    }
}
