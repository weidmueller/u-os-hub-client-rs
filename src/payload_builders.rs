//! Contains flatbuffers payload builders.

use std::collections::BTreeMap;

use bytes::Bytes;
use flatbuffers::FlatBufferBuilder;

use crate::{
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionChangedEvent, ProviderDefinitionChangedEventArgs, ProviderDefinitionT,
        ReadVariablesQueryRequestT, ReadVariablesQueryResponseT, State, StateChangedEvent,
        StateChangedEventArgs, VariableListT, VariablesChangedEventT, WriteVariablesCommandT,
    },
    variable::{calc_variables_hash, Variable},
};

/// Builds the payload of the read variables query request
pub fn build_read_variables_query_request(ids: Option<Vec<u32>>) -> Bytes {
    let mut builder = FlatBufferBuilder::new();
    let request_content = ReadVariablesQueryRequestT { ids }.pack(&mut builder);
    builder.finish(request_content, None);
    // TODO: Use builder.collapse() to increase performance
    Bytes::from(builder.finished_data().to_vec())
}

/// Builds the payload of the write variables command
pub fn build_write_variables_command(variables: Vec<Variable>, based_on_fingerprint: u64) -> Bytes {
    let mut builder = FlatBufferBuilder::new();

    let var_list = VariableListT {
        provider_definition_fingerprint: based_on_fingerprint,
        items: Some(variables.iter().map(|v| v.into()).collect()),
        ..Default::default()
    };

    let content = WriteVariablesCommandT {
        variables: Box::new(var_list),
    }
    .pack(&mut builder);

    builder.finish(content, None);
    // TODO: Use builder.collapse() to increase performance
    Bytes::from(builder.finished_data().to_vec())
}

/// Build the payload for a state changed event message.
pub fn build_state_changed_event_payload(state: State) -> Bytes {
    let builder = &mut FlatBufferBuilder::new();
    let state_changed_event_args = StateChangedEventArgs { state };
    let state_changed_event = StateChangedEvent::create(builder, &state_changed_event_args);

    builder.finish(state_changed_event, None);

    Bytes::copy_from_slice(builder.finished_data())
}

/// Builds the payload of the provider definition changed event
pub fn build_provider_definition_changed_event(
    provider_definition: Option<ProviderDefinitionT>,
) -> Bytes {
    let mut builder = FlatBufferBuilder::new();
    let provider_definition = provider_definition.unwrap_or_default();

    let packed_provider_definition = provider_definition.pack(&mut builder);
    let changed_provider_definition_event = ProviderDefinitionChangedEvent::create(
        &mut builder,
        &ProviderDefinitionChangedEventArgs {
            provider_definition: Some(packed_provider_definition),
        },
    );

    builder.finish(changed_provider_definition_event, None);
    // TODO: Use builder.collapse() to increase performance
    Bytes::from(builder.finished_data().to_vec())
}

/// Builds the payload of the read variables query response
pub fn build_read_variables_query_response(
    msg: ReadVariablesQueryRequestT,
    variables: &BTreeMap<u32, Variable>,
) -> Bytes {
    let mut response = ReadVariablesQueryResponseT::default();

    let mut var_list_flat = VariableListT::default();

    let items: Vec<&Variable> = match msg.ids {
        Some(ids) => variables
            .iter()
            .filter(|(id, _)| ids.contains(id))
            .map(|(_, var)| var)
            .collect(),
        None => variables.iter().map(|(_, var)| var).collect(),
    };

    let items = items.into_iter().map(|x| x.into()).collect();

    var_list_flat.items = Some(items);
    var_list_flat.provider_definition_fingerprint = calc_variables_hash(variables);

    response.variables = Box::new(var_list_flat);

    let builder = &mut FlatBufferBuilder::new();
    let packed_response = response.pack(builder);

    builder.finish(packed_response, None);
    // TODO: Use builder.collapse() to increase performance
    Bytes::copy_from_slice(builder.finished_data())
}

/// Builds the payload of the variables changed event
pub fn build_variables_changed_event(variables: &BTreeMap<u32, Variable>) -> Bytes {
    let mut response = VariablesChangedEventT::default();

    let to_publish = variables.iter().map(|(_, x)| x.into()).collect();

    let var_list_flat = VariableListT {
        items: Some(to_publish),
        provider_definition_fingerprint: calc_variables_hash(variables),
        ..Default::default()
    };

    response.changed_variables = Box::new(var_list_flat);

    let builder = &mut FlatBufferBuilder::new();
    let packed_response = response.pack(builder);

    builder.finish(packed_response, None);
    // TODO: Use builder.collapse() to increase performance
    Bytes::copy_from_slice(builder.finished_data())
}
