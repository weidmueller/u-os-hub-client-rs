//! This example shows how to provide variables to the data hub.

use clap::Parser;
use std::{collections::HashMap, time::Duration};
use tracing::error;

use tokio::{select, task, time::sleep};

use u_os_hub_client::{
    dh_types::{DurationValue, TimestampValue, VariableQuality},
    provider::{Provider, ProviderBuilder, VariableBuilder},
};

mod utils;

/// It is recommended to use the deploy examples script to copy this example to a device and register it as a systemd service.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let conf = utils::Config::parse();
    let auth_settings = utils::build_auth_settings_from_conf(&conf, true).await?;

    println!("Connecting to nats server & registering provider ...");
    let builder = ProviderBuilder::new();
    let hub_provider = builder
        .register(
            format!("nats://{}:{}", &conf.nats_ip, &conf.nats_port),
            &auth_settings,
        )
        .await?;

    println!("Serving variables ...");

    // The provider can be copied into different tasks
    let provider_cloned = hub_provider.clone();
    task::spawn(async move {
        if let Err(e) = example_service_1(provider_cloned).await {
            error!("Error in example_service_1: {e}");
        }
    });

    example_service_2(hub_provider.clone()).await?;

    Ok(())
}

async fn example_service_1(hub_provider: Provider) -> anyhow::Result<()> {
    let dat1_builder = VariableBuilder::new(0, "folder1.int_counter").initial_value(0);

    let mut data1 = dat1_builder.build()?;

    let folder_version = VariableBuilder::new(1, "folder1.version")
        .initial_value("1.0.0")
        .build()?;

    hub_provider
        .add_variables(vec![data1.clone(), folder_version])
        .await?;

    let mut counter = 0;
    loop {
        let data1_state = data1.get_mut_state();
        data1_state.set_value(counter);
        counter += 1;

        hub_provider
            .update_variable_states(vec![data1_state.clone()])
            .await
            .ok();
        sleep(Duration::from_secs(1)).await;
    }
}

async fn example_service_2(hub_provider: Provider) -> anyhow::Result<()> {
    let dat1_builder = VariableBuilder::new(3, "folder2.float_counter").initial_value(0.0);

    //Make sure that there is one writable variable for each type so we can test read/write of all types
    //Make some experimental
    let writable_vars = vec![
        VariableBuilder::new(4, "folder2.writable_string")
            .initial_value("Write me!")
            .read_write()
            .build()?,
        VariableBuilder::new(5, "folder2.writable_int")
            .initial_value(1337)
            .read_write()
            .build()?,
        VariableBuilder::new(6, "folder2.writable_bool")
            .initial_value(true)
            .read_write()
            .build()?,
        VariableBuilder::new(7, "folder2.writable_float")
            .initial_value(1122.3344)
            .read_write()
            .build()?,
        VariableBuilder::new(8, "folder2.writable_timestamp")
            .initial_value(TimestampValue::now())
            .read_write()
            .build()?,
        VariableBuilder::new(9, "folder2.writable_duration")
            .initial_value(DurationValue::new(123, 456))
            .read_write()
            .build()?,
        VariableBuilder::new(10, "folder2.experimental_string")
            .initial_value("experimental_value")
            .initial_quality(VariableQuality::Uncertain)
            .initial_timestamp(None)
            .experimental()
            .read_write()
            .build()?,
    ];

    let mut data1 = dat1_builder.build()?;

    let folder_version = VariableBuilder::new(11, "folder2.version")
        .initial_value("1.0.0")
        .build()?;

    let mut all_vars = vec![data1.clone(), folder_version];
    all_vars.append(&mut writable_vars.clone());
    hub_provider.add_variables(all_vars).await?;

    let mut interval = tokio::time::interval(Duration::from_secs(1));

    let mut write_command_sub = hub_provider
        .subscribe_to_write_command(writable_vars.clone())
        .await?;

    //Convert to hashmap for faster ID lookup
    let mut writable_vars = writable_vars
        .into_iter()
        .map(|v| (v.get_definition().id, v))
        .collect::<HashMap<_, _>>();

    let mut float_counter = 0.0;
    loop {
        select! {
            //Update the value of our read-only variable periodically
            _ = interval.tick() => {
                let data1_state = data1.get_mut_state();
                data1_state.set_value(float_counter);

                float_counter += 1.23;

                // Publish the updated value to the Data Hub
                if let Err(e) = hub_provider.update_variable_states(vec![data1_state.clone()]).await {
                    eprintln!("Error updating variable states: {e}");
                }
            }
            //React to write commands of consumers
            Some(write_commands) = write_command_sub.recv() => {
                // The logic here is implementation defined
                // In this example, we simply accept all write commands and update the states
                let mut updated_states = Vec::with_capacity(write_commands.len());

                for write_cmd in write_commands {
                    let written_var = writable_vars.get_mut(&write_cmd.id);

                    if let Some(written_var) = written_var {
                        // Update the variable state with the new value. This will automatically update the timestamp
                        let written_var_state = written_var.get_mut_state();
                        written_var_state.set_value(write_cmd.value);
                        updated_states.push(written_var_state.clone());
                    }
                    else {
                        eprintln!("Received write command for unknown variable ID: {}", write_cmd.id);
                    }
                }

                // Publish the updated states to the Data Hub
                if let Err(e) = hub_provider.update_variable_states(updated_states).await {
                    eprintln!("Error updating variable states: {e}");
                }
            }
        }
    }
}
