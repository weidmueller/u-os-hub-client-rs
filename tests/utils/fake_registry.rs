use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_nats::client;
use futures::StreamExt;
use tokio::task;
use u_os_hub_client::{
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionChangedEvent, ProviderDefinitionState, ProviderDefinitionT,
    },
    nats_subjects,
    payload_builders::{
        build_provider_definition_changed_event, build_providers_changed_event,
        build_read_provider_definition_response, build_read_providers_response,
    },
};

use super::create_auth_con;

#[derive(Clone, Debug)]
pub struct FakeRegistryState {
    registered_providers: HashMap<String, ProviderDefinitionT>,
}

type SharedFakeRegistryState = Arc<Mutex<FakeRegistryState>>;

pub struct FakeRegistry {
    worker_task: task::JoinHandle<()>,
    state: SharedFakeRegistryState,
}

impl Drop for FakeRegistry {
    fn drop(&mut self) {
        self.worker_task.abort();
    }
}

impl FakeRegistry {
    /// Create a fake registry. Every Provider definition will be forwarded and marked as OK.
    pub async fn new() -> Self {
        let state = Arc::new(Mutex::new(FakeRegistryState {
            registered_providers: HashMap::new(),
        }));

        let auth_nats_con = create_auth_con("fake-registry").await;

        let mut def_changed_subscribtion = auth_nats_con
            .get_client()
            .subscribe(nats_subjects::provider_changed_event("*"))
            .await
            .unwrap();

        let mut read_provider_ids_sub = auth_nats_con
            .get_client()
            .subscribe(nats_subjects::registry_providers_read_query())
            .await
            .unwrap();

        let mut read_provider_def_sub = auth_nats_con
            .get_client()
            .subscribe(nats_subjects::registry_provider_definition_read_query("*"))
            .await
            .unwrap();

        let state_clone = state.clone();
        let worker_task = tokio::spawn(async move {
            let client = auth_nats_con.get_client();

            loop {
                tokio::select! {
                    Some(msg) = def_changed_subscribtion.next() => {
                        Self::handle_provider_definition_changed(msg, client, &state_clone).await.unwrap();
                    },
                    Some(msg) = read_provider_ids_sub.next() => {
                        Self::handle_read_provider_ids(msg, client, &state_clone).await.unwrap();
                    },
                    Some(msg) = read_provider_def_sub.next() => {
                        Self::handle_read_provider_def(msg, client, &state_clone).await.unwrap();
                    },
                }
            }
        });

        Self { worker_task, state }
    }

    pub fn get_state(&self) -> FakeRegistryState {
        self.state.lock().unwrap().clone()
    }

    async fn handle_provider_definition_changed(
        msg: async_nats::Message,
        client: &client::Client,
        state: &SharedFakeRegistryState,
    ) -> anyhow::Result<()> {
        let provider_id = nats_subjects::get_provider_id_from_subject(&msg.subject)?;
        let subject = nats_subjects::registry_provider_definition_changed_event(&provider_id);

        let parsed_message = flatbuffers::root::<ProviderDefinitionChangedEvent>(&msg.payload)?;

        match parsed_message.provider_definition() {
            None => {
                // Null provider definitions means that the provider needs to be removed.
                state
                    .lock()
                    .unwrap()
                    .registered_providers
                    .remove(&provider_id);

                // Publish the provider definition changed event with null payload to indicate removal
                client
                    .publish(subject, build_provider_definition_changed_event(None))
                    .await?;
            }
            Some(provider_definition) => {
                // Update the provider definition
                let mut new_provider_definition = provider_definition.unpack();
                new_provider_definition.state = ProviderDefinitionState::OK;

                state
                    .lock()
                    .unwrap()
                    .registered_providers
                    .insert(provider_id.clone(), new_provider_definition.clone());

                // Publish the provider definition changed event with the new state
                client
                    .publish(
                        subject,
                        build_provider_definition_changed_event(Some(new_provider_definition)),
                    )
                    .await?;
            }
        }

        // Publish provider definitions changed eventz
        let providers_changed_event = build_providers_changed_event(
            state
                .lock()
                .unwrap()
                .registered_providers
                .keys()
                .map(String::as_ref),
        );

        client
            .publish(
                nats_subjects::registry_providers_changed_event(),
                providers_changed_event,
            )
            .await?;

        //flush events
        client.flush().await?;

        Ok(())
    }

    async fn handle_read_provider_ids(
        msg: async_nats::Message,
        client: &client::Client,
        state: &SharedFakeRegistryState,
    ) -> anyhow::Result<()> {
        let resp = {
            let locked_state = state.lock().unwrap();
            let iter = locked_state.registered_providers.keys();
            build_read_providers_response(iter.map(String::as_ref))
        };

        client.publish(msg.reply.unwrap(), resp).await?;
        client.flush().await?;

        Ok(())
    }

    async fn handle_read_provider_def(
        msg: async_nats::Message,
        client: &client::Client,
        state: &SharedFakeRegistryState,
    ) -> anyhow::Result<()> {
        let provider_id = nats_subjects::get_provider_id_from_subject(&msg.subject)?;

        let resp = {
            let locked_state = state.lock().unwrap();
            let provider_def = locked_state.registered_providers.get(&provider_id);
            build_read_provider_definition_response(provider_def.cloned())
        };

        client.publish(msg.reply.unwrap(), resp).await?;
        client.flush().await?;

        Ok(())
    }
}
