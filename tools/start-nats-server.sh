#!/bin/bash

set -euo pipefail
set -x

nats-server -a 127.0.0.1 -p 4222
