#!/bin/bash

script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

set -eux

# Check formatting once
cargo fmt --check

# Build with u-OS rust toolchain and oldest possible dependencies
rm -f Cargo.lock
cargo +nightly -Zminimal-versions update
"${script_dir}/build-for-target.sh" dev x86_64-unknown-linux-gnu ${U_OS_RUST_VERSION}
cargo +${U_OS_RUST_VERSION} clippy --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo +${U_OS_RUST_VERSION} doc --no-deps
cargo +${U_OS_RUST_VERSION} test --all-features --target x86_64-unknown-linux-gnu

# Build with latest rust toolchain and latest dependencies
rm -f Cargo.lock
"${script_dir}/build-for-target.sh" dev x86_64-unknown-linux-gnu

# Lib and high level examples must also build without low level feature flag
cargo build
cargo build --example u-os-hub-example-provider
cargo build --example u-os-hub-example-consumer

cargo clippy --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo test --all-features --target x86_64-unknown-linux-gnu

# Audit once, independently of the rust version
cargo audit
