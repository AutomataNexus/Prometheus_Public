# prometheus-server

Axum 0.7 HTTP server powering the Prometheus AI edge ML training orchestrator. Provides a REST API, WebSocket streaming, bearer-token auth, and serves the Leptos WASM UI as a single binary.

## Architecture

```
main.rs          Entry point: tracing, config, Aegis-DB init, Shield init, lifecycle spawn, bind
config.rs        ServerConfig populated from environment variables (all secrets via vault/env.sh)
state.rs         AppState: shared reqwest client, ServerConfig, active training map, training queue
router.rs        Axum Router: public routes, auth-protected routes, WS, CORS, static file serving
error.rs         AppError enum -> HTTP status code mapping (thiserror + IntoResponse)
auth/            Login/logout proxy to Aegis-DB, bearer token middleware, RBAC (admin/operator/viewer)
ws/              WebSocket handler for real-time training progress streaming
api/             Handler modules (one per domain)
```

### Request flow

1. **Shield middleware** (outermost layer) -- SQL injection, SSRF, path traversal, rate limiting, threat scoring
2. **CORS** (permissive) + **TraceLayer** (tower-http)
3. **Auth middleware** on protected routes -- validates `Authorization: Bearer <token>` against Aegis-DB `/api/v1/auth/me`, or validates `prom_*` ingestion keys. Injects `AuthUser` into request extensions.
4. **Handler** reads `AuthUser` from extensions, calls Aegis-DB document/SQL APIs via `AppState` helpers, returns JSON.

### Auth model

Opaque bearer tokens issued by Aegis-DB. No JWT. The server proxies login to Aegis-DB and passes the returned token to the client. Every protected request is validated by calling `GET /api/v1/auth/me` on Aegis-DB with the bearer header. Ingestion keys (`prom_*` prefix) are validated locally against the `ingestion_keys` collection.

User lifecycle: signup -> email verification -> admin approval -> login (optional MFA via TOTP).

## API endpoint groups

All endpoints are under `/api/v1`. Public routes have no auth; protected routes require a valid bearer token.

| Group | Prefix | Description |
|---|---|---|
| **Auth** | `/auth/*` | Login, logout, session, signup, email verify, password reset, MFA validate (public) |
| **Datasets** | `/datasets/*` | Upload, list, get, delete, preview, validate, connect external sources, catalog browser |
| **Training** | `/training/*` | Start/stop runs, list, queue status, clear completed |
| **Models** | `/models/*` | List, get, delete, download (AxonML/ONNX/HEF), convert, compare |
| **Deployment** | `/deployments/*` | Create, list, get, download edge binary, list targets |
| **Agent** | `/agent/*` | Chat (proxied to DO Gradient AI), analyze, history |
| **Evaluation** | `/evaluations/*` | List, get, run Gradient-powered evaluation |
| **Billing** | `/billing/*` | Stripe checkout, portal, subscription status, usage, webhook (public) |
| **Email** | `/email/*` | Welcome, verification, password reset, support, security alert, daily report |
| **Profile** | `/profile/*` | User profile, preferences, token balance |
| **MFA** | `/mfa/*` | Setup, verify, disable TOTP-based MFA |
| **Admin** | `/admin/users/*` | List/create/update/delete/approve users (admin role only) |
| **Service accounts** | `/service-accounts/*` | Create/list/delete machine accounts (admin only) |
| **Connections** | `/connections/*` | Saved encrypted data source credentials |
| **Ingestion keys** | `/ingestion-keys/*` | Create/list/delete API keys for programmatic access |
| **Push** | `/push/register` | Register mobile push notification tokens |
| **System** | `/system/metrics` | Server metrics |
| **Health** | `/health` | Health check (no auth, no prefix) |
| **WebSocket** | `/ws/training/:id` | Real-time training progress stream |

## Running

```bash
# Load secrets from the credential vault
source vault/env.sh

# Build and run
cargo run -p prometheus-server

# Or with custom log level
RUST_LOG=debug cargo run -p prometheus-server
```

The server binds to `0.0.0.0:3030` by default and serves the Leptos WASM UI from `dist/` with an SPA fallback to `dist/index.html`.

## Environment variables

### Required

| Variable | Default | Description |
|---|---|---|
| `AEGIS_DB_URL` | `http://localhost:9091` | Aegis-DB base URL |

### Server

| Variable | Default | Description |
|---|---|---|
| `PROMETHEUS_HOST` | `0.0.0.0` | Bind address |
| `PROMETHEUS_PORT` | `3030` | Bind port |
| `PROMETHEUS_DATA_DIR` | `/tmp/prometheus-data` | Local storage for datasets, models, deployments |
| `PROMETHEUS_PUBLIC_URL` | -- | Public-facing URL (for email links) |
| `PROMETHEUS_MAX_TRAININGS` | CPU core count (min 2) | Max concurrent training runs |
| `RUST_LOG` | `info,prometheus_server=debug` | Tracing filter |

### DigitalOcean Gradient AI

| Variable | Description |
|---|---|
| `DO_GENAI_ACCESS_KEY` | GenAI API key (preferred over `GRADIENT_MODEL_ACCESS_KEY`) |
| `DO_GENAI_ENDPOINT` | Chat completions endpoint URL |
| `GRADIENT_AGENT_ID` | Agent ID |

### Stripe billing (optional, from vault)

| Variable | Description |
|---|---|
| `STRIPE_SECRET_KEY` | `sk_live_*` or `sk_test_*` |
| `STRIPE_WEBHOOK_SECRET` | `whsec_*` |
| `STRIPE_PRICE_BASIC` | Price ID for basic tier |
| `STRIPE_PRICE_PRO` | Price ID for pro tier |
| `STRIPE_PRICE_ENTERPRISE` | Price ID for enterprise tier |
| `STRIPE_METER_ID` | Meter ID for usage-based billing |
| `STRIPE_PRICE_OVERAGE` | Price ID for token overage |

### Email (optional)

| Variable | Description |
|---|---|
| `RESEND_API_KEY` | Resend API key (read by `prometheus-email` crate) |

All secrets should be managed through `vault/env.sh` (AES-256-GCM encrypted credential vault) rather than set directly.

## Dependencies

| Crate | Role |
|---|---|
| `prometheus-ui` | Leptos 0.7 WASM CSR frontend (compiled to `dist/`) |
| `prometheus-training` | AxonML-based training pipeline orchestrator |
| `prometheus-reports` | PDF report generation (headless Chrome) |
| `prometheus-shield` | Security engine: SQLi, SSRF, rate limiting, threat scoring |
| `prometheus-email` | Transactional email via Resend API |
| `aegis-client` | Aegis-DB client (document store + SQL + auth) |
| `axum` 0.7 | HTTP framework |
| `tower-http` | CORS, tracing, static file serving |
| `tokio` | Async runtime |
| `reqwest` | HTTP client for Aegis-DB and external API calls |
| `totp-rs` | TOTP-based MFA |
| `zstd` | Dataset compression for data lifecycle management |

## Aegis-DB collections

The server initializes these document collections on startup:

`datasets`, `models`, `training_plans`, `deployments`, `evaluations`, `agent_history`, `cli_sessions`, `subscriptions`, `user_status`, `email_verifications`, `password_resets`, `mfa_secrets`, `user_preferences`, `push_tokens`, `ingestion_keys`

Plus SQL tables: `prometheus_users`, `prometheus_audit`.

## Background tasks

- **Data lifecycle manager** -- compresses inactive datasets after 24h, frees storage after 30d retention.
