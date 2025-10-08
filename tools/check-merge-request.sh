#!/bin/bash

# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

if [ -n "${BUILD_ALL_ARCHS}" ]; then
    build_all_archs="true"
else
    build_all_archs="false"
fi
echo "BUILD_ALL_ARCHS is set to ${build_all_archs}"

set -euxo pipefail

# Check copyright headers and license files
reuse lint

# Default build profile
profile="dev"

# Check version consistency
"${script_dir}/check_version.sh"

# Check licenses of all transitive dependencies
cargo-deny check licenses

# Check formatting once
cargo fmt --check

# Build with u-OS rust toolchain and oldest possible dependencies
rm -f Cargo.lock
cargo +nightly -Zminimal-versions update

"${script_dir}/build-for-target.sh" ${profile} x86_64-unknown-linux-gnu ${U_OS_RUST_VERSION}
if [ "${build_all_archs}" = "true" ]; then
    "${script_dir}/build-for-target.sh" ${profile} armv7-unknown-linux-gnueabihf ${U_OS_RUST_VERSION}
    "${script_dir}/build-for-target.sh" ${profile} aarch64-unknown-linux-gnu ${U_OS_RUST_VERSION}
fi

cargo +${U_OS_RUST_VERSION} clippy --profile ${profile} --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo +${U_OS_RUST_VERSION} doc --profile ${profile} --no-deps
cargo +${U_OS_RUST_VERSION} test --profile ${profile} --all-features --target x86_64-unknown-linux-gnu

# Build with latest rust toolchain and latest dependencies
rm -f Cargo.lock
"${script_dir}/build-for-target.sh" ${profile} x86_64-unknown-linux-gnu

# Lib and high level examples must also build without low level feature flag
cargo build --profile ${profile} --target x86_64-unknown-linux-gnu
cargo build --profile ${profile} --target x86_64-unknown-linux-gnu --example u-os-hub-example-provider
cargo build --profile ${profile} --target x86_64-unknown-linux-gnu --example u-os-hub-example-consumer

if [ "${build_all_archs}" = "true" ]; then
    "${script_dir}/build-for-target.sh" ${profile} armv7-unknown-linux-gnueabihf
    "${script_dir}/build-for-target.sh" ${profile} aarch64-unknown-linux-gnu
fi

cargo clippy --profile ${profile} --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --profile ${profile}
cargo test --profile ${profile} --all-features --target x86_64-unknown-linux-gnu

# Audit once, independently of the rust version
cargo audit
