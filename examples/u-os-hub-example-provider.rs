//! This example shows how to provide variables to the data hub.

use clap::Parser;
use std::{sync::Arc, time::SystemTime};

use tokio::{
    select, task,
    time::{sleep, Duration},
};

use u_os_hub_client::{prelude::provider::*, variable::value::Value};

mod utils;

/// It is recommended to use the deploy examples script to copy this example to a device and register it as a systemd service.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let conf = utils::Config::parse();
    let auth_settings = utils::build_auth_settings_from_conf(&conf, true)?;

    println!("Connecting to nats server ...");
    let auth_nats_con = Arc::new(
        AuthenticatedNatsConnection::new(
            format!("nats://{}:{}", &conf.nats_ip, &conf.nats_port),
            &auth_settings,
        )
        .await
        .unwrap(),
    );

    println!("Registering provider ...");
    let builder = ProviderOptions::new();
    let hub_provider = builder.register(auth_nats_con).await?;

    println!("Serving variables ...");

    // The provider can be copied into different tasks
    let provider_cloned = hub_provider.clone();
    task::spawn(async move {
        example_service_1(provider_cloned).await.unwrap();
    });

    example_service_2(hub_provider.clone()).await?;

    Ok(())
}

async fn example_service_1(hub_provider: Provider) -> anyhow::Result<()> {
    let dat1_builder = VariableBuilder::new(0, "folder1.int_counter").value(Value::Int(0));

    let mut data1 = dat1_builder.build()?;

    let folder_version = VariableBuilder::new(1, "folder1.version")
        .value(Value::String("1.0.0".to_string()))
        .build()?;

    hub_provider
        .add_variables(&[data1.clone(), folder_version.clone()])
        .await?;

    let mut counter = 0;
    loop {
        data1.value = Value::Int(counter);
        data1.last_value_change = SystemTime::now();
        counter += 1;

        hub_provider
            .update_variable_values(vec![data1.clone()])
            .await
            .ok();
        sleep(Duration::from_secs(1)).await;
    }
}

async fn example_service_2(hub_provider: Provider) -> anyhow::Result<()> {
    let dat1_builder = VariableBuilder::new(3, "folder2.float_counter").value(Value::Float64(0.0));

    let writable_string = VariableBuilder::new(5, "folder2.writable_string")
        .value(Value::String("Write me!".to_owned()))
        .read_write()
        .build()?;
    let writable_int = VariableBuilder::new(6, "folder2.writable_int")
        .value(Value::Int(0))
        .read_write()
        .build()?;

    let mut data1 = dat1_builder.build()?;

    let folder_version = VariableBuilder::new(4, "folder2.version")
        .value("1.0.0".into())
        .build()?;

    hub_provider
        .add_variables(&[
            data1.clone(),
            folder_version.clone(),
            writable_string.clone(),
            writable_int.clone(),
        ])
        .await?;

    let mut interval = tokio::time::interval(Duration::from_secs(1));

    let mut rw_subscribtion = hub_provider
        .subscribe_to_write_command(&[writable_string.clone(), writable_int.clone()])
        .await?;

    let mut float_counter = 0.0;
    loop {
        select! {
            _ = interval.tick() => {
                data1.value = Value::Float64(float_counter);
                data1.last_value_change = SystemTime::now();

                float_counter += 1.23;

                hub_provider.update_variable_values(vec![data1.clone()]).await?;
            }

            Some(mut vars) = rw_subscribtion.recv() => {
                // Just accept all and update the values and timestamps
                for var in &mut vars {
                    var.last_value_change = SystemTime::now();
                }
                hub_provider.update_variable_values(vars).await?;
            }
        }
    }
}
