#!/bin/sh

REMOTE_ADDR=$1
TARGET=$2

set -eu

REMOTE_DIRECTORY="/usr/bin"
SYSTEMD_SERVICE_DIRECTORY="/usr/lib/systemd/system"
HYDRA_CLIENTS_DIR="/usr/share/uc-iam/clients"
SERVICES="u-os-hub-example-provider u-os-hub-example-consumer"

print_usage() {
    echo ""
    echo "Usage:"
    echo "  $0 REMOTE_ADDRESS TARGET"
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

if [ -z "$REMOTE_ADDR" ]; then
    echo "Error: Missing remote address"
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

echo "--> Build services"
for service in $SERVICES; do
    echo "--> Building $service"
    cargo build --release --example $service --target $TARGET
done

echo "--> Stop the $SERVICES"
ssh root@$REMOTE_ADDR "systemctl stop $SERVICES" || true

echo "--> Mount / as rw and growfs"
ssh root@$REMOTE_ADDR "mount / -o rw,remount && /usr/lib/systemd/systemd-growfs /"

echo "--> Copy new files"
for service in $SERVICES; do
    scp ./target/$TARGET/release/examples/$service root@$REMOTE_ADDR:$REMOTE_DIRECTORY/
    scp ./examples/systemd/$service.service root@$REMOTE_ADDR:$SYSTEMD_SERVICE_DIRECTORY/
done

echo "--> Overwrite file permissions"
for service in $SERVICES; do
    ssh root@$REMOTE_ADDR "chmod +x $REMOTE_DIRECTORY/$service"
done


# generate credentials
echo "--> Generate credentials"
ssh root@$REMOTE_ADDR "echo 'hub.variables.provide' > $HYDRA_CLIENTS_DIR/u_os_hub_example_provider"
ssh root@$REMOTE_ADDR "echo 'hub.variables.readwrite' > $HYDRA_CLIENTS_DIR/u_os_hub_example_consumer"
ssh root@$REMOTE_ADDR "systemctl restart hydra-client-creator"

echo "--> Mount / as ro"
ssh root@$REMOTE_ADDR "mount / -o ro,remount"

echo "--> Reload systemd"
ssh root@$REMOTE_ADDR "systemctl daemon-reload"

echo "--> Enable and start the services"
ssh root@$REMOTE_ADDR "systemctl enable --now $SERVICES"
