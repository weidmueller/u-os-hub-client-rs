use std::time::Duration;

use flatbuffers::{FlatBufferBuilder, WIPOffset};
use futures::StreamExt;
use tokio::time::timeout;
use u_os_hub_client::{
    authenticated_nats_con::AuthenticatedNatsConnection,
    dh_types::{self, TimestampValue},
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionChangedEvent, ProviderDefinitionState, ProviderDefinitionT,
        ReadVariablesQueryResponse, ReadVariablesQueryResponseArgs, TimestampT, Variable,
        VariableAccessType, VariableArgs, VariableDataType, VariableDefinitionT, VariableList,
        VariableListArgs, VariableQuality, VariableValue, VariableValueInt64,
        VariableValueInt64Args, VariableValueT, VariablesChangedEvent, VariablesChangedEventArgs,
    },
    nats_subjects,
    payload_builders::build_provider_definition_changed_event,
};

use crate::utils::create_auth_con;

pub const PROVIDER_ID: &str = "incompatible_provider";
pub const VARIABLE_UPDATE_RATE: Duration = Duration::from_millis(200);

/// Dummy value that is used to simulate an incompatible enum value.
pub const INCOMPATIBLE_ENUM_VALUE: u8 = 100;

/// The variable IDs of the incompatible provider.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum VariableIDs {
    //Must start with 0 and equal indices in variable list vector
    Valid = 0,
    InvalidDataType = 1,
    InvalidAccessType = 2,
    InvalidQuality = 3,
    InvalidValueType = 4,
}

#[derive(Debug, Clone)]
enum RawVariableValue {
    ///Insert known value type
    KnownValueType(VariableValueT),
    ///Insert unknown value type and use filler dummy data
    IncompatibleValueType(VariableValue),
}

/// A special provider that publishes flatbuffer structs with invalid enum and union values.
///
/// Used to simulate a newer provider/hub version that is incompatible with the current consumer implementation.
pub struct IncompatibleProvider {
    worker_task: tokio::task::JoinHandle<()>,
}

impl Drop for IncompatibleProvider {
    fn drop(&mut self) {
        self.worker_task.abort();
    }
}

impl IncompatibleProvider {
    pub async fn new() -> anyhow::Result<Self> {
        Self::new_with_delay(Duration::ZERO).await
    }

    pub async fn new_with_delay(registration_delay: Duration) -> anyhow::Result<Self> {
        //Create connection for dummy provider
        let auth_nats_con = create_auth_con(PROVIDER_ID).await;

        //Build incompatible providerDef
        let fb_provider_def = Self::build_definition();
        let my_fingerprint = fb_provider_def.fingerprint;

        //Create subscription for variable changes
        let mut read_var_request_sub = auth_nats_con
            .get_client()
            .subscribe(nats_subjects::read_variables_query(PROVIDER_ID))
            .await?;

        //start worker thread
        let worker_task = tokio::spawn(async move {
            if !registration_delay.is_zero() {
                tokio::time::sleep(registration_delay).await;
            }

            //Register
            Self::update_definition(&auth_nats_con, fb_provider_def)
                .await
                .unwrap();

            let mut fb_var_list = Self::init_variable_list_raw(
                my_fingerprint,
                TimestampValue::now(),
                &dh_types::VariableValue::Int(1234),
            );

            let mut var_write_timer = tokio::time::interval(VARIABLE_UPDATE_RATE);
            let mut cur_counter = 0;

            var_write_timer.tick().await; //skip first tick

            loop {
                tokio::select! {
                    //wait for timer
                    _ = var_write_timer.tick() => {
                        cur_counter += 1;

                        fb_var_list = Self::init_variable_list_raw(
                            my_fingerprint,
                            TimestampValue::now(),
                            &dh_types::VariableValue::Int(cur_counter),
                        );

                        Self::publish_variable_values(&auth_nats_con, fb_var_list.clone()).await.unwrap();
                    },
                    //handle variable read requests
                    Some(msg) = read_var_request_sub.next() => {
                        Self::handle_variable_read_query(&auth_nats_con, msg, fb_var_list.clone()).await.unwrap();
                    }
                }
            }
        });

        Ok(Self { worker_task })
    }

    fn build_definition() -> ProviderDefinitionT {
        let mut fb_var_valid = VariableDefinitionT::default();
        fb_var_valid.id = VariableIDs::Valid as u32;
        fb_var_valid.key = "valid".to_string();
        fb_var_valid.data_type = VariableDataType::INT64;
        fb_var_valid.access_type = VariableAccessType::READ_WRITE;

        let mut fb_var_incomp_dt = VariableDefinitionT::default();
        fb_var_incomp_dt.id = VariableIDs::InvalidDataType as u32;
        fb_var_incomp_dt.key = "incompatible_data_type".to_string();
        fb_var_incomp_dt.data_type = VariableDataType(INCOMPATIBLE_ENUM_VALUE as i8);
        fb_var_incomp_dt.access_type = VariableAccessType::READ_WRITE;

        let mut fb_var_incomp_at = VariableDefinitionT::default();
        fb_var_incomp_at.id = VariableIDs::InvalidAccessType as u32;
        fb_var_incomp_at.key = "incompatible_access_type".to_string();
        fb_var_incomp_at.data_type = VariableDataType::INT64;
        fb_var_incomp_at.access_type = VariableAccessType(INCOMPATIBLE_ENUM_VALUE as i8);

        let mut fb_var_incomp_quality = VariableDefinitionT::default();
        fb_var_incomp_quality.id = VariableIDs::InvalidQuality as u32;
        fb_var_incomp_quality.key = "incompatible_quality".to_string();
        fb_var_incomp_quality.data_type = VariableDataType::INT64;
        fb_var_incomp_quality.access_type = VariableAccessType::READ_WRITE;

        let mut fb_var_incomp_value = VariableDefinitionT::default();
        fb_var_incomp_value.id = VariableIDs::InvalidValueType as u32;
        fb_var_incomp_value.key = "incompatible_value".to_string();
        fb_var_incomp_value.data_type = VariableDataType::INT64;
        fb_var_incomp_value.access_type = VariableAccessType::READ_WRITE;

        //Build incompatible providerDef
        let mut provider_def = ProviderDefinitionT::default();
        provider_def.fingerprint = 1;
        provider_def.variable_definitions = Some(vec![
            fb_var_valid,
            fb_var_incomp_dt,
            fb_var_incomp_at,
            fb_var_incomp_quality,
            fb_var_incomp_value,
        ]);
        provider_def.state = ProviderDefinitionState::UNSPECIFIED;

        provider_def
    }

    /// Creates a variable with the given ID, quality and value using the low level flatbuffer builder.
    ///
    /// Lifetime bounds are needed here to ensure that the mut builder is borrowed shorter than the immutable builder.
    /// See ::create methods from flatbuffers for more details.
    fn create_variable_raw<'bldr: 'mut_bldr, 'mut_bldr>(
        builder: &'mut_bldr mut FlatBufferBuilder<'bldr>,
        id: VariableIDs,
        quality: VariableQuality,
        value: RawVariableValue,
    ) -> WIPOffset<Variable<'bldr>> {
        let var_offset = match value {
            RawVariableValue::KnownValueType(value) => {
                let value_data = value.pack(builder);

                Variable::create(
                    builder,
                    &VariableArgs {
                        id: id as u32,
                        quality,
                        timestamp: None,
                        value_type: value.variable_value_type(),
                        value: value_data,
                    },
                )
            }
            RawVariableValue::IncompatibleValueType(value_type) => {
                // Create some dummy data for the union's value part.
                // Even if the type is unknown, the offset must be valid.
                let value_data =
                    VariableValueInt64::create(builder, &VariableValueInt64Args { value: 10 });

                Variable::create(
                    builder,
                    &VariableArgs {
                        id: id as u32,
                        quality,
                        timestamp: None,
                        value_type,
                        value: Some(value_data.as_union_value()),
                    },
                )
            }
        };

        var_offset
    }

    /// We must init the variable values using the low level flatbuffer builder,
    /// as this allows us to set incompatible union values.
    ///
    /// The generated code does not allow us to set the union value directly.
    fn init_variable_list_raw<'a>(
        fingerprint: u64,
        base_timestamp: TimestampValue,
        used_valid_value: &dh_types::VariableValue,
    ) -> (FlatBufferBuilder<'a>, WIPOffset<VariableList<'a>>) {
        let mut builder = FlatBufferBuilder::new();

        let cur_time = TimestampT::from(base_timestamp);
        let valid_value = RawVariableValue::KnownValueType(used_valid_value.into());

        let variable_offsets = &[
            Self::create_variable_raw(
                &mut builder,
                VariableIDs::Valid,
                VariableQuality::GOOD,
                valid_value.clone(),
            ),
            Self::create_variable_raw(
                &mut builder,
                VariableIDs::InvalidDataType,
                VariableQuality::GOOD,
                valid_value.clone(),
            ),
            Self::create_variable_raw(
                &mut builder,
                VariableIDs::InvalidAccessType,
                VariableQuality::GOOD,
                valid_value.clone(),
            ),
            Self::create_variable_raw(
                &mut builder,
                VariableIDs::InvalidQuality,
                VariableQuality(INCOMPATIBLE_ENUM_VALUE),
                valid_value.clone(),
            ),
            Self::create_variable_raw(
                &mut builder,
                VariableIDs::InvalidValueType,
                VariableQuality::GOOD,
                RawVariableValue::IncompatibleValueType(VariableValue(INCOMPATIBLE_ENUM_VALUE)),
            ),
        ];

        let vec_offset = builder.create_vector(variable_offsets);

        let var_list_offset: WIPOffset<VariableList<'_>> = VariableList::create(
            &mut builder,
            &VariableListArgs {
                provider_definition_fingerprint: fingerprint,
                base_timestamp: Some(&cur_time.pack()),
                items: Some(vec_offset),
            },
        );

        (builder, var_list_offset)
    }

    async fn update_definition(
        nats_con: &AuthenticatedNatsConnection,
        provider_definition: ProviderDefinitionT,
    ) -> anyhow::Result<()> {
        let mut registry_provider_definition_updated_subscription = nats_con
            .get_client()
            .subscribe(nats_subjects::registry_provider_definition_changed_event(
                PROVIDER_ID,
            ))
            .await?;

        let provider_def_payload =
            build_provider_definition_changed_event(Some(provider_definition));

        nats_con
            .get_client()
            .publish(
                nats_subjects::provider_changed_event(PROVIDER_ID),
                provider_def_payload,
            )
            .await?;

        let msg = timeout(
            Duration::from_secs(2),
            registry_provider_definition_updated_subscription.next(),
        )
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!("Registry Provider definition changed event stream stopped.")
        })?;

        if let Ok(parsed_message) =
            flatbuffers::root::<ProviderDefinitionChangedEvent>(&msg.payload)
        {
            if let Some(def) = parsed_message.provider_definition() {
                if def.state() == ProviderDefinitionState::OK {
                    return Ok(());
                }

                return Err(anyhow::anyhow!(
                    "The registry marked the definition as invalid"
                ));
            }
            Err(anyhow::anyhow!(
                "Provider definition changed event did not contain provider definition"
            ))
        } else {
            Err(anyhow::anyhow!(
                "Could not parse provider definition changed event"
            ))
        }
    }

    async fn publish_variable_values<'a>(
        nats_con: &AuthenticatedNatsConnection,
        var_list: (FlatBufferBuilder<'a>, WIPOffset<VariableList<'a>>),
    ) -> anyhow::Result<()> {
        let (mut builder, var_list_offset) = var_list;
        let final_offset = VariablesChangedEvent::create(
            &mut builder,
            &VariablesChangedEventArgs {
                changed_variables: Some(var_list_offset),
            },
        );
        builder.finish(final_offset, None);
        let payload_bytes = builder.finished_data().to_vec();

        nats_con
            .get_client()
            .publish(
                nats_subjects::vars_changed_event(PROVIDER_ID),
                payload_bytes.into(),
            )
            .await?;

        Ok(())
    }

    async fn handle_variable_read_query<'a>(
        nats_con: &AuthenticatedNatsConnection,
        msg: async_nats::Message,
        var_list: (FlatBufferBuilder<'a>, WIPOffset<VariableList<'a>>),
    ) -> anyhow::Result<()> {
        let reply_subject = msg.reply.ok_or(anyhow::anyhow!(
            "Read variables query request did not contain a reply subject"
        ))?;

        let (mut builder, var_list_offset) = var_list;
        let final_offset = ReadVariablesQueryResponse::create(
            &mut builder,
            &ReadVariablesQueryResponseArgs {
                variables: Some(var_list_offset),
            },
        );
        builder.finish(final_offset, None);
        let payload_bytes = builder.finished_data().to_vec();

        nats_con
            .get_client()
            .publish(reply_subject.into_string(), payload_bytes.into())
            .await?;

        Ok(())
    }
}
