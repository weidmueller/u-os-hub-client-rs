#!/bin/bash

set -e

BUILD_MODE="$1"
TARGET="$2"

set -eux

script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source $script_dir/configure-cross-env.sh $TARGET

cargo build --profile=$BUILD_MODE --target=$TARGET