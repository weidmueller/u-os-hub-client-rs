#!/bin/bash

# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

set -euo pipefail

# Where the compiled example executables will be placed on the target device
EXECUTABLE_DIR="/opt"

# Where the systemd service files will be placed on the target device
SYSTEMD_SERVICE_DIRECTORY="/etc/systemd/system"

# List of services to build and deploy
SERVICES="u-os-hub-example-provider u-os-hub-example-consumer"

# Important paths
CREDENTIAL_STORE="/etc/credstore.encrypted"

echo "--> Disable the $SERVICES"
systemctl stop $SERVICES || true
systemctl disable $SERVICES || true

echo "--> Remove service and executable files"
for service in $SERVICES; do
    rm -f $EXECUTABLE_DIR/$service
    rm -f $SYSTEMD_SERVICE_DIRECTORY/$service.service
done

echo "--> Remove credentials"
rm -f $CREDENTIAL_STORE/u_os_hub_example_*

echo "--> Reload systemd"
systemctl daemon-reload
