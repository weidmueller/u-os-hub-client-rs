//! This is the internal u-OS Data Hub client library.
//! The focus of this library is high performance.
//!
//! This library primary contains:
//!
//! - Provider Handling
//! - Consumer Handling (currently not implemented)
//! - The generated flatbuffers of the Variable-NATS-API
//!

// The module comment/description is located here to not change the generated files.
// Because of the same reason the "dead_code", "unused_imports" and "clippy::all" is allowed for the generated module.
#[allow(dead_code, unused_imports, clippy::all)]
#[rustfmt::skip]
/// Contains the generated flatbuffers.
pub mod generated;
pub mod env_file_parser;
pub mod oauth2;
pub mod payload_builders;
pub mod provider;
pub mod subjects;
pub mod variable;
