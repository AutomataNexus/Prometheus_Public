#!/bin/bash
# Prometheus server startup script
# Loads credentials from vault and starts the server

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load vaulted credentials
if [ -f "$SCRIPT_DIR/.secrets/do_credentials.env" ]; then
    set -a
    source "$SCRIPT_DIR/.secrets/do_credentials.env"
    set +a
fi

exec "$SCRIPT_DIR/target/release/prometheus-server"
