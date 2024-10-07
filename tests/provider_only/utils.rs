use async_nats::Client;
use futures::StreamExt;
use tokio::task::JoinHandle;
use uc_hub_client::{
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionChangedEvent, ProviderDefinitionState,
    },
    payload_builders::build_provider_definition_changed_event,
    subjects::registry_provider_definition_changed_event,
};

/// Create a fake registry. Every Provider definition will be forwareded and marked as OK.
pub fn create_fake_registry(client: Client, provider_id: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut def_changed_subscribtion = client
            .subscribe(format!("v1.loc.{}.def.evt.changed", provider_id.clone()))
            .await
            .unwrap();

        loop {
            if let Some(msg) = def_changed_subscribtion.next().await {
                let parsed_message =
                    flatbuffers::root::<ProviderDefinitionChangedEvent>(&msg.payload)
                        .expect("should parse provider definition changed event");

                match parsed_message.provider_definition() {
                    None => {
                        // Not provider definitions means, that the provider need to be removed.
                    }
                    Some(provider_definition) => {
                        // Update the provider definition
                        let mut new_provider_definition = provider_definition.unpack();
                        new_provider_definition.state = ProviderDefinitionState::OK;

                        client
                            .publish(
                                registry_provider_definition_changed_event(provider_id.clone()),
                                build_provider_definition_changed_event(Some(
                                    new_provider_definition,
                                )),
                            )
                            .await
                            .expect("should republish provider definition");
                    }
                }
            }
        }
    })
}
