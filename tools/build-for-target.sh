#!/bin/bash

set -e

BUILD_MODE="$1"
TARGET="$2"
TOOLCHAIN="$3"

set -eux

script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source $script_dir/configure-cross-env.sh $TARGET

if [ -z "$TOOLCHAIN" ]; then
    cargo build --all-targets --profile=$BUILD_MODE --target=$TARGET
else
    cargo +$TOOLCHAIN build --all-targets --profile=$BUILD_MODE --target=$TARGET
fi