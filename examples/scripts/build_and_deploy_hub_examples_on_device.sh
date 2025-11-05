#!/bin/bash

# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

TARGET=$1

set -euo pipefail

# Where the compiled example executables will be placed on the target device
EXECUTABLE_DIR="/opt"

# Where the systemd service files will be placed on the target device
SYSTEMD_SERVICE_DIRECTORY="/etc/systemd/system"

# List of services to build and deploy
SERVICES="u-os-hub-example-provider u-os-hub-example-consumer"

# Important paths
HYDRA_ADMIN_SOCKET="/run/hydra/admin.sock"
CREDENTIAL_STORE="/etc/credstore.encrypted"

# Function to print usage information
print_usage() {
    echo ""
    echo "Usage:"
    echo "  $0 TARGET"
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

if [ "$EUID" -ne 0 ]; then
    echo "Please run as root or with sudo"
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

# Function to create a machine client in Hydra and store its credentials securely
# Arguments:
#   $1 - client name
#   $2 - scope
create_machine_client() {
    local client_name=$1
    local scope=$2

    echo "Creating client: ${client_name}"

    local client_create_payload="{
        \"client_name\": \"${client_name}\",
        \"grant_types\": [\"client_credentials\"],
        \"owner\": \"System\",
        \"scope\": \"${scope}\",
        \"token_endpoint_auth_method\": \"client_secret_basic\"
    }"

    client_result=$(curl -s --unix-socket "$HYDRA_ADMIN_SOCKET" --location "http://hydra/admin/clients" \
        --header "Content-Type: application/json" --header "Accept: application/json" --data "$client_create_payload")

    client_id=$(echo -e "$client_result" | jq -r ".client_id")
    client_secret=$(echo -e "$client_result" | jq -r ".client_secret")

    creds="CLIENT_ID=${client_id}\nCLIENT_SECRET=${client_secret}"
    echo -e "${creds}" | systemd-creds encrypt -H - ${CREDENTIAL_STORE}/"${client_name}".creds --name="${client_name}"
}

# Create credentials for NATS using the hydra admin socket
echo "--> Generate credentials"
create_machine_client "u_os_hub_example_provider" "hub.variables.provide"
create_machine_client "u_os_hub_example_consumer" "hub.variables.readwrite"

echo "--> Stop the $SERVICES"
systemctl stop $SERVICES 2>/dev/null || true

echo "--> Move files from /tmp to their final locations"
for service in $SERVICES; do
    mv /tmp/$service $EXECUTABLE_DIR/$service
    mv /tmp/$service.service $SYSTEMD_SERVICE_DIRECTORY/$service.service
done

echo "--> Overwrite file permissions"
for service in $SERVICES; do
    chmod +x $EXECUTABLE_DIR/$service
done

echo "--> Reload systemd"
systemctl daemon-reload

echo "--> Enable and start the services"
systemctl enable --now $SERVICES
