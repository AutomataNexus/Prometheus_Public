#!/usr/bin/env bash
# ============================================================================
# Prometheus Vault Setup
# Initializes HashiCorp Vault, creates secret paths, and stores all env vars.
# Run once after fresh install. Re-run to update secrets.
#
# Usage:
#   ./vault/setup.sh init     — First-time init (generates unseal keys + root token)
#   ./vault/setup.sh secrets  — Store/update secrets interactively
#   ./vault/setup.sh status   — Check vault status
# ============================================================================

set -euo pipefail

VAULT_ADDR="http://127.0.0.1:8200"
export VAULT_ADDR

VAULT_DIR="$(cd "$(dirname "$0")" && pwd)"
DATA_DIR="${VAULT_DIR}/data"
KEYS_FILE="${VAULT_DIR}/.vault-keys"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log()  { echo -e "${GREEN}[vault]${NC} $1"; }
warn() { echo -e "${YELLOW}[vault]${NC} $1"; }
err()  { echo -e "${RED}[vault]${NC} $1" >&2; }

# ── Start Vault server if not running ─────────────────────────────────────
ensure_running() {
    if vault status >/dev/null 2>&1; then
        return 0
    fi
    log "Starting Vault server..."
    mkdir -p "$DATA_DIR"
    nohup vault server -config="${VAULT_DIR}/config.hcl" \
        >"${VAULT_DIR}/vault.log" 2>&1 &
    echo $! > "${VAULT_DIR}/vault.pid"
    sleep 2
    if ! vault status >/dev/null 2>&1; then
        # May be sealed or uninitialized — that's ok
        return 0
    fi
}

# ── Initialize Vault (first time only) ────────────────────────────────────
cmd_init() {
    ensure_running

    if vault status -format=json 2>/dev/null | grep -q '"initialized": true'; then
        warn "Vault already initialized."
        cmd_unseal
        return 0
    fi

    log "Initializing Vault (1 key share, threshold 1 for dev/hackathon)..."
    local init_output
    init_output=$(vault operator init -key-shares=1 -key-threshold=1 -format=json)

    local unseal_key root_token
    unseal_key=$(echo "$init_output" | python3 -c "import sys,json; print(json.load(sys.stdin)['unseal_keys_b64'][0])")
    root_token=$(echo "$init_output" | python3 -c "import sys,json; print(json.load(sys.stdin)['root_token'])")

    # Save keys (chmod 600 — owner-only read)
    cat > "$KEYS_FILE" <<KEYS
VAULT_UNSEAL_KEY=${unseal_key}
VAULT_ROOT_TOKEN=${root_token}
KEYS
    chmod 600 "$KEYS_FILE"

    log "Unseal key and root token saved to ${KEYS_FILE}"
    warn "BACK UP THIS FILE. If lost, vault data is unrecoverable."

    # Unseal
    vault operator unseal "$unseal_key" >/dev/null
    export VAULT_TOKEN="$root_token"
    log "Vault unsealed and ready."

    # Enable KV v2 secrets engine
    vault secrets enable -path=prometheus kv-v2 >/dev/null 2>&1 || true
    log "KV v2 secrets engine enabled at prometheus/"

    echo ""
    log "Vault initialized. Run './vault/setup.sh secrets' to store secrets."
}

# ── Unseal Vault ──────────────────────────────────────────────────────────
cmd_unseal() {
    if ! [ -f "$KEYS_FILE" ]; then
        err "No keys file found at ${KEYS_FILE}. Run 'init' first."
        return 1
    fi
    source "$KEYS_FILE"

    local sealed
    sealed=$(vault status -format=json 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('sealed', True))" 2>/dev/null || echo "True")

    if [ "$sealed" = "True" ]; then
        log "Unsealing vault..."
        vault operator unseal "$VAULT_UNSEAL_KEY" >/dev/null
        log "Vault unsealed."
    fi
    export VAULT_TOKEN="$VAULT_ROOT_TOKEN"
}

# ── Store secrets interactively ───────────────────────────────────────────
cmd_secrets() {
    ensure_running
    cmd_unseal

    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  Prometheus Secret Configuration${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Press Enter to keep existing value (shown as ****), or type new value."
    echo ""

    # Helper: prompt for a secret, store if provided
    store_secret() {
        local path="$1" key="$2" prompt="$3" required="${4:-false}"
        local current
        current=$(vault kv get -field="$key" "prometheus/${path}" 2>/dev/null || echo "")

        local display="(not set)"
        if [ -n "$current" ]; then
            display="(****${current: -4})"
        fi

        local value
        if [ "$required" = "true" ] && [ -z "$current" ]; then
            echo -ne "  ${YELLOW}*${NC} ${prompt} ${display}: "
        else
            echo -ne "  ${prompt} ${display}: "
        fi
        read -r value

        if [ -n "$value" ]; then
            vault kv put "prometheus/${path}" "$key=$value" >/dev/null 2>&1 || \
            vault kv patch "prometheus/${path}" "$key=$value" >/dev/null 2>&1
            echo -e "    ${GREEN}Updated${NC}"
        fi
    }

    # Helper: store multiple keys at a path
    store_group() {
        local path="$1"
        shift
        local args=()
        local updated=false

        while [ $# -gt 0 ]; do
            local key="$1" prompt="$2"
            shift 2
            local current
            current=$(vault kv get -field="$key" "prometheus/${path}" 2>/dev/null || echo "")

            local display="(not set)"
            if [ -n "$current" ]; then
                display="(****${current: -4})"
            fi

            local value
            echo -ne "  ${prompt} ${display}: "
            read -r value

            if [ -n "$value" ]; then
                args+=("${key}=${value}")
                updated=true
            elif [ -n "$current" ]; then
                args+=("${key}=${current}")
            fi
        done

        if [ "$updated" = true ] && [ ${#args[@]} -gt 0 ]; then
            vault kv put "prometheus/${path}" "${args[@]}" >/dev/null
            echo -e "    ${GREEN}Updated${NC}"
        fi
    }

    # ── Server Core ───────────────────────────
    echo -e "${CYAN}── Server Core ──${NC}"
    store_group "server/core" \
        "host"     "PROMETHEUS_HOST [0.0.0.0]" \
        "port"     "PROMETHEUS_PORT [3030]" \
        "data_dir" "PROMETHEUS_DATA_DIR [/tmp/prometheus-data]" \
        "public_url" "PROMETHEUS_PUBLIC_URL" \
        "aegis_db_url" "AEGIS_DB_URL [http://localhost:9091]" \
        "max_trainings" "PROMETHEUS_MAX_TRAININGS [auto]"
    echo ""

    # ── Credential Vault Key ──────────────────
    echo -e "${CYAN}── Credential Vault (AES-256-GCM) ──${NC}"
    store_group "server/vault" \
        "vault_key" "PROMETHEUS_VAULT_KEY (32+ chars)"
    echo ""

    # ── DigitalOcean / Gradient ───────────────
    echo -e "${CYAN}── DigitalOcean / Gradient AI ──${NC}"
    store_group "gradient" \
        "access_key"  "DO_GENAI_ACCESS_KEY" \
        "agent_id"    "GRADIENT_AGENT_ID" \
        "endpoint"    "DO_GENAI_ENDPOINT" \
        "do_api_token" "DIGITALOCEAN_API_TOKEN"
    echo ""

    # ── Stripe ────────────────────────────────
    echo -e "${CYAN}── Stripe Billing ──${NC}"
    store_group "stripe" \
        "secret_key"      "STRIPE_SECRET_KEY" \
        "webhook_secret"  "STRIPE_WEBHOOK_SECRET" \
        "price_basic"     "STRIPE_PRICE_BASIC" \
        "price_pro"       "STRIPE_PRICE_PRO" \
        "price_enterprise" "STRIPE_PRICE_ENTERPRISE" \
        "meter_id"        "STRIPE_METER_ID" \
        "price_overage"   "STRIPE_PRICE_OVERAGE"
    echo ""

    # ── Email (Resend) ────────────────────────
    echo -e "${CYAN}── Email Service (Resend) ──${NC}"
    store_group "email" \
        "resend_api_key"  "RESEND_API_KEY" \
        "from"            "EMAIL_FROM [Prometheus <noreply@automatanexus.com>]" \
        "reply_to"        "EMAIL_REPLY_TO [support@automatanexus.com]" \
        "base_url"        "PROMETHEUS_BASE_URL [http://localhost:3030]" \
        "support_email"   "SUPPORT_EMAIL [support@automatanexus.com]" \
        "security_recipients" "SECURITY_EMAIL_RECIPIENTS (comma-separated)"
    echo ""

    # ── Aegis-DB ──────────────────────────────
    echo -e "${CYAN}── Aegis-DB ──${NC}"
    store_group "aegis" \
        "username" "AEGIS_DB_USERNAME [admin]" \
        "password" "AEGIS_DB_PASSWORD"
    echo ""

    # ── Expo / Mobile ─────────────────────────
    echo -e "${CYAN}── Expo / Mobile ──${NC}"
    store_group "mobile" \
        "expo_token" "EXPO_TOKEN"
    echo ""

    echo -e "${GREEN}All secrets stored in Vault.${NC}"
    echo ""
    echo "To load secrets into your shell:"
    echo "  source ./vault/env.sh"
    echo ""
    echo "To start Prometheus with Vault:"
    echo "  ./vault/start.sh"
}

# ── Status ────────────────────────────────────────────────────────────────
cmd_status() {
    ensure_running
    echo ""
    vault status
    echo ""

    if [ -f "$KEYS_FILE" ]; then
        source "$KEYS_FILE"
        export VAULT_TOKEN="$VAULT_ROOT_TOKEN"

        echo -e "${CYAN}Stored secrets:${NC}"
        for path in server/core server/vault gradient stripe email aegis mobile; do
            local keys
            keys=$(vault kv get -format=json "prometheus/${path}" 2>/dev/null | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)['data']['data']
    for k,v in data.items():
        masked = '****' + v[-4:] if len(v) > 4 else '****'
        print(f'    {k}: {masked}')
except: pass
" 2>/dev/null || echo "    (empty)")
            echo -e "  ${YELLOW}prometheus/${path}${NC}"
            echo "$keys"
        done
    fi
}

# ── Main ──────────────────────────────────────────────────────────────────
case "${1:-help}" in
    init)    cmd_init ;;
    secrets) cmd_secrets ;;
    unseal)  ensure_running; cmd_unseal; log "Ready." ;;
    status)  cmd_status ;;
    stop)
        if [ -f "${VAULT_DIR}/vault.pid" ]; then
            kill "$(cat "${VAULT_DIR}/vault.pid")" 2>/dev/null || true
            rm -f "${VAULT_DIR}/vault.pid"
            log "Vault stopped."
        fi
        ;;
    *)
        echo "Usage: $0 {init|secrets|unseal|status|stop}"
        echo ""
        echo "  init     — Initialize vault (first time)"
        echo "  secrets  — Store/update secrets interactively"
        echo "  unseal   — Unseal vault after restart"
        echo "  status   — Show vault status and stored paths"
        echo "  stop     — Stop vault server"
        ;;
esac
