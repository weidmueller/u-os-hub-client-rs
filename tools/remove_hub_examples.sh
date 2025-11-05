#!/bin/bash

# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

REMOTE_LOGIN=$1

DEVICE_SCRIPT_NAME="remove_hub_examples_on_device.sh"

set -euo pipefail

print_usage() {
    echo ""
    echo "Usage:"
    echo "  $0 user@address"
    exit 1
}

if [ -z "$REMOTE_LOGIN" ]; then
    echo "Error: Missing remote login"
    print_usage
    exit 1
fi

cd "$(dirname "$(readlink -f "$0")")/.." || exit 1

# Prompt password for sshpass
read -sp "Enter SSH password: " PASSWORD
echo
export SSHPASS="$PASSWORD"

# Copy script to device
echo "--> Copy uninstall script to /tmp on device"
sshpass -e scp ./examples/scripts/$DEVICE_SCRIPT_NAME $REMOTE_LOGIN:/tmp/

# Run script with interactive SSH shell
echo "--> Run uninstall script on device"
echo "Note: You will be prompted for the sudo password on the remote device."
sshpass -e ssh -tt $REMOTE_LOGIN "sudo bash /tmp/$DEVICE_SCRIPT_NAME"

# Remove script from tmp again
sshpass -e ssh $REMOTE_LOGIN "rm -f /tmp/$DEVICE_SCRIPT_NAME"
