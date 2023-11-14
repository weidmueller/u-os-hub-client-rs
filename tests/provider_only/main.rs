// For integration test code, these lints are explicitly allowed.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "../utils/mod.rs"]
mod utils;

mod add_variable_test;
mod read_variable_test;
mod register_provider_test;
mod remove_variable_test;
mod update_variable_value_test;
mod write_variable_test;
