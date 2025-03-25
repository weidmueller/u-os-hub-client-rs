//! This example shows how to connect to a data hub provider and read/write/observe variables.
//! using the high level data hub API.

use clap::Parser;
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::task::JoinSet;
use u_os_hub_client::prelude::consumer::*;

mod utils;

/// Run in dev container like this:
/// cargo run --example dh_consumer -- --client-name test-consumer
/// Note that you will need a data hub registry that is running in the devcontainer for this to work.
///
/// Run on a device like this (Replace IP and machine client credentials):
/// cargo run --example dh_consumer -- --nats-ip 192.168.1.102 --nats-port 49360 --client-name test_consumer --client-id 65f20f74-3803-48b4-9e6e-29f72a96be51 --client-secret WkegQAoS0~-g77LVzgWeGG36C-
/// Note that the nats server on the device must be reachable from outside for this to work.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    let args = utils::Args::parse();
    let auth_settings = utils::build_auth_settings_from_args(&args, false);

    let mut js = JoinSet::new();

    //Create consumer
    let dh_consumer = Arc::new(
        DataHubConsumer::connect(
            format!("nats://{}:{}", &args.nats_ip, &args.nats_port),
            &auth_settings,
        )
        .await?,
    );

    //Print list of providers
    let provider_ids = dh_consumer.read_provider_ids().await?;
    println!("Registered provider IDs:");
    for prov_id in &provider_ids {
        println!("\t{prov_id}");
    }

    //Connect to a provider
    let provider_id = "test_provider";
    println!("Trying to connect to provider {provider_id:?} ...");
    let dh_provider_con =
        Arc::new(ConnectedDataHubProvider::new(dh_consumer, provider_id, true).await?);

    //Wait until the test provider has registered all its services
    //This is a special case because the test provider changes its variables after registration
    println!("Waiting until all variables are available ...");
    dh_provider_con
        .wait_until_variable_keys_are_available(&[
            "folder1.int_counter",
            "folder1.version",
            "folder2.float_counter",
            "folder2.writable_string",
            "folder2.writable_int",
            "folder2.version",
        ])
        .await?;

    //Print all variable ids, their definition and their values
    println!("Variable overview:");
    for def in dh_provider_con.get_all_variable_definitions()? {
        let var_key = &def.key;
        let val = dh_provider_con.read_single_variable(var_key).await?;
        println!("\t{var_key}:");
        println!("\t\tDefinition: {def:?}");
        println!("\t\tValue: {val:?}");
    }

    //Print all variable states
    println!("All variable states:");
    for (id, state) in dh_provider_con
        .read_variables(Option::<&[VariableKey]>::None)
        .await?
    {
        println!("\tVariable {id}: {state:?}");
    }

    //Watch changes on some variables in the background
    let mut change_stream = dh_provider_con
        .subscribe_variables_with_filter(Some(vec![
            "folder2.writable_string",
            "folder2.writable_int",
        ]))
        .await?;

    let dh_provider_con_clone = dh_provider_con.clone();
    js.spawn(async move {
        while let Some(change) = change_stream.next().await {
            for (id, new_state) in change {
                let key = dh_provider_con_clone
                    .variable_key_from_id(id)
                    .unwrap_or_else(|_| "???".to_owned());

                println!("New value for {key}: {new_state:?}");
            }
        }
    });

    let written_var_handle1 = VariableKey::from("folder2.writable_string");

    //write single variable
    for i in 0..10 {
        dh_provider_con
            .write_single_variable(written_var_handle1, format!("Hello World! {i}"))
            .await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    //write multiple variables at once
    let var_changes = [
        (written_var_handle1, "Multi write!!!".into()),
        ("folder2.writable_int".into(), 123.into()),
    ];
    dh_provider_con.write_variables(&var_changes).await?;

    //Listen for some changes, then stop
    let _ignored = tokio::time::timeout(Duration::from_secs(10), js.join_all()).await;

    Ok(())
}
