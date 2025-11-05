#!/bin/bash

# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

REMOTE_LOGIN=$1
TARGET=$2

# List of services to build and deploy
SERVICES="u-os-hub-example-provider u-os-hub-example-consumer"

# Name of the script that runs on the target device
DEVICE_SCRIPT_NAME="build_and_deploy_hub_examples_on_device.sh"

set -euo pipefail

# Function to print usage information
print_usage() {
    echo ""
    echo "Usage:"
    echo "  $0 USER@ADDRESS TARGET"
    echo ""
    echo "User must have administrative rights on the remote device."
    echo ""
    echo "Target examples:"
    echo "  'ucu', 'ucg', 'ucm' and 'x86_64' which got mapped to the cargo target"
    echo ""
    echo "  Or the cargo targets direct:"
    echo "    aarch64-unknown-linux-gnu   - arm64 (ucu)"
    echo "    armv7-unknown-linux-gnueabihf - arm32 (ucm, ucg)"
    echo "    x86_64-unknown-linux-gnu    - x86_64"
    exit 1
}

if [ -z "$REMOTE_LOGIN" ]; then
    echo "Error: Missing remote user and address"
    print_usage
    exit 1
fi

if [ -z "$TARGET" ]; then
    echo "Error: Missing target"
    print_usage
    exit 1
fi

if [ "$TARGET" = "ucu" ]; then
    TARGET="aarch64-unknown-linux-gnu"
elif [ "$TARGET" = "ucm" ] || [ "$TARGET" = "ucg" ]; then
    TARGET="armv7-unknown-linux-gnueabihf"
elif [ "$TARGET" = "x86_64" ]; then
    TARGET="x86_64-unknown-linux-gnu"
fi

cd "$(dirname "$(readlink -f "$0")")/.." || exit 1

# Prompt password for sshpass
read -sp "Enter SSH password: " PASSWORD
echo
export SSHPASS="$PASSWORD"

echo "--> Build services"
for service in $SERVICES; do
    echo "--> Building $service"
    cargo build --release --example $service --target $TARGET
done

# Copy service and executable files to tmp folder on device
for service in $SERVICES; do
    echo "--> Copying $service files to /tmp on device"
    sshpass -e scp ./target/$TARGET/release/examples/$service $REMOTE_LOGIN:/tmp/
    sshpass -e scp ./examples/systemd/$service.service $REMOTE_LOGIN:/tmp/
done

# Copy install script to device
echo "--> Copy install script to /tmp on device"
sshpass -e scp ./examples/scripts/$DEVICE_SCRIPT_NAME $REMOTE_LOGIN:/tmp/

# Run install script with interactive SSH shell
echo "--> Run install script on device"
echo "Note: You will be prompted for the sudo password on the remote device."
sshpass -e ssh -tt $REMOTE_LOGIN "sudo bash /tmp/$DEVICE_SCRIPT_NAME $TARGET"

# Remove script from tmp again
sshpass -e ssh $REMOTE_LOGIN "rm -f /tmp/$DEVICE_SCRIPT_NAME"
