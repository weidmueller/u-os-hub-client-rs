#!/bin/sh

REMOTE_ADDR=$1

set -eu

REMOTE_DIRECTORY="/usr/bin"
SYSTEMD_SERVICE_DIRECTORY="/usr/lib/systemd/system"
HYDRA_CLIENTS_DIR="/usr/share/uc-iam/clients"
#Note: This includes the old names on purpose, so that old dummy examples also get cleaned up
SERVICES="u-os-hub-example-provider u-os-hub-example-consumer uc-hub-dummy-provider uc-hub-dummy-consumer"

print_usage() {
    echo ""
    echo "Usage:"
    echo "  $0 REMOTE_ADDRESS"
    exit 1
}

if [ -z "$REMOTE_ADDR" ]; then
    echo "Error: Missing remote address"
    print_usage
    exit 1
fi

cd "$(dirname "$(readlink -f "$0")")/.." || exit 1

echo "--> Disable the $SERVICES"
ssh root@$REMOTE_ADDR "systemctl stop $SERVICES" || true
ssh root@$REMOTE_ADDR "systemctl disable $SERVICES" || true

echo "--> Mount / as rw and growfs"
ssh root@$REMOTE_ADDR "mount / -o rw,remount && /usr/lib/systemd/systemd-growfs /"

echo "--> Remove files"
for service in $SERVICES; do
    ssh root@$REMOTE_ADDR "rm -f $REMOTE_DIRECTORY/$service"
    ssh root@$REMOTE_ADDR "rm -f $REMOTE_DIRECTORY/$service.service"
done

echo "--> Remove credentials"
ssh root@$REMOTE_ADDR "rm -f $HYDRA_CLIENTS_DIR/u_os_hub_example_*"
#Note: This includes the old names on purpose, so that old dummy examples also get cleaned up
ssh root@$REMOTE_ADDR "rm -f $HYDRA_CLIENTS_DIR/u_os_uc_hub_dummy_*"
ssh root@$REMOTE_ADDR "systemctl restart hydra-client-creator"

echo "--> Mount / as ro"
ssh root@$REMOTE_ADDR "mount / -o ro,remount"

echo "--> Reload systemd"
ssh root@$REMOTE_ADDR "systemctl daemon-reload"
