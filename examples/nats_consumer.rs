// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

//! This example shows how to connect to a data hub provider and read/write/observe variables.
//! using the low level data hub API.

use clap::Parser;
use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use tokio::task::JoinSet;

use u_os_hub_client::{
    authenticated_nats_con::AuthenticatedNatsConnection,
    consumer::{connected_nats_provider::ConnectedNatsProvider, nats_consumer::NatsConsumer},
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionState, ReadVariablesQueryRequestT, VariableListT, VariableT,
        VariableValueStringT, VariableValueT, WriteVariablesCommandT,
    },
};

mod utils;

/// It is recommended to use the deploy examples script to copy this example to a device and register it as a systemd service.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let conf = utils::Config::parse();

    let test_provider_id = conf
        .provider_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Provider ID is mandatory for consumers"))?;

    let auth_settings = utils::build_auth_settings_from_conf(&conf, false).await?;

    let auth_nats_con = Arc::new(
        AuthenticatedNatsConnection::new(
            format!("nats://{}:{}", &conf.nats_ip, &conf.nats_port),
            &auth_settings,
        )
        .await
        .map_err(|e| anyhow::anyhow!(format!("Failed to connect to NATS server: {e}")))?,
    );

    let mut js = JoinSet::new();

    //Monitor nats events
    let mut nats_events = auth_nats_con.get_events();
    js.spawn(async move {
        while let Ok(event) = nats_events.recv().await {
            println!("NATS event: {event:?}");
        }
    });

    //Create NatsConsumer
    let consumer = Arc::new(NatsConsumer::new(auth_nats_con).await?);

    //Monitor registry state & provider list
    let mut registry_events = consumer.subscribe_registry_state().await?;
    let mut provider_events = consumer.subscribe_provider_ids().await?;

    js.spawn(async move {
        loop {
            tokio::select! {
                Some(revent) = registry_events.next() => {
                    println!("Registry event: {revent:?}");
                }
                Some(pevent) = provider_events.next() => {
                    println!("Provider event: {pevent:?}");
                }
                else => break
            };
        }
    });

    //Get list of providers
    println!("Registered providers:");
    for prov_id in consumer
        .read_provider_ids()
        .await?
        .providers
        .items
        .unwrap_or_default()
    {
        println!("\t {prov_id:?}");
    }

    //wait for test provider to become available
    println!("Waiting for {test_provider_id:?} to become available ...");
    consumer.wait_for_provider(test_provider_id).await?;

    //Connect to test provider
    let provider_con = ConnectedNatsProvider::new(consumer, test_provider_id.to_owned()).await?;

    //Wait until the test provider has registered all its services
    //This is a special case because the test provider changes its variables after registration
    let mut provider_def_evt_stream = provider_con.subscribe_provider_definition().await?;
    if provider_con.get_all_variable_definitions().len() < 6 {
        println!("Waiting until provider has registered all its variables ...");
        while let Some(provider_def_evt) = provider_def_evt_stream.next().await {
            if let Ok(provider_def_evt) = &provider_def_evt {
                if let Some(provider_def) = &provider_def_evt.provider_definition {
                    if provider_def.state == ProviderDefinitionState::OK {
                        if let Some(var_defs) = &provider_def.variable_definitions {
                            //Well.. thats quite some nesting >:D
                            if var_defs.len() >= 6 {
                                println!("Seems like its done!");
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    //Print all variables of this provider
    println!("Variable defs of {test_provider_id}:");
    let var_ids = provider_con.get_variable_ids();
    for id in &var_ids {
        let def = provider_con.get_variable_definition(*id)?;
        println!("\t{id} -> {def:?}");
    }

    //Read all variable values of this provider
    let mut request = ReadVariablesQueryRequestT::default();
    request.ids = None;
    let var_states = provider_con.read_variables(&request).await?;
    println!("Variable state of {test_provider_id}:");
    for var_state in &var_states.variables.items.unwrap_or_default() {
        println!("\t{var_state:?}");
    }

    //Subscribe to changes of variables
    let mut variable_events = provider_con.subscribe_variables().await?;
    js.spawn(async move {
        while let Some(event) = variable_events.next().await {
            if let Ok(event) = event {
                println!("Variables have changed: {event:?}");
            }
        }
    });

    // Try to modify single variable of this provider
    let writable_string_var_id = provider_con.variable_id_from_key("folder2.writable_string")?;

    let mut new_var_value = VariableT::default();
    new_var_value.id = writable_string_var_id;

    let mut written_val = VariableValueStringT::default();
    written_val.value = Some("hello from low level consumer API!".to_owned());
    new_var_value.value = VariableValueT::String(Box::new(written_val));

    let mut variables = VariableListT::default();
    variables.provider_definition_fingerprint = provider_con
        .get_fingerprint()
        .ok_or_else(|| anyhow::anyhow!("Provider went offline while we were working"))?;
    variables.items = Some(vec![new_var_value]);

    let mut write_vars_cmd = WriteVariablesCommandT::default();
    write_vars_cmd.variables = Box::new(variables);

    //This will send the write command to the provider after checking preconditions,
    //e.g. variable ids, write permissions, data types etc.
    provider_con.write_variables(&write_vars_cmd).await?;

    //Listen for some events, then stop
    let _ignored = tokio::time::timeout(Duration::from_secs(10), js.join_all()).await;

    Ok(())
}
