//! Welcome to the u-OS Data Hub client library.
//!
//! This library allows to interact with the uOS Variable Data Hub as a provider or consumer.
//!
//! As a starting point, please take a look at the [provider] and [consumer] module documentation.
//! They provide example code and a detailed description of the available APIs.

// The module comment/description is located here to not change the generated files.
// Because of the same reason the "dead_code", "unused_imports" and "clippy::all" is allowed for the generated module.
#[allow(dead_code, unused_imports, clippy::all, clippy::unwrap_used, clippy::expect_used)]
#[rustfmt::skip]
/// Contains the generated flatbuffers.
pub mod generated;

pub mod authenticated_nats_con;
pub mod consumer;
pub mod env_file_parser;
pub mod nats_subjects;
pub mod oauth2;
pub mod payload_builders;
pub mod provider;
pub mod variable;
pub mod variable_definition_serde_extension;
pub mod variable_value_type_serde_extension;

/// Commonly used imports for the u-OS Data Hub client library.
pub mod prelude {
    /// Common imports for data hub consumers.
    pub mod consumer {
        pub use crate::authenticated_nats_con::{
            AuthenticatedNatsConnection, AuthenticationSettings, AuthenticationSettingsBuilder,
            NatsPermission, DEFAULT_U_OS_NATS_ADDRESS,
        };
        pub use crate::consumer::connected_dh_provider::{
            ConnectedDataHubProvider, ProviderEvent, VariableKeyLike,
        };
        pub use crate::consumer::connected_nats_provider::VariableID;
        pub use crate::consumer::dh_consumer::DataHubConsumer;
        pub use crate::consumer::dh_types::{
            ConsumerVariableDefinition, ConsumerVariableQuality, ConsumerVariableState,
            ConsumerVariableType, DhRegistryState,
        };
        pub use crate::consumer::variable_key::VariableKey;
        pub use crate::oauth2::OAuth2Credentials;
        pub use crate::variable::value::Value as ConsumerVariableValue;
        pub use crate::variable::value::{DhDuration, DhTimestamp};
    }

    /// Common imports for data hub providers.
    pub mod provider {
        pub use crate::authenticated_nats_con::{
            AuthenticatedNatsConnection, AuthenticationSettings, AuthenticationSettingsBuilder,
            NatsPermission, DEFAULT_U_OS_NATS_ADDRESS,
        };
        pub use crate::oauth2::OAuth2Credentials;
        pub use crate::provider::{Provider, ProviderOptions, VariableBuilder};
        pub use crate::variable::value::{DhDuration, DhTimestamp};
    }
}
