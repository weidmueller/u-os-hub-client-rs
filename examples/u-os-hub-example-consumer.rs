//! This example shows how to connect to a data hub provider and read/write/observe variables.
//! using the high level data hub API.

use clap::Parser;
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::task::JoinSet;
use u_os_hub_client::consumer::{
    connected_dh_provider::DataHubProviderConnection, dh_consumer::DataHubConsumer,
    variable_key::VariableKey,
};

mod utils;

/// It is recommended to use the deploy examples script to copy this example to a device and register it as a systemd service.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let conf = utils::Config::parse();

    let provider_id = conf
        .provider_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Provider ID is mandatory for consumers"))?;

    let auth_settings = utils::build_auth_settings_from_conf(&conf, false).await?;

    let mut js = JoinSet::new();

    //Create consumer
    let dh_consumer = Arc::new(
        DataHubConsumer::connect(
            format!("nats://{}:{}", &conf.nats_ip, &conf.nats_port),
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
    println!("Trying to connect to provider {provider_id:?} ...");
    let dh_provider_con =
        Arc::new(DataHubProviderConnection::new(dh_consumer, provider_id, true).await?);

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
