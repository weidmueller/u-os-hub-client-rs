//! Contains flatbuffers payload builders.

use std::collections::BTreeMap;

use bytes::Bytes;
use flatbuffers::FlatBufferBuilder;

use crate::{
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionChangedEventT, ProviderDefinitionT, ProviderListT, ProviderT,
        ProvidersChangedEventT, ReadProviderDefinitionQueryResponseT, ReadProvidersQueryResponseT,
        ReadVariablesQueryRequestT, ReadVariablesQueryResponseT, State, StateChangedEvent,
        StateChangedEventArgs, VariableListT, VariableQuality, VariableT, VariableValueT,
        VariablesChangedEventT, WriteVariablesCommandT,
    },
    variable::{calc_variables_hash, Variable},
};

#[derive(Clone, Debug)]
pub struct VariableUpdate {
    pub id: u32,
    pub value: VariableValueT,
}

impl From<Variable> for VariableUpdate {
    fn from(var: Variable) -> Self {
        VariableUpdate {
            id: var.id,
            value: VariableValueT::from(&var.value),
        }
    }
}

impl From<VariableUpdate> for VariableT {
    fn from(variable_update: VariableUpdate) -> Self {
        VariableT {
            id: variable_update.id,
            value: variable_update.value,
            //Write defaults/zeroes for quality and timestamp
            quality: VariableQuality::BAD,
            timestamp: None,
        }
    }
}

/// Builds the payload of the read variables query request
pub fn build_read_variables_query_request(ids: Option<Vec<u32>>) -> Bytes {
    let mut builder = FlatBufferBuilder::new();
    let request_content = ReadVariablesQueryRequestT { ids }.pack(&mut builder);
    builder.finish(request_content, None);
    let (all_bytes, data_start_offset) = builder.collapse();
    Bytes::from(all_bytes).slice(data_start_offset..)
}

/// Builds the payload of the write variables command
pub fn build_write_variables_command(
    variables: Vec<VariableUpdate>,
    based_on_fingerprint: u64,
) -> Bytes {
    let mut builder = FlatBufferBuilder::new();

    let var_list = VariableListT {
        provider_definition_fingerprint: based_on_fingerprint,
        items: Some(variables.into_iter().map(|v| v.into()).collect()),
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
pub fn build_read_providers_response<'a, I>(provider_ids_iter: I) -> Bytes
where
    I: Iterator<Item = &'a str>,
{
    let provider_list = ProviderListT {
        items: Some(
            provider_ids_iter
                .map(|id| ProviderT { id: id.to_owned() })
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
    I: Iterator<Item = &'a str>,
{
    let provider_list = ProviderListT {
        items: Some(
            provider_ids_iter
                .map(|id| ProviderT { id: id.to_owned() })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::weidmueller::ucontrol::hub::*;
    use rstest::rstest;

    fn valid_provider_definition_with_variables() -> ProviderDefinitionT {
        ProviderDefinitionT {
            fingerprint: 1,
            variable_definitions: Some(vec![
                VariableDefinitionT {
                    key: "var_boolean".to_string(),
                    id: 1,
                    data_type: VariableDataType::BOOLEAN,
                    access_type: VariableAccessType::READ_ONLY,
                    ..VariableDefinitionT::default()
                },
                VariableDefinitionT {
                    key: "var_string".to_string(),
                    id: 2,
                    data_type: VariableDataType::STRING,
                    access_type: VariableAccessType::READ_ONLY,
                    ..VariableDefinitionT::default()
                },
                VariableDefinitionT {
                    key: "var_int64".to_string(),
                    id: 3,
                    data_type: VariableDataType::INT64,
                    access_type: VariableAccessType::READ_WRITE,
                    ..VariableDefinitionT::default()
                },
            ]),
            state: ProviderDefinitionState::OK,
            ..ProviderDefinitionT::default()
        }
    }

    #[rstest]
    #[case(Some(valid_provider_definition_with_variables()))]
    fn test_build_provider_definition_changed_payload(
        #[case] definition: Option<ProviderDefinitionT>,
    ) {
        // arrange
        // act
        let payload = build_provider_definition_changed_event(definition.clone());

        // assert
        let result: ProviderDefinitionChangedEventT =
            root_as_provider_definition_changed_event(&payload)
                .unwrap()
                .unpack();

        match definition {
            Some(def) => {
                assert_eq!(result.provider_definition.unwrap().as_ref().clone(), def);
            }
            None => {
                assert!(result.provider_definition.is_none());
            }
        }
    }

    #[rstest]
    #[case(&[])]
    #[case(&["Provider 1"])]
    #[case(&["Provider 1", "Pröväidêr_2"])]
    fn test_build_providers_changed_event_payload(#[case] provider_ids: &[&str]) {
        // arrange
        // act

        let payload = build_providers_changed_event(provider_ids.iter().copied());

        // assert
        let result_providers = root_as_read_providers_query_response(&payload)
            .unwrap()
            .providers()
            .items()
            .unwrap();
        assert_eq!(result_providers.len(), provider_ids.len());
        for (pos, id) in provider_ids.iter().enumerate() {
            assert_eq!(&result_providers.get(pos).id(), id);
        }
    }

    #[rstest]
    #[case::valid_provider_definition_without_nodes(ProviderDefinitionT::default())]
    #[case::valid_provider_definition_with_variables(valid_provider_definition_with_variables())]
    fn test_build_read_provider_definition_response_payload(
        #[case] provider_definition: ProviderDefinitionT,
    ) {
        // act
        let payload = build_read_provider_definition_response(Some(provider_definition.clone()));

        // assert
        let result_provider_definition = root_as_read_provider_definition_query_response(&payload)
            .unwrap()
            .unpack()
            .provider_definition
            .unwrap();
        assert_eq!(
            result_provider_definition.as_ref().clone(),
            provider_definition
        );
    }

    #[rstest]
    #[case(&[])]
    #[case(&["Provider 1"])]
    #[case(&["Provider 1", "Pröväidêr_2"])]
    fn test_build_read_providers_query_response_payload(#[case] provider_ids: &[&str]) {
        // arrange
        // act
        let payload = build_read_providers_response(provider_ids.iter().copied());

        // assert
        let result_providers = root_as_read_providers_query_response(&payload)
            .unwrap()
            .providers()
            .items()
            .unwrap();
        assert_eq!(result_providers.len(), provider_ids.len());
        for (pos, id) in provider_ids.iter().enumerate() {
            assert_eq!(&result_providers.get(pos).id(), id);
        }
    }

    #[rstest]
    #[case::running(State::RUNNING)]
    #[case::stopping(State::STOPPING)]
    fn test_build_state_changed_payload(#[case] state_input: State) {
        let payload = build_state_changed_event_payload(state_input);

        root_as_state_changed_event(&payload).unwrap();
    }
}
