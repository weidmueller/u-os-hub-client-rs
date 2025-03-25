//! This example shows how to provide variables to the data hub.

use clap::Parser;
use std::{sync::Arc, time::SystemTime};

use tokio::{
    select, task,
    time::{sleep, Duration},
};

use u_os_hub_client::{prelude::provider::*, variable::value::Value};

mod utils;

/// Run in dev container like this:
/// cargo run --example provide -- --client-name test-provider
/// Note that you will need a data hub registry that is running in the devcontainer for this to work.
///
/// Run on a device like this (Replace IP and machine client credentials):
/// cargo run --example provide -- --nats-ip 192.168.1.102 --nats-port 49360 --client-name test_provider --client-id ab102c8a-13b2-49db-8f0d-1389aa6f8a12 --client-secret QIk81W9UC87gHV~u7bCiRPgZkY
/// Note that the nats server on the device must be reachable from outside for this to work.
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    let args = utils::Args::parse();
    let auth_settings = utils::build_auth_settings_from_args(&args, true);

    println!("Connecting to nats server ...");
    let auth_nats_con = Arc::new(
        AuthenticatedNatsConnection::new(
            format!("nats://{}:{}", &args.nats_ip, &args.nats_port),
            &auth_settings,
        )
        .await
        .unwrap(),
    );

    println!("Registering provider ...");
    let builder = ProviderOptions::new();
    let hub_provider = builder.register(auth_nats_con).await.unwrap();

    println!("Serving variables ...");

    // The provider can be copied into different tasks
    let provider_cloned = hub_provider.clone();
    task::spawn(async move {
        example_service_1(provider_cloned).await;
    });

    example_service_2(hub_provider.clone()).await;
}

async fn example_service_1(hub_provider: Provider) {
    let dat1_builder = VariableBuilder::new(0, "folder1.int_counter").value(Value::Int(0));

    let mut data1 = dat1_builder.build().unwrap();

    let folder_version = VariableBuilder::new(1, "folder1.version")
        .value(Value::String("1.0.0".to_string()))
        .build()
        .unwrap();

    hub_provider
        .add_variables(&[data1.clone(), folder_version.clone()])
        .await
        .unwrap();

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

async fn example_service_2(hub_provider: Provider) {
    let dat1_builder = VariableBuilder::new(3, "folder2.float_counter").value(Value::Float64(0.0));

    let writable_string = VariableBuilder::new(5, "folder2.writable_string")
        .value(Value::String("Write me!".to_owned()))
        .read_write()
        .build()
        .unwrap();
    let writable_int = VariableBuilder::new(6, "folder2.writable_int")
        .value(Value::Int(0))
        .read_write()
        .build()
        .unwrap();

    let mut data1 = dat1_builder.build().unwrap();

    let folder_version = VariableBuilder::new(4, "folder2.version")
        .value("1.0.0".into())
        .build()
        .unwrap();

    hub_provider
        .add_variables(&[
            data1.clone(),
            folder_version.clone(),
            writable_string.clone(),
            writable_int.clone(),
        ])
        .await
        .unwrap();

    let mut interval = tokio::time::interval(Duration::from_secs(1));

    let mut rw_subscribtion = hub_provider
        .subscribe_to_write_command(&[writable_string.clone(), writable_int.clone()])
        .await
        .unwrap();

    let mut float_counter = 0.0;
    loop {
        select! {
            _ = interval.tick() => {
                data1.value = Value::Float64(float_counter);
                data1.last_value_change = SystemTime::now();

                float_counter += 1.23;

                hub_provider.update_variable_values(vec![data1.clone()]).await.unwrap();
            }

            Some(mut vars) = rw_subscribtion.recv() => {
                // Just accept all and update the values and timestamps
                for var in &mut vars {
                    var.last_value_change = SystemTime::now();
                }
                hub_provider.update_variable_values(vars).await.unwrap();
            }
        }
    }
}
