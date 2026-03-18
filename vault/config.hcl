// Prometheus Vault Configuration
// Storage: file-based (persistent across restarts)
// Listener: local only (127.0.0.1:8200)

storage "file" {
  path = "/opt/Prometheus/vault/data"
}

listener "tcp" {
  address     = "127.0.0.1:8200"
  tls_disable = 1  // TLS handled by reverse proxy in production
}

// Disable mlock for WSL/container environments
disable_mlock = true

api_addr = "http://127.0.0.1:8200"

ui = true

// Auto-unseal not configured — manual unseal with key shares
// For production, use transit auto-unseal or cloud KMS
