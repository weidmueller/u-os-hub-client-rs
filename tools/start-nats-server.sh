#!/bin/bash

set -euo pipefail

# Check for invalid, e.g. empty, downloads
# This may happen if a nats version gets yanked or is taken offline
# Unfortunately, the nats download script does not properly handle this
if [[ ! -s "$(command -v nats-server)" || ! -x "$(command -v nats-server)" ]]; then
    echo "Error: nats-server is not found, not executable, or is empty. Validate download."
    exit 1
fi

set -x
nats-server -a 127.0.0.1 -p 4222
