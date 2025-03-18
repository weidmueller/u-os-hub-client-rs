//! Contains flatbuffers payload builders.

use std::collections::BTreeMap;

use bytes::Bytes;
use flatbuffers::FlatBufferBuilder;

use crate::{
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionChangedEventT, ProviderDefinitionT, ProviderListT, ProviderT,
        ProvidersChangedEventT, ReadProviderDefinitionQueryResponseT, ReadProvidersQueryResponseT,
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
    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
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
    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
}

/// Build the payload for a state changed event message.
pub fn build_state_changed_event_payload(state: State) -> Bytes {
    let mut builder = FlatBufferBuilder::new();
    let state_changed_event_args = StateChangedEventArgs { state };
    let state_changed_event = StateChangedEvent::create(&mut builder, &state_changed_event_args);

    builder.finish(state_changed_event, None);
    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
}

/// Builds the payload of the provider definition changed event
pub fn build_provider_definition_changed_event(
    provider_definition: Option<ProviderDefinitionT>,
) -> Bytes {
    let event = ProviderDefinitionChangedEventT {
        provider_definition: provider_definition.map(Box::new),
    };

    let mut builder = FlatBufferBuilder::new();
    let packed_response = event.pack(&mut builder);
    builder.finish(packed_response, None);

    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
}

/// Builds the payload of the read provider ids query response
pub fn build_provider_ids_response<'a, I>(provider_ids_iter: I) -> Bytes
where
    I: Iterator<Item = &'a String>,
{
    let provider_list = ProviderListT {
        items: Some(
            provider_ids_iter
                .map(|id| ProviderT { id: id.clone() })
                .collect(),
        ),
    };

    let resp = ReadProvidersQueryResponseT {
        providers: Box::new(provider_list),
    };

    let mut builder = FlatBufferBuilder::new();
    let packed_response = resp.pack(&mut builder);
    builder.finish(packed_response, None);

    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
}

/// Builds the payload of the provider definition changed event
pub fn build_read_provider_definition_response(
    provider_definition: Option<ProviderDefinitionT>,
) -> Bytes {
    let event = ReadProviderDefinitionQueryResponseT {
        provider_definition: provider_definition.map(Box::new),
    };

    let mut builder = FlatBufferBuilder::new();
    let packed_response = event.pack(&mut builder);
    builder.finish(packed_response, None);

    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
}

/// Builds the payload of the providers changed event
pub fn build_providers_changed_event<'a, I>(provider_ids_iter: I) -> Bytes
where
    I: Iterator<Item = &'a String>,
{
    let provider_list = ProviderListT {
        items: Some(
            provider_ids_iter
                .map(|id| ProviderT { id: id.clone() })
                .collect(),
        ),
    };

    let resp = ProvidersChangedEventT {
        providers: Box::new(provider_list),
    };

    let mut builder = FlatBufferBuilder::new();
    let packed_response = resp.pack(&mut builder);
    builder.finish(packed_response, None);

    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
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

    let mut builder = FlatBufferBuilder::new();
    let packed_response = response.pack(&mut builder);

    builder.finish(packed_response, None);
    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
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

    let mut builder = FlatBufferBuilder::new();
    let packed_response = response.pack(&mut builder);

    builder.finish(packed_response, None);
    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
}
