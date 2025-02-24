#!/bin/bash

TARGET="$1"

set -e

if [ "$TARGET" = "armv7-unknown-linux-gnueabihf" ]; then
    export PKG_CONFIG_PATH="/usr/lib/arm-linux-gnueabihf/pkgconfig"
    export PKG_CONFIG_ALLOW_CROSS="true"
elif [ "$TARGET" = "aarch64-unknown-linux-gnu" ]; then
    export PKG_CONFIG_PATH="/usr/lib/aarch64-linux-gnu/pkgconfig"
    export PKG_CONFIG_ALLOW_CROSS="true"
elif [ "$TARGET" = "x86_64-unknown-linux-gnu" ]; then
    export PKG_CONFIG_ALLOW_CROSS="false"
else
    echo "Invalid target: $TARGET"
    exit 1
fi
