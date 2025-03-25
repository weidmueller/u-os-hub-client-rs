#!/bin/bash

set -eux

cargo build --all-targets
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo audit
cargo test