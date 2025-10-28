#!/bin/bash

# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

BUILD_MODE="$1"
TARGET="$2"
TOOLCHAIN="$3"

set -euxo pipefail

if [ -z "$TOOLCHAIN" ]; then
    cargo build --all-features --all-targets --profile=$BUILD_MODE --target=$TARGET
else
    cargo +$TOOLCHAIN build --all-features --all-targets --profile=$BUILD_MODE --target=$TARGET
fi
