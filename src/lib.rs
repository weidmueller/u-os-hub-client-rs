//! Welcome to the u-OS Data Hub client library.
//!
//! This library allows to interact with the u-OS Variable Data Hub as a provider or consumer.
//!
//! As a starting point, please take a look at the [provider] and [consumer] module documentation.
//! They provide example code and a detailed description of the available APIs.
//!
//! # Features
//!
//! * `export-low-level-api` - Exports low level features as public API. See [consumer] module documentation for details about the low level api. (Default: `false`)

// Contains the generated flatbuffers.
// The module comment/description is located here to not change the generated files.
// Because of the same reason the "dead_code", "unused_imports" and "clippy::all" is allowed for the generated module.
#[cfg(not(feature = "export-low-level-api"))]
#[allow(dead_code, unused_imports, clippy::all, clippy::unwrap_used, clippy::expect_used)]
#[rustfmt::skip]
mod generated;
#[cfg(feature = "export-low-level-api")]
#[allow(dead_code, unused_imports, clippy::all, clippy::unwrap_used, clippy::expect_used)]
#[rustfmt::skip]
pub mod generated;

// Note: We force documentation for public items for some select modules.
#[warn(missing_docs)]
pub mod authenticated_nats_con;
#[warn(missing_docs)]
pub mod consumer;
#[warn(missing_docs)]
pub mod dh_types;
pub mod env_file_parser;
pub mod nats_subjects;
pub mod oauth2;
pub mod payload_builders;
#[warn(missing_docs)]
pub mod provider;
#[warn(missing_docs)]
pub mod variable;
pub mod variable_definition_serde_extension;
pub mod variable_value_type_serde_extension;
