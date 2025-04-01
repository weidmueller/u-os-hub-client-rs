//! This is the internal u-OS Data Hub client library.
//! The focus of this library is high performance.
//!
//! This library primary contains:
//!
//! - Provider Handling
//! - Consumer Handling
//! - The generated flatbuffers of the Variable-NATS-API
//!

// The module comment/description is located here to not change the generated files.
// Because of the same reason the "dead_code", "unused_imports" and "clippy::all" is allowed for the generated module.
#[allow(dead_code, unused_imports, clippy::all)]
#[rustfmt::skip]
/// Contains the generated flatbuffers.
pub mod generated;
pub mod authenticated_nats_con;
pub mod consumer;
pub mod env_file_parser;
pub mod oauth2;
pub mod payload_builders;
pub mod provider;
pub mod subjects;
pub mod variable;
pub mod variable_definition_serde_extension;
pub mod variable_value_type_serde_extension;

/// Commonly used imports for the u-OS Data Hub client library.
pub mod prelude {
    /// Common imports for data hub consumers.
    pub mod consumer {
        pub use crate::authenticated_nats_con::AuthenticatedNatsConnection;
        pub use crate::consumer::connected_dh_provider::{
            ConnectedDataHubProvider, ProviderEvent, VariableKeyLike,
        };
        pub use crate::consumer::connected_nats_provider::{VariableID, VariableKey};
        pub use crate::consumer::dh_consumer::DataHubConsumer;
        pub use crate::consumer::dh_types::{
            ConsumerVariableDefinition, ConsumerVariableQuality, ConsumerVariableState,
            ConsumerVariableType, DhRegistryState,
        };
    }

    /// Common imports for data hub providers.
    pub mod provider {
        pub use crate::authenticated_nats_con::AuthenticatedNatsConnection;
        pub use crate::provider::{Provider, ProviderOptions, VariableBuilder};
    }
}
