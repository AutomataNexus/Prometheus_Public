# Prometheus -- System Architecture

## Overview

Prometheus is an AI-powered general-purpose ML training orchestrator that trains models across 13 neural network architectures using AxonML 0.4.1 (pure Rust, GPU/CUDA support) with autograd-based real backpropagation and deploys them to edge devices. Each architecture implements the `TrainableModel` trait using AxonML's nn layers, optimizers (Adam/AdamW), and loss functions (MSELoss/BCELoss/CrossEntropyLoss). The system uses a 5-stage pipeline: Validate, Preprocess, Train, Evaluate, Export. Includes a CLI/TUI terminal interface with QR code authentication and OpenZL compression.

## High-Level Architecture

```
                    DigitalOcean Gradient AI Platform
                    +-------------------------------+
                    |  PrometheusForge Agent (ADK)  |
                    |  Knowledge Base (RAG)         |
                    |  Serverless Inference         |
                    +---------------+---------------+
                                    |
                    +---------------v---------------+
                    |     PROMETHEUS CORE SERVER     |
                    |     (Rust -- Axum + Leptos)    |
                    |                               |
                    |  REST API    WebSocket   WASM UI
                    |  (Axum)     (tokio-     (Leptos
                    |              tungstenite) CSR)
                    +-------+-----------+-----------+
                            |           |
                +-----------v-+   +-----v-----------+
                |  AEGIS-DB    |   |  AXONML ENGINE   |
                |  (Port 9091) |   |  (Pure Rust ML)  |
                |              |   |                   |
                |  SQL, Docs,  |   |  13 Architectures |
                |  TimeSeries, |   |  GPU/CUDA, Quant  |
                |  KV, Graph,  |   |  Serialization    |
                |  Streaming   |   +--------+----------+
                +--------------+            |
                                            | Cross-compile
                                            | armv7-musleabihf
                                            v
                                +-----------+-----------+
                                |   EDGE DEPLOYMENT     |
                                |   Raspberry Pi 4/5    |
                                |   Inference Daemon    |
                                |   (port 6200)         |
                                +-----------------------+
```

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `prometheus-server` | Main Axum HTTP server, REST API, WebSocket, CLI auth endpoints |
| `prometheus-ui` | Leptos WASM reactive UI (compiled to WebAssembly) |
| `prometheus-training` | AxonML training pipeline orchestrator (13 architectures) |
| `prometheus-edge` | Edge inference daemon for Raspberry Pi |
| `prometheus-reports` | PDF report generation via headless Chrome |
| `prometheus-shield` | Adaptive zero-trust security engine (fingerprinting, rate limiting) |
| `prometheus-email` | Transactional email (training alerts, queue notifications, deployment notifications) |
| `prometheus-cli` | CLI/TUI terminal interface (Clap + Ratatui) |
| `prometheus-agent` | Gradient AI ADK agent (Python) |

## Data Ingestion Architecture

Prometheus supports a two-tier data ingestion architecture, combining native edge connectivity with external data source imports.

### Tier 1: Aegis-to-Aegis via AegisControlBridge (Default)

The native/default data path for edge controller data. Each edge controller (Raspberry Pi) runs Aegis-DB locally. AegisControlBridge continuously syncs data from the edge Aegis-DB to the cloud Prometheus Aegis-DB instance.

```
  Raspberry Pi (Edge)                         Prometheus (Cloud)
  +---------------------+                    +---------------------+
  | Hardware Daemon      |                    |                     |
  | (port 6100)          |                    |                     |
  |   |                  |                    |                     |
  |   v                  |                    |                     |
  | Aegis-DB (edge)      |   Aegis-to-Aegis  | Aegis-DB (cloud)    |
  | (port 9090)          |<=================>| (port 9091)         |
  |   ^                  |  AegisControlBridge|                     |
  |   |                  |                    |                     |
  | Equipment Executors  |                    | Prometheus Server   |
  +---------------------+                    +---------------------+
```

**Data flow:**
1. Hardware Daemon polls I2C sensors every 1 second
2. Equipment Executors read sensor data, run control logic, write outputs
3. aegisControlBridge reads metrics from edge Aegis-DB
4. aegisControlBridge writes metrics to cloud Aegis-DB
5. Prometheus `connect_source(source_type: "aegis_bridge")` pulls data directly from any edge controller's Aegis-DB

### Tier 2: External Data Sources

Users can import data from 6 external database types. All external data is normalized to CSV internally and stored as a standard Prometheus dataset.

```
  External Sources                           Prometheus Server
  +-------------------+                     +-------------------+
  | InfluxDB (v3 SQL) |--+                  |                   |
  +-------------------+  |                  |  connect_source   |
  | PostgreSQL        |--+                  |  dispatcher       |
  +-------------------+  |    HTTP/REST     |       |           |
  | TiDB              |--+================>|       v           |
  +-------------------+  |                  |  Normalize to CSV |
  | SQLite3           |--+                  |       |           |
  +-------------------+  |                  |       v           |
  | MongoDB Data API  |--+                  |  Store as dataset |
  +-------------------+  |                  |  in Aegis-DB      |
  | SpaceTimeDB       |--+                  |                   |
  +-------------------+                     +-------------------+
```

**Supported external sources:**

| Source | Protocol | Connection Method |
|--------|----------|-------------------|
| InfluxDB | HTTP | InfluxDB v3 SQL API (`/api/v3/query_sql`) |
| PostgreSQL | HTTP | Aegis-DB federated query or direct REST API |
| TiDB | HTTP | Aegis-DB federated query or direct REST API |
| SQLite3 | File | Direct file access on server |
| MongoDB | HTTP | MongoDB Data API (`/action/find`) |
| SpaceTimeDB | HTTP | SpaceTimeDB SQL endpoint (`/database/{db}/sql`) |

### CSV Upload

In addition to the two programmatic tiers, users can always upload CSV files directly through the web UI or API (`POST /api/v1/datasets` with `multipart/form-data`).

## Data Flow

1. **Ingest** -- Operator uploads CSV sensor data, connects an edge controller via AegisControlBridge, or imports from an external database (InfluxDB, PostgreSQL, TiDB, SQLite3, MongoDB, SpaceTimeDB). Data is normalized to CSV, statistics are computed, and metadata is stored in Aegis-DB.

2. **Validate & Lock** -- After ingestion, datasets go through a validation phase. Prometheus scans all rows for type consistency, missing values, and schema issues. Validated datasets are "locked" (frozen) and auto-compressed with zstd for storage efficiency. Locked datasets cannot be modified without unlocking (which requires re-validation). Training refuses to start on unvalidated datasets.

3. **Analyze** -- The PrometheusForge Gradient AI agent analyzes data patterns, detects seasonality, identifies anomalies, and recommends one of 13 model architectures. PrometheusForge provides guided model recommendations by analyzing column types (time-series, text, image, categorical), ranking model architectures by match score, and showing expected inputs, outputs, and inference results. Users can click to create a model, name it, and start training.
   - **Time-Series**: LSTM Autoencoder, GRU Predictor, RNN, Sentinel MLP, Conv1d
   - **Vision**: Conv2d, VGG-11/16, ResNet, ViT
   - **NLP**: BERT (encoder), GPT-2 (decoder)
   - **Specialized**: Nexus (multi-modal fusion), Phantom (edge-optimized)

4. **Train** -- The AxonML 0.4.1 autograd engine trains the selected architecture using real backpropagation (single forward + backward pass per batch, not numerical gradients). Each architecture implements the `TrainableModel` trait (`forward(&Variable) -> Variable`, `parameters() -> Vec<Parameter>`) composed from AxonML nn layers (Linear, LSTM, GRU, RNN, Conv1d, Conv2d, BatchNorm2d, TransformerEncoder, TransformerDecoder, ResidualBlock, MultiHeadAttention, CrossAttention, Sequential). Training uses AxonML's built-in optimizers (Adam, AdamW with weight_decay) and loss functions (MSELoss, BCELoss, CrossEntropyLoss). The training pipeline runs 5 stages: validate -> preprocess -> train -> evaluate -> export. Live metrics (loss, accuracy) stream to the UI via WebSocket. Training runs are stored in Aegis-DB. Training runs are managed with a server-wide concurrency limit (configurable via `PROMETHEUS_MAX_TRAININGS`, defaults to CPU core count). When all slots are occupied, new training requests are queued (FIFO). When a slot opens, the next queued run starts automatically and the user is notified via push notification and email (if enabled in preferences).

   **AxonML Performance Optimizations:**
   - **Rayon-parallel Conv1d/Conv2d**: Both forward and backward passes parallelize across batch samples via Rayon
   - **Fused im2col**: 5-level nested loops collapsed to 3-level with arithmetic decomposition and bounds-eliminated inner loops (`unsafe get_unchecked`)
   - **CUDA full-GPU pipeline**: Conv2d backward uses GPU-resident im2col → cuBLAS GEMM, auto-activates when tensors are on device
   - **Hoisted RNN/LSTM/GRU weight transposes**: Weight transposes pre-computed once before the per-timestep loop instead of every timestep; input-hidden projection computed as a single batched GEMM
   - **matrixmultiply threading**: All CPU BLAS GEMM operations use threaded matrixmultiply via Rayon
   - **Benchmark**: 8.3 → 14.2 img/s on CPU (71% speedup), 1,018 tests passing (105 autograd + 171 nn + 704 vision + 38 HVAC + 83 LLM)

5. **Evaluate** -- Trained models are evaluated with standard ML metrics (precision, recall, F1, AUC-ROC) plus Gradient AI's 19-metric evaluation suite. Results include confusion matrices and training curves.

6. **Deploy** -- Models are quantized to INT8 via `axonml-quant`, cross-compiled to ARM, and packaged as a static binary for Raspberry Pi. The binary can be downloaded or deployed directly to registered edge controllers.

## Authentication & User Lifecycle

Prometheus implements a complete auth system powered entirely by Aegis-DB (no Firebase or external auth providers):

- **Password hashing**: Aegis-DB handles Argon2id hashing automatically on user creation
- **Session tokens**: Opaque bearer tokens (NOT JWT), validated via `GET /api/v1/auth/me` on Aegis-DB
- **Roles**: `admin`, `operator`, `viewer`
- **Rate limiting**: 30 login attempts/min per IP, 1000 API requests/min per user

### User Lifecycle

```
Signup → Email Verification → Login → (MFA if enabled) → Session Token
   |         |                   |
   |    emailPending status      |
   |    (6-digit code,           |
   |     15-min expiry)          |
   |                             |
   +-→ Forgot Password → Reset Token (30-min expiry) → New Password
```

**Aegis-DB Collections:**
- `datasets` -- Dataset metadata, schema, and statistics (includes `is_validated` and `locked` fields)
- `users` -- User accounts (managed by Aegis-DB admin API)
- `user_status` -- Email verification status per user
- `email_verifications` -- Pending verification codes (6-digit, 15-min TTL)
- `password_resets` -- Password reset tokens (UUID, 30-min TTL)
- `mfa_secrets` -- TOTP secrets for multi-factor authentication
- `user_preferences` -- Theme, notification, and display preferences
- `push_tokens` -- Expo push notification device tokens
- `subscriptions` -- Stripe subscription state and token balances

### MFA (TOTP)

Optional TOTP-based 2FA. When enabled, login returns `mfa_required: true` and the client must validate a TOTP code before the session token is usable.

## Billing & Subscription System

Stripe-powered subscription management with server-side enforcement.

```
User → Checkout (Stripe) → Webhook → Subscription Created → Tier Limits Enforced
                                |
                    checkout.session.completed
                    customer.subscription.updated
                    customer.subscription.deleted
```

**Tiers:** Free (default), Pro ($49/mo), Enterprise ($199/mo)

All resource limits (datasets, models, trainings, deployments, dataset size, tokens) are enforced at the API handler level before any operation proceeds. Each mutating endpoint checks the user's tier via `get_user_tier()` and calls `enforce_limit()`.

Webhook payloads are verified using HMAC-SHA256 signature comparison against `STRIPE_WEBHOOK_SECRET`.

## Push Notifications

The server sends push notifications via the Expo Push API for key events:

| Event | Channel | Priority |
|-------|---------|----------|
| Training queued | `training` | Default |
| Training started | `training` | High |
| Training complete | `training` | High |
| Training failed | `training` | High |
| Epoch milestone | `training` | High |
| Deployment ready | `alerts` | Max |
| Security alert | `alerts` | Max |
| Account verified | `default` | Default |
| Subscription changed | `default` | Default |

Push tokens are registered per-device via `POST /push/register` and stored in the `push_tokens` collection. Notifications include typed data payloads that the mobile app uses for deep-link navigation.

## Mobile App

An Expo SDK 55 (React Native) mobile app provides iOS and Android access:

- **EAS Build**: Cloud-native builds for iOS (App Store / TestFlight) and Android (Play Store)
- **EAS Update**: OTA updates pushed to channels (development, preview, production) without App Store resubmission
- **Push notifications**: Expo Push with typed handlers and navigation on tap
- **Auth**: Same Aegis-DB bearer token flow as the web UI

## Networking

| Service | Port | Protocol |
|---------|------|----------|
| Prometheus Server | 3030 | HTTP + WebSocket |
| Aegis-DB (cloud) | 9091 | HTTP |
| Aegis-DB (edge) | 9090 | HTTP |
| Edge Inference Daemon | 6200 | HTTP |
| NexusEdge Hardware Daemon | 6100 | HTTP |

## CLI / TUI Architecture

The `prometheus-cli` crate provides a full terminal interface:

```
prometheus (binary)
├── Clap Parser          # 18 subcommands + interactive REPL mode
├── Interactive REPL     # 22+ slash commands, non-slash -> PrometheusForge agent
├── Auth Module          # QR code + URL browser-based auth
│   ├── POST /api/v1/auth/cli/init    → session_code + auth_url
│   ├── GET  /api/v1/auth/cli/poll    → pending/authenticated
│   └── POST /api/v1/auth/cli/verify  → link token to session
├── OpenZL Compression   # .ozl format: zstd + SHA-256 + 54-byte header
│   ├── compress_file()  → OZL\x01 magic + hash + compressed data
│   ├── decompress_file()→ verify hash + decompress
│   └── train_dictionary()→ zstd dict from samples
├── TUI (Ratatui)        # NexusEdge branded terminal UI
│   ├── Dashboard View   → training runs table + models list + stats
│   └── Monitor View     → live loss chart (Braille) + epoch metrics
└── API Client           # Bearer token auth, JSON API helpers
```

## Model Architectures (13)

All architectures implement the `TrainableModel` trait (`forward(&Variable) -> Variable`, `parameters() -> Vec<Parameter>`) and are trained via AxonML's autograd engine with real backpropagation.

| Architecture | Category | Parameters | Output | AxonML Layer Composition |
|-------------|----------|-----------|--------|--------------------------|
| LSTM Autoencoder | Time-Series | 32K-128K | Reconstruction error | LSTM (encoder) + Linear (latent) + LSTM (decoder) + Linear (output) |
| GRU Predictor | Time-Series | 64K-192K | 3 horizon probs | GRU (stacked) + Linear (head) |
| Sentinel MLP | Time-Series | 16K-64K | 0.0-1.0 score | Sequential(Linear + Linear + Linear) |
| RNN | Time-Series | 32K-96K | Class probs | RNN (stacked) + Linear (classifier) |
| Conv1d | Time-Series | 16K-64K | Sigmoid score | Conv1d + Conv1d + Linear |
| Conv2d | Vision | 64K-512K | Softmax classes | Conv2d + BatchNorm2d + Conv2d + BatchNorm2d + Linear |
| VGG (11/16) | Vision | 512K-4M | Softmax classes | Sequential(Conv2d + BatchNorm2d) stacked + Linear (classifier) |
| ResNet | Vision | 256K-2M | Softmax classes | Conv2d + ResidualBlock (stacked) + Linear |
| ViT | Vision | 128K-1M | Softmax classes | Linear (patch embed) + TransformerEncoder (stacked) + Linear |
| BERT | NLP | 256K-2M | Softmax classes | Linear (embed) + TransformerEncoder (stacked, non-causal) + Linear |
| GPT-2 | NLP | 256K-2M | Softmax tokens | Linear (embed) + TransformerDecoder (stacked, causal) + Linear |
| Nexus | Specialized | 64K-256K | Multi-head | Linear (per-modality) + CrossAttention + MultiHeadAttention + Linear |
| Phantom | Specialized | 32K-128K | Sigmoid score | Sequential(Linear + Linear + Linear), edge-optimized |

## External Dependencies

- **AxonML 0.4.1** (`/opt/AxonML`) -- Pure Rust ML framework with autograd engine, nn layers, optimizers, loss functions, GPU/CUDA support, Rayon-parallel convolutions, fused im2col, and hoisted RNN/LSTM/GRU weight transposes (1,018 tests passing)
- **Aegis-DB** (`/opt/Aegis-DB`) -- Multi-paradigm database for all data storage
- **DigitalOcean Gradient AI** -- Agent hosting, knowledge base, serverless inference
