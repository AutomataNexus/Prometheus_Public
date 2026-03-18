#!/usr/bin/env bash
# ============================================================================
# Start Prometheus with Vault-managed secrets
# Unseals vault, loads env vars, then starts the server.
# ============================================================================

set -euo pipefail

VAULT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$VAULT_DIR")"

echo "Starting Prometheus with HashiCorp Vault..."

# 1. Unseal vault
"${VAULT_DIR}/setup.sh" unseal

# 2. Load secrets into environment
source "${VAULT_DIR}/env.sh"

# 3. Start server
cd "$PROJECT_DIR"
exec ./target/release/prometheus-server
