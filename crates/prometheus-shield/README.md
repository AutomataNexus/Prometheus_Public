# prometheus-shield

Adaptive zero-trust security engine for the Prometheus ML platform. Provides layered threat defense as an Axum middleware layer, protecting the multi-source data connector and all API endpoints from SQL injection, SSRF, credential theft, automated attacks, email abuse, and data smuggling.

Built in pure Rust. No C dependencies. No regex-based security.

## Architecture

```
Incoming Request
    |
    v
[Rate Governor]       per-IP token bucket with behavioral escalation
    |
    v
[Fingerprinter]       header analysis, bot detection, behavioral tracking
    |
    v
[Threat Scorer]       weighted multi-signal assessment (0.0-1.0)
    |
    +---> BLOCK (>= 0.7) --> 403 + audit event
    +---> WARN  (>= 0.4) --> allow + log + audit event
    +---> ALLOW (< 0.4)  --> pass through, track behavior
    |
    v
[Handler]             route-level validation via Shield methods:
                      validate_sql(), validate_url(), quarantine_csv(), etc.
    |
    v
[Post-response]       error tracking for behavioral analysis
```

## Security Modules

### SQL Firewall (`sql_firewall.rs`)

AST-level SQL injection detection using `sqlparser`. Parses queries into an abstract syntax tree and performs semantic analysis rather than pattern matching. Detects:

- Stacked queries (`;`-separated multi-statement)
- UNION-based injection
- Tautology attacks (`1=1`, `'a'='a'`, `OR TRUE`)
- Dangerous function calls (`LOAD_FILE`, `xp_cmdshell`, `pg_read_file`, `SLEEP`, `BENCHMARK` -- 30+ functions)
- System catalog access (`information_schema`, `pg_catalog`, `sqlite_master`, etc.)
- INTO OUTFILE / DUMPFILE exfiltration
- Comment injection (`--`, `/* */`)
- Hex-encoded and CHAR/CHR-encoded payloads
- Excessive subquery nesting (configurable depth limit)
- Non-SELECT statements (INSERT, UPDATE, DELETE, DROP, ALTER, TRUNCATE)

Each violation contributes to a per-query risk score. Queries with any violation or risk score >= 0.5 are rejected.

### SSRF Guard (`ssrf_guard.rs`)

Validates URLs and IP addresses to prevent server-side request forgery. Blocks:

- Private IP ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
- Loopback addresses (127.0.0.0/8, ::1)
- Link-local addresses (169.254.0.0/16, fe80::/10)
- Cloud metadata endpoints (169.254.169.254, metadata.google.internal)
- IPv4-mapped IPv6 bypass attempts (::ffff:127.0.0.1)
- Internal TLDs (.internal, .local, .corp, .home, .lan)
- Dangerous port numbers (22, 3306, 5432, 6379, 27017, etc.)
- Non-HTTP schemes (file://, gopher://, etc.)
- Configurable allowlist/blocklist for explicit overrides

### Rate Governor (`rate_governor.rs`)

Per-IP adaptive rate limiter using a token bucket algorithm with automatic escalation. Well-behaved clients get full capacity; repeat violators are progressively restricted.

Escalation levels: **None** -> **Warn** -> **Throttle** -> **Block** -> **Ban**

- Token bucket: configurable requests/second (default 50) and burst capacity (default 100)
- Violations decay over time (default 60s decay period)
- Throttled clients must have > 50% bucket capacity to proceed
- Bans are temporary (default 300s) with `Retry-After` header
- Manual ban/unban API for administrative control
- Stale bucket pruning to bound memory usage

### Credential Vault (`credential_vault.rs`)

Encrypts sensitive fields (API keys, connection strings, passwords, tokens) at rest using AES-256-GCM before storage in Aegis-DB.

**Key hierarchy:**

```
PROMETHEUS_VAULT_KEY (env var, >= 32 chars)
  +-- SHA-256(master_key + ":user:" + user_id) --> per-user 256-bit key
        +-- AES-256-GCM(key, random 96-bit nonce, plaintext) --> ciphertext
```

- Per-user key derivation ensures credential isolation between users
- Random nonce per encryption operation (no nonce reuse)
- Encrypted values are prefixed with `vault:v1:` for identification
- Idempotent: already-encrypted fields are not re-encrypted
- Graceful degradation: if `PROMETHEUS_VAULT_KEY` is unset, credentials pass through unencrypted (dev mode)
- Redaction API masks secrets in API responses (`"my-a......23"`)
- Sensitive fields detected automatically: `api_key`, `token`, `connection_string`, `password`, `secret`, `api_secret`, `access_key`, `secret_key`

### Audit Chain (`audit_chain.rs`)

SHA-256 hash-chained, tamper-evident security event log. Every event includes the hash of the previous event, forming an append-only chain. If any event is modified, inserted, or deleted, the chain breaks -- detectable via `verify_chain()`.

Recorded event types: `RequestAllowed`, `RequestBlocked`, `RateLimitHit`, `SqlInjectionAttempt`, `SsrfAttempt`, `PathTraversalAttempt`, `MaliciousPayload`, `DataQuarantined`, `AuthFailure`, `BanIssued`, `BanLifted`, `ChainVerified`.

Each event stores: UUID, timestamp, event type, source IP, details (internal only), threat score, previous hash, and its own SHA-256 hash. The genesis event chains from the string `"genesis"`.

Features:
- Configurable max events with automatic pruning (default 100,000)
- `count_since()` for time-windowed event queries
- `recent()` for most-recent-first retrieval
- `export_json()` for external audit consumption

### Data Quarantine (`quarantine.rs`)

Validates imported data (CSV, JSON) before it enters the training pipeline. Detects:

- Formula injection (`=CMD()`, `@SUM()`, `+`, `-` followed by non-numeric content)
- Embedded scripts (`<script>`, `javascript:`, event handlers)
- Null byte injection
- Oversized payloads (configurable: default 500 MB, 5M rows, 500 columns)
- Padding/amplification attacks (low character diversity detection)

CSV scanning is capped at 10,000 rows for performance since attackers typically inject early in the payload.

### Request Fingerprinting (`fingerprint.rs`)

Extracts behavioral signals from HTTP requests to identify automated attack tools. Signals include:

- Presence/absence of standard headers (User-Agent, Accept, Accept-Language, Accept-Encoding)
- Header count and ordering (SHA-256 hash of ordered header names)
- Known attack tool User-Agent detection (sqlmap, nikto, nmap, nuclei, gobuster, ffuf, etc.)
- Behavioral tracking per IP: request rate, error rate, burst patterns, endpoint scanning velocity

Fingerprints are stable -- the same client configuration produces the same hash.

### Threat Scoring (`threat_score.rs`)

Combines all signals into a single 0.0--1.0 threat score using weighted aggregation:

| Signal | Weight |
|---|---|
| Fingerprint anomaly | 0.30 |
| Behavioral anomaly | 0.30 |
| Rate pressure | 0.25 |
| Recent violations | 0.15 |

Thresholds (configurable):
- **Allow**: score < 0.4
- **Warn**: 0.4 <= score < 0.7 (request proceeds, event logged)
- **Block**: score >= 0.7 (403 Forbidden)

### Input Sanitizer (`sanitizer.rs`)

Validates connection strings, file paths, and error messages:

- Shell metacharacter blocking (`` ` ``, `$`, `|`, `&`, `;`, newlines, null bytes)
- Command substitution detection (`$()`, `${}`)
- Database URL parameter injection (`sslrootcert=/etc`, `init_command=`, etc.)
- Path traversal prevention (`..`, blocked system directories, sensitive filenames)
- Error message sanitization: redacts internal paths, private IPs, and stack traces from client-facing errors
- Header injection prevention (CRLF stripping)

### Email Guard (`email_guard.rs`)

Protects email-sending endpoints from abuse:

- CRLF header injection detection in addresses, subjects, and names
- Email address format validation and domain safety checks
- Disposable/temporary email domain blocking (mailinator, guerrillamail, yopmail, etc.)
- IP-address domain rejection
- Per-recipient rate limiting (default: 5 emails per 5-minute window)
- Recipient count limits (default: 10 per message)
- HTML/script injection detection in template content (24+ patterns: `<script>`, `<svg>`, `<iframe>`, event handlers, `javascript:`, `data:text/html`, etc.)
- Encoded payload detection: base64-encoded attack strings, Unicode BiDi overrides, zero-width characters
- HTML escaping utility for safe template interpolation

## Axum Middleware Integration

```rust
use prometheus_shield::{Shield, ShieldConfig, shield_middleware};
use std::sync::Arc;

let shield = Arc::new(Shield::new(ShieldConfig::default()));

let app = Router::new()
    .route("/api/v1/sources", post(create_source))
    .layer(Extension(shield.clone()))
    .layer(axum::middleware::from_fn(shield_middleware));
```

The middleware runs on every request and performs: rate limiting, fingerprinting, behavioral scoring, threat assessment, and post-response error tracking. Route handlers call Shield methods directly for content-specific validation:

```rust
async fn create_source(
    Extension(shield): Extension<Arc<Shield>>,
    Json(body): Json<CreateSource>,
) -> Result<Json<Source>, ShieldError> {
    shield.validate_url(&body.url)?;
    shield.validate_sql(&body.query)?;
    shield.validate_connection_string(&body.connection_string)?;
    shield.quarantine_csv(&body.csv_data)?;
    // ...
}
```

`ShieldError` implements `IntoResponse` with deliberately vague client-facing messages to avoid leaking security internals. Internal details are recorded in the audit chain.

## Configuration

All thresholds and limits are configurable via `ShieldConfig`:

```rust
ShieldConfig {
    block_threshold: 0.7,        // threat score to block
    warn_threshold: 0.4,         // threat score to warn
    audit_max_events: 100_000,   // max events before pruning
    sql: SqlFirewallConfig { .. },
    ssrf: SsrfConfig { .. },
    rate: RateConfig { .. },
    quarantine: QuarantineConfig { .. },
    email: EmailGuardConfig { .. },
}
```

## Dependencies

- `sqlparser` -- AST-level SQL parsing (not regex)
- `aes-gcm` -- AES-256-GCM authenticated encryption
- `sha2` -- SHA-256 hashing for audit chain and fingerprinting
- `parking_lot` -- Fast RwLock/Mutex without lock poisoning
- `url` -- URL parsing for SSRF validation
- `axum` -- Middleware integration
- `chrono`, `uuid`, `serde`, `tracing`, `thiserror`
