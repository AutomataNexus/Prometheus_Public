#!/usr/bin/env bash
# ============================================================================
# Prometheus Vault → Environment Loader
# Sources secrets from HashiCorp Vault into environment variables.
#
# Usage:
#   source ./vault/env.sh          — Load all secrets into current shell
#   source ./vault/env.sh quiet    — Same but suppress output
#
# Call this before starting prometheus-server, or use vault/start.sh.
# ============================================================================

VAULT_ADDR="${VAULT_ADDR:-http://127.0.0.1:8200}"
export VAULT_ADDR

VAULT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEYS_FILE="${VAULT_DIR}/.vault-keys"
QUIET="${1:-}"

_log() { [ "$QUIET" != "quiet" ] && echo -e "\033[0;32m[vault]\033[0m $1" || true; }
_warn() { [ "$QUIET" != "quiet" ] && echo -e "\033[1;33m[vault]\033[0m $1" || true; }

# Load vault token
if [ -f "$KEYS_FILE" ]; then
    source "$KEYS_FILE"
    export VAULT_TOKEN="$VAULT_ROOT_TOKEN"
else
    _warn "No vault keys found. Run './vault/setup.sh init' first."
    return 1 2>/dev/null || exit 1
fi

# Check vault is reachable and unsealed
if ! vault status >/dev/null 2>&1; then
    _warn "Vault not running or sealed. Run './vault/setup.sh unseal'"
    return 1 2>/dev/null || exit 1
fi

# Helper: read a vault path and export env vars
_load_path() {
    local vault_path="$1"
    shift
    # Remaining args are pairs: vault_key ENV_VAR_NAME
    while [ $# -ge 2 ]; do
        local vkey="$1" envvar="$2"
        shift 2
        local val
        val=$(vault kv get -field="$vkey" "prometheus/${vault_path}" 2>/dev/null) || val=""
        if [ -n "$val" ]; then
            export "$envvar"="$val"
            _log "  ${envvar}=****${val: -4}"
        fi
    done
}

_log "Loading secrets from Vault..."

# ── Server Core ──
_load_path "server/core" \
    host          PROMETHEUS_HOST \
    port          PROMETHEUS_PORT \
    data_dir      PROMETHEUS_DATA_DIR \
    public_url    PROMETHEUS_PUBLIC_URL \
    aegis_db_url  AEGIS_DB_URL \
    max_trainings PROMETHEUS_MAX_TRAININGS

# ── Credential Vault Key ──
_load_path "server/vault" \
    vault_key     PROMETHEUS_VAULT_KEY

# ── Gradient AI ──
_load_path "gradient" \
    access_key    DO_GENAI_ACCESS_KEY \
    agent_id      GRADIENT_AGENT_ID \
    endpoint      DO_GENAI_ENDPOINT \
    do_api_token  DIGITALOCEAN_API_TOKEN

# ── Stripe ──
_load_path "stripe" \
    secret_key       STRIPE_SECRET_KEY \
    webhook_secret   STRIPE_WEBHOOK_SECRET \
    price_basic      STRIPE_PRICE_BASIC \
    price_pro        STRIPE_PRICE_PRO \
    price_enterprise STRIPE_PRICE_ENTERPRISE \
    meter_id         STRIPE_METER_ID \
    price_overage    STRIPE_PRICE_OVERAGE

# ── Email ──
_load_path "email" \
    resend_api_key      RESEND_API_KEY \
    from                EMAIL_FROM \
    reply_to            EMAIL_REPLY_TO \
    base_url            PROMETHEUS_BASE_URL \
    support_email       SUPPORT_EMAIL \
    security_recipients SECURITY_EMAIL_RECIPIENTS

# ── Aegis-DB ──
_load_path "aegis" \
    username  AEGIS_DB_USERNAME \
    password  AEGIS_DB_PASSWORD

# ── Expo / Mobile ──
_load_path "mobile" \
    expo_token  EXPO_TOKEN

# ── Deployment ──
_load_path "deployment" \
    remote_host  PROMETHEUS_DEPLOY_HOST \
    sudo_pass    PROMETHEUS_DEPLOY_SUDO

_log "Done. $(env | grep -cE '^(PROMETHEUS_|STRIPE_|DO_|GRADIENT_|AEGIS_|RESEND_|EMAIL_|EXPO_|SUPPORT_|SECURITY_|DIGITALOCEAN_)') secrets loaded."
