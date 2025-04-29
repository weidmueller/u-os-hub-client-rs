//! This module provides APIs and data types for interacting with the u-OS Data Hub as a consumer.
//!
//! The consumer API is split into a low- and a high-level API.
//!
//! The high-level API is designed for ease of use and provides a more user-friendly interface.
//! It is recommended to use the high-level API unless you have very specific performance requirements or need to access advanced features.
//!
//! The low-level API is designed for advanced users who want to have full control and performance, at the cost of lower usability.
//! It may change at any time without compatibility guarantees and uses raw NATS and flatbuffer data types without abstraction.
//!
//! All modules and structs with "DataHub / dh" in their name are part of the high-level API, while "Nats" indicates the low-level API.
//! Please refer to the module and struct documentation for more details on the available APIs and data types.
//!
//! The following example demonstrates how to connect to a provider and read and write variables via the high-level API:
//!
//! ```no_run
//!# use std::sync::Arc;
//!#
//!# use u_os_hub_client::{
//!#     authenticated_nats_con::{
//!#         AuthenticationSettingsBuilder, NatsPermission, DEFAULT_U_OS_NATS_ADDRESS,
//!#     },
//!#     consumer::{
//!#         connected_dh_provider::ConnectedDataHubProvider, dh_consumer::DataHubConsumer,
//!#         variable_key::VariableKey,
//!#     },
//!#     oauth2::OAuth2Credentials,
//!#     variable::value::Value,
//!# };
//!#
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     //The provider id to connect to
//!     let provider_id = "test-provider";
//!
//!     //Configure your nats server authentication
//!     let auth_settings = AuthenticationSettingsBuilder::new(NatsPermission::VariableHubReadWrite)
//!         .with_credentials(OAuth2Credentials {
//!             //NATS client name of the consumer
//!             client_name: "test-consumer".to_string(),
//!             //Obtained by the uOS Identity&Access Client GUI
//!             client_id: "<your_oauth_client_id>".to_string(),
//!             client_secret: "<your_oauth_client_secret>".to_string(),
//!         })
//!         .build();
//!
//!     //Create consumer
//!     let dh_consumer =
//!         Arc::new(DataHubConsumer::connect(DEFAULT_U_OS_NATS_ADDRESS, &auth_settings).await?);
//!
//!     //Connect to a provider
//!     println!("Trying to connect to provider {provider_id:?} ...");
//!     let dh_provider_con = ConnectedDataHubProvider::new(dh_consumer, provider_id, true).await?;
//!
//!     //Print all variable ids, their definition and their values
//!     println!("Variable overview:");
//!     for def in dh_provider_con.get_all_variable_definitions()? {
//!         let var_key = &def.key;
//!         let val = dh_provider_con.read_single_variable(var_key).await?;
//!         println!("\t{var_key}:");
//!         println!("\t\tDefinition: {def:?}");
//!         println!("\t\tValue: {val:?}");
//!     }
//!
//!     //Explicitly creating a variable key once and reusing it
//!     //multiple times improves performance
//!     let written_var_handle1 = VariableKey::from("folder2.writable_string");
//!
//!     //write multiple variables at once
//!     let var_changes = [
//!         (written_var_handle1, Value::from("Multi write!!!")),
//!         //DataHub types usually implement the From trait,
//!         //so you can use .into() for values and keys
//!         ("folder2.writable_int".into(), 123.into()),
//!     ];
//!     dh_provider_con.write_variables(&var_changes).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod connected_dh_provider;
pub mod dh_consumer;
pub mod dh_types;
pub mod variable_key;

#[cfg(feature = "export-low-level-api")]
pub mod connected_nats_provider;
#[cfg(not(feature = "export-low-level-api"))]
mod connected_nats_provider;
#[cfg(feature = "export-low-level-api")]
pub mod nats_consumer;
#[cfg(not(feature = "export-low-level-api"))]
mod nats_consumer;
