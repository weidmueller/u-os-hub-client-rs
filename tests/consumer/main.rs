// For integration test code, these lints are explicitly allowed.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "../utils/mod.rs"]
mod utils;

mod dh_consumer;
mod dh_provider_con;
mod dummy_provider;
mod incompatible_provider;
