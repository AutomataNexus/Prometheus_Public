<p align="center">
  <img src="./Prometheus_logo.png" alt="Prometheus -- AI-Forged Edge Intelligence" width="400">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Leptos-0.7-blue?logo=webassembly" alt="Leptos">
  <img src="https://img.shields.io/badge/Axum-0.7-blueviolet" alt="Axum">
  <img src="https://img.shields.io/badge/AxonML_0.4.1-Autograd%2FGPU-red" alt="AxonML">
  <img src="https://img.shields.io/badge/Aegis--DB-Multi--Paradigm-teal" alt="Aegis-DB">
  <img src="https://img.shields.io/badge/Gradient_AI-ADK-0069ff?logo=digitalocean" alt="Gradient AI">
  <img src="https://img.shields.io/badge/Tests-1018_total-45ba4b" alt="Tests">
  <img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-green" alt="License">
  <img src="https://img.shields.io/badge/DigitalOcean-Hackathon-0080FF?logo=digitalocean" alt="DigitalOcean">
</p>

<p align="center">
  A DigitalOcean Gradient AI agent that automatically analyzes data, selects from 13 neural network architectures,<br>
  trains ML models via AxonML 0.4.1 (pure Rust, autograd backpropagation, GPU/CUDA), converts to ONNX/HEF, and deploys to edge devices --<br>
  LSTM/GRU/RNN models run directly on Raspberry Pi (~1.8 MB RSS), CNN/MLP models accelerate via Hailo-8 NPU.<br>
  A general-purpose cloud-to-edge ML training orchestrator with CLI/TUI, OpenZL compression, and full-stack Rust.
</p>

---

<details open>
<summary><strong>Web UI Demo</strong></summary>
<p align="center">
  <img src="./assets/ui-navigation.gif" alt="Prometheus UI Navigation" width="720">
  <br>
  <em>Full UI walkthrough — Landing, Login, Datasets, Agent, Training, Monitor, Models, Evaluation, Convert, Quantize, Deployment, Billing, Admin</em>
</p>
</details>

<details>
<summary><strong>CLI Demo</strong></summary>
<p align="center">
  <img src="./assets/cli-demo.gif" alt="Prometheus CLI" width="720">
  <br>
  <em>CLI — Auth, datasets, models, training, agent chat, REPL with /commands, OpenZL compression</em>
</p>
</details>

<details>
<summary><strong>TUI Demo</strong></summary>
<p align="center">
  <img src="./assets/tui-demo.gif" alt="Prometheus TUI" width="720">
  <br>
  <em>TUI — 8 tabs: Dashboard, Datasets, Models, Monitor, Agent, Convert, Quantize, Deploy</em>
</p>
</details>

<p align="center">
  <strong>Live Demo:</strong> <a href="https://prometheus.automatanexus.com">prometheus.automatanexus.com</a>
</p>

---

## The Problem

Training and deploying ML models requires stitching together Python scripts, cloud GPU instances ($10K+/month), Docker containers, conversion tools, and edge deployment infrastructure. Most teams spend 80% of their time on infrastructure and 20% on the actual model. There is no unified platform that takes raw data → trained model → edge-deployed inference in a single workflow.

## The Solution

Prometheus is a **full-stack Rust ML training orchestrator** powered by **DigitalOcean Gradient AI**. Upload any dataset, get AI-powered architecture recommendations from PrometheusForge (our Gradient AI agent with RAG knowledge base), train across 13 neural network architectures via AxonML, quantize (Q8/Q4/F16), convert to ONNX/HEF, and deploy to edge devices with Hailo-8 NPU acceleration — all from one platform.

Prometheus closes this gap with a **6-stage AI pipeline**:

```
+-------------+  +--------------+  +-------------+  +--------------+  +--------------+  +-------------+
| 1. INGEST   |->| 2. ANALYZE   |->| 3. TRAIN    |->| 4. EVALUATE  |->| 5. CONVERT   |->| 6. DEPLOY   |
|             |  |              |  |             |  |              |  |              |  |             |
| Upload CSV, |  | Gradient AI  |  | AxonML      |  | Validation   |  | Export to    |  | Push to any |
| connect     |  | Agent picks  |  | trains 13   |  | metrics,     |  | ONNX or HEF  |  | edge device |
| edge via    |  | from 13 arch |  | arch types  |  | Gradient     |  | (Hailo-8/8L) |  | with Hailo  |
| Aegis, or   |  | types &      |  | on DO infra |  | AI powered   |  | for hardware |  | NPU accel   |
| import from |  | tunes hyper- |  | (GPU/CUDA)  |  | evaluations  |  | accelerators |  | inference   |
| external DB |  | params       |  |             |  |              |  |              |  |             |
+-------------+  +--------------+  +-------------+  +--------------+  +--------------+  +-------------+
```

**Key differentiators:**
- **Full-stack Rust** -- UI (Leptos/WASM), ML framework (AxonML), database (Aegis-DB), edge inference
- **Cloud-to-edge pipeline** -- DigitalOcean Gradient AI -> AxonML training -> ONNX/HEF conversion -> deploy to any edge device (LSTM/GRU/RNN run natively on Pi at ~1.8 MB RSS, CNN/MLP accelerate via Hailo-8 NPU)
- **Multi-source data ingestion** -- Native Aegis-to-Aegis edge connectivity + InfluxDB, PostgreSQL, TiDB, SQLite3, MongoDB, SpaceTimeDB
- **Real-world proven** -- Architecture validated with 29 deployed models across 5 commercial buildings
- **Tiny models** -- 32K-288K parameters, ~1.5-3.2 MB RSS, running on $50 hardware

---

## Architecture

```
                        DigitalOcean Gradient AI Platform
                    +--------------------------------------+
                    |  PrometheusForge Agent (ADK)            |
                    |  - Analyze sensor data patterns       |
                    |  - Select model architecture          |
                    |  - Tune hyperparameters               |
                    |  - Evaluate results                   |
                    +--------------+-----------------------+
                                   |
        +--------------------------+---------------------------+
        |                 Prometheus Server                     |
        |                                                      |
        |  +------------+  +--------------+  +-------------+   |
        |  | Leptos UI  |  | Axum REST    |  | WebSocket   |   |
        |  | (WASM CSR) |  | API + Auth   |  | Live Train  |   |
        |  +-----+------+  +------+-------+  +------+------+   |
        |        |                |                  |          |
        |  +-----+----------------+------------------+-------+  |
        |  |              Core Services                       |  |
        |  |  +----------+  +----------+  +----------+  +-------+  |  |
        |  |  | Training |  | Convert  |  | Reports  |  | Deploy|  |  |
        |  |  | Pipeline |  | ONNX/HEF |  | (PDF)    |  | Edge  |  |  |
        |  |  +----+-----+  +----+-----+  +----+-----+  +---+---+  |  |
        |  +-------+-----------------------------+------------+  |
        +----------+-----------+---+-----------+-+--------------+
                   |           |   |           |
        +----------+------+    |   |   +-------+---------------+
        |   Aegis-DB      |    |   |   |  Edge Device             |
        |  - Documents    |    |   |   |  (RPi5, any Hailo host)  |
        |  - Time Series  |    |   |   |                          |
        |  - Auth / KV    |    |   |   |  prometheus-edge          |
        |  - SQL Tables   |    |   |   |  + Hailo-8/8L NPU         |
        +-----------------+    |   |   +--------------------------+
                               |   |
        +----------------------+   +-------------------------+
        |    Data Source Connectors                           |
        |                                                     |
        |  +------+ +------+ +------+ +-----+ +-----+ +----+ |
        |  |Aegis | |Influx| |Postgr| |TiDB | |Mongo| |STDB| |
        |  |Bridge| |DB    | |eSQL  | |     | |DB   | |    | |
        |  +------+ +------+ +------+ +-----+ +-----+ +----+ |
        +-----------------------------------------------------+
```

---

## Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Frontend** | Leptos 0.7 (WASM CSR) | Reactive SPA with NexusEdge design system |
| **Backend** | Axum 0.7, Tokio 1.35 | Async HTTP server, REST API, WebSocket |
| **ML Framework** | AxonML 0.4.1 (pure Rust, GPU/CUDA) | Autograd engine with real backpropagation, 13 architectures via `TrainableModel` trait, nn layers (Linear, LSTM, GRU, Conv1d/2d, Transformer, etc.), built-in optimizers (Adam/AdamW) and loss functions (MSELoss, BCELoss, CrossEntropyLoss). Rayon-parallel Conv1d/Conv2d, fused im2col, CUDA full-GPU pipeline, hoisted RNN/LSTM/GRU weight transposes — 71% CPU speedup (8.3 → 14.2 img/s). 1,018 tests passing. |
| **CLI/TUI** | Clap 4 + Ratatui 0.28 | Terminal interface with QR auth, training monitor, OpenZL compression |
| **Database** | Aegis-DB | Multi-paradigm: documents, time-series, SQL |
| **AI Agent** | Gradient AI ADK (Python) | Architecture selection, evaluation, chat |
| **Model Export** | Python converter (PyTorch, ONNX, Hailo DFC) | .axonml -> ONNX / HEF conversion for all 13 architectures |
| **Edge Runtime** | Custom Rust daemon (tiny_http) | Inference on any edge device with Hailo-8/8L NPU |
| **PDF Reports** | headless_chrome 1.0 | Training reports, deployment certificates |
| **E2E Testing** | Playwright + Puppeteer | Browser testing, visual regression, PDF validation |
| **Styling** | Tailwind CSS 4.1 | NexusEdge design system |
| **Auth** | Argon2id + opaque bearer tokens | Full user lifecycle via Aegis-DB (signup, verification, MFA) |
| **Billing** | Stripe | Subscription tiers (Free/Pro/Enterprise) with server-side enforcement |
| **Mobile** | Expo SDK 55 (React Native) | iOS + Android app with push notifications and OTA updates |
| **Email** | Resend (transactional) | Verification codes, password resets, training alerts |
| **Security** | prometheus-shield | Adaptive zero-trust request validation |

---

## Data Ingestion

Prometheus supports three data ingestion methods with a two-tier architecture:

### Tier 1: Aegis-to-Aegis (Default / Native)

The native data path for edge controllers. Each Raspberry Pi runs Aegis-DB locally; **AegisControlBridge** continuously syncs data from edge to cloud.

```
  Hardware Daemon (port 6100)
       |
  Aegis-DB (edge, port 9090)
       |
  AegisControlBridge  <==>  Aegis-DB (cloud, port 9091)
                                     |
                              Prometheus Server
```

### Tier 2: External Sources

Import from any of 6 supported external databases:

| Source | Protocol | Description |
|--------|----------|-------------|
| **InfluxDB** | HTTP (v3 SQL API) | Time-series data from InfluxDB instances |
| **PostgreSQL** | HTTP (Aegis federated query) | Relational sensor data |
| **TiDB** | HTTP (Aegis federated query) | Distributed SQL sensor data |
| **SQLite3** | File access | Local database files |
| **MongoDB** | HTTP (Data API) | Document-based sensor data |
| **SpaceTimeDB** | HTTP (SQL endpoint) | SpaceTimeDB sensor tables |

### CSV Upload

Direct CSV upload via web UI or REST API (`POST /api/v1/datasets`).

All data sources are normalized to CSV internally and stored as standard Prometheus datasets in Aegis-DB.

---

## Project Structure

```
Prometheus/
+-- Cargo.toml                    # Workspace root -- 8 Rust crates
+-- README.md
+-- PRD.md                        # Product Requirements Document
+-- Dockerfile                    # Multi-stage production build (6 stages)
+-- docker-compose.yml            # prometheus-server + aegis-db
+-- tailwind.config.js            # NexusEdge theme configuration
+-- input.css                     # Tailwind input with 30+ custom components
+-- LICENSE-MIT / LICENSE-APACHE
|
+-- crates/
|   +-- prometheus-server/        # Axum HTTP server (25+ source files)
|   |   +-- src/
|   |       +-- main.rs           # Entry point -- binds 0.0.0.0:3030
|   |       +-- config.rs         # Env-based config
|   |       +-- state.rs          # AppState -- HTTP client, training handles
|   |       +-- router.rs         # 50+ REST routes + WebSocket + SPA fallback
|   |       +-- error.rs          # AppError -> HTTP status code mapping
|   |       +-- auth/
|   |       |   +-- middleware.rs  # Bearer token validation via Aegis-DB
|   |       |   +-- models.rs     # Role enum, LoginRequest, AuthUser
|   |       +-- api/
|   |       |   +-- health.rs     # GET /health + system metrics
|   |       |   +-- datasets.rs   # CRUD + upload + multi-source connect
|   |       |   +-- training.rs   # Start/stop/status training runs
|   |       |   +-- models.rs     # List/detail/download/compare models
|   |       |   +-- deployment.rs # Deploy to edge targets
|   |       |   +-- evaluation.rs # Metrics + Gradient AI evaluation
|   |       |   +-- agents.rs     # Chat + analyze via Gradient AI
|   |       |   +-- users.rs      # Signup, email verification, password reset, admin CRUD
|   |       |   +-- billing.rs    # Stripe subscriptions, checkout, webhooks, usage
|   |       |   +-- push.rs       # Expo push notification delivery
|   |       |   +-- mfa.rs        # TOTP-based multi-factor authentication
|   |       |   +-- profile.rs    # User profile + preferences
|   |       |   +-- email.rs      # Transactional email triggers
|   |       +-- ws/
|   |           +-- mod.rs        # WebSocket live training progress
|   |
|   +-- prometheus-ui/            # Leptos WASM frontend (34 source files)
|   |   +-- src/
|   |       +-- app.rs            # Leptos router -- 12 routes
|   |       +-- theme.rs          # NexusEdge color constants + global CSS
|   |       +-- icons/mod.rs      # 25+ inline Lucide SVG icons
|   |       +-- components/       # 16 reusable UI components
|   |       |   +-- layout.rs     # AppShell (sidebar + header + content)
|   |       |   +-- sidebar.rs    # Navigation with active route state
|   |       |   +-- card.rs       # Card container
|   |       |   +-- button.rs     # Primary / Ghost / Danger variants
|   |       |   +-- input.rs      # TextInput, TextArea, SelectInput
|   |       |   +-- modal.rs      # Modal + ConfirmModal (CSS toggle)
|   |       |   +-- table.rs      # DataTable with pagination
|   |       |   +-- chart.rs      # LineChart + BarChart (SVG)
|   |       |   +-- metric_card.rs# Dashboard metric with trend
|   |       |   +-- badge.rs      # 7 status variants
|   |       |   +-- toast.rs      # Toast notification system
|   |       |   +-- loader.rs     # Spinner, Skeleton, PageLoader
|   |       |   +-- file_upload.rs# Drag-and-drop CSV upload
|   |       |   +-- code_block.rs # Syntax-highlighted code display
|   |       |   +-- header.rs     # Top bar with user + actions
|   |       |   +-- icon.rs       # Dynamic icon renderer
|   |       +-- pages/            # 12 page components
|   |           +-- home.rs       # Dashboard -- metrics + pipeline viz
|   |           +-- login.rs      # Authentication form
|   |           +-- datasets.rs   # Upload + manage + connect sources
|   |           +-- dataset_detail.rs
|   |           +-- training.rs   # Start + monitor training runs
|   |           +-- training_detail.rs # Live loss chart via WebSocket
|   |           +-- models.rs     # Model gallery + comparison
|   |           +-- model_detail.rs
|   |           +-- deployment.rs # Edge deployment management
|   |           +-- evaluation.rs # Metrics + confusion matrix
|   |           +-- agent.rs      # PrometheusForge AI chat interface
|   |           +-- settings.rs   # API keys, system config
|   |
|   +-- prometheus-training/      # AxonML training pipeline (18 source files)
|   |   +-- src/
|   |       +-- lib.rs            # Public API + re-exports
|   |       +-- pipeline.rs       # 5-stage async pipeline orchestrator
|   |       +-- preprocessor.rs   # CSV loading, z-score normalization, sequencing
|   |       +-- architectures/
|   |       |   +-- mod.rs              # Architecture enum (13 variants) + builder
|   |       |   +-- lstm_autoencoder.rs # LSTM encoder-decoder (anomaly)
|   |       |   +-- gru_predictor.rs    # Multi-horizon GRU (prediction)
|   |       |   +-- sentinel.rs         # MLP health scorer (0.0-1.0)
|   |       |   +-- rnn_model.rs        # Vanilla RNN for sequence tasks
|   |       |   +-- conv_models.rs      # Conv1d + Conv2d models
|   |       |   +-- vgg_model.rs        # VGG-11/16 image classification
|   |       |   +-- resnet_model.rs     # ResNet with skip connections
|   |       |   +-- nlp_models.rs       # BERT + GPT-2 NLP architectures
|   |       |   +-- advanced_models.rs  # ViT, Nexus, Phantom models
|   |       +-- metrics.rs        # Precision, recall, F1, AUC-ROC, confusion matrix
|   |       +-- export.rs         # .axonml binary format + INT8 quantization
|   |       +-- cross_compile.rs  # ARM cross-compilation toolchain
|   |
|   +-- prometheus-edge/          # Edge inference daemon (4 source files)
|   |   +-- src/
|   |       +-- main.rs           # tiny_http server on port 6200
|   |       +-- inference.rs      # .axonml model loader + forward pass
|   |       +-- sensor_poll.rs    # Sensor polling client
|   |       +-- config.rs         # Per-unit TOML/JSON configuration
|   |
|   +-- prometheus-reports/       # PDF report generation (3 source files)
|   |   +-- src/
|   |       +-- lib.rs            # Chrome launcher, HTML->PDF pipeline
|   |       +-- training_report.rs# Training summary with loss curves
|   |       +-- deployment_cert.rs# Deployment certificate
|   |
|   +-- prometheus-shield/        # Adaptive zero-trust security engine
|   |   +-- src/                  # 11 source files
|   |
|   +-- prometheus-email/         # Transactional email (training alerts, reports)
|   |   +-- src/
|   |       +-- lib.rs            # Email templates + SMTP sender
|   |
|   +-- prometheus-cli/           # CLI/TUI terminal interface (15 source files)
|   |   +-- src/
|   |       +-- main.rs           # Clap CLI + interactive REPL (22 slash commands)
|   |       +-- theme.rs          # NexusEdge ANSI true-color theme
|   |       +-- config.rs         # ~/.prometheus/ config + credentials
|   |       +-- api.rs            # HTTP API client with Bearer auth
|   |       +-- auth.rs           # QR-code + URL-based browser auth flow
|   |       +-- compression.rs    # OpenZL format (.ozl) with zstd + SHA-256
|   |       +-- commands/         # CLI command handlers (6 files)
|   |       +-- tui/              # Ratatui TUI (dashboard + training monitor)
|   |
|   +-- prometheus-agent/         # Python Gradient AI ADK agent
|       +-- main.py               # ADK entrypoint
|       +-- config.yaml           # Agent config (model, tools, intents)
|       +-- requirements.txt      # gradient-adk, langchain, pandas, numpy
|       +-- .env.example          # Environment variable template
|       +-- agents/
|       |   +-- athena.py         # Orchestrator agent
|       |   +-- architect.py      # Architecture selection sub-agent
|       |   +-- data_analyst.py   # Data profiling sub-agent
|       |   +-- evaluator.py      # Model evaluation sub-agent
|       +-- tools/
|       |   +-- sensor_analysis.py
|       |   +-- architecture_db.py
|       |   +-- training_trigger.py
|       |   +-- model_evaluator.py
|       +-- knowledge/
|           +-- hvac_fault_catalog.md
|           +-- equipment_specs.md
|           +-- axonml_reference.md
|           +-- training_history.md
|
+-- mobile/
|   +-- prometheus-mobile/        # Expo SDK 55 React Native app
|       +-- app/                  # Expo Router screens (tabs layout)
|       +-- src/
|       |   +-- api/client.ts     # API client with auth
|       |   +-- hooks/
|       |       +-- usePushNotifications.ts  # Expo push + navigation
|       |       +-- useUpdates.ts            # OTA update checker
|       +-- app.json              # Expo config + EAS project ID
|       +-- eas.json              # EAS build profiles (dev/preview/prod)
|
+-- tests/
|   +-- e2e/                      # Playwright browser tests
|   |   +-- playwright.config.ts
|   |   +-- package.json
|   |   +-- fixtures/             # Auth helpers, sample CSVs
|   |   +-- specs/                # 15 spec files, 204 test cases
|   +-- puppeteer/                # PDF + visual regression tests
|   |   +-- package.json
|   |   +-- *.test.js             # 5 test files, 63 test cases
|   +-- integration/              # Rust API integration tests
|       +-- *.rs                  # 8 files, 123 test functions
|
+-- docs/
|   +-- ARCHITECTURE.md
|   +-- API_REFERENCE.md
|   +-- DEPLOYMENT.md
|   +-- GRADIENT_SETUP.md
|   +-- CONTRIBUTING.md
|
+-- assets/
    +-- logo.png
```

**100+ Rust source files** across 8 crates + **19 Python files** for the Gradient AI agent + **React Native mobile app**.

---

## Quick Start

### Prerequisites

- **Rust 1.75+** with `wasm32-unknown-unknown` target -- [rustup.rs](https://rustup.rs)
- **Node.js 20+** -- for Tailwind CSS build and E2E tests
- **Python 3.10+** -- for the Gradient AI agent (optional)
- **Chromium** -- for PDF report generation (optional)

### Option 1: Docker Compose (Recommended)

```bash
git clone https://github.com/AutomataNexus/Prometheus.git
cd Prometheus

# Configure environment
export AEGIS_DB_PASSWORD=your_secure_password
export GRADIENT_MODEL_ACCESS_KEY=your_gradient_key    # optional
export GRADIENT_AGENT_ID=your_agent_id                # optional

# Build and start (prometheus-server + aegis-db)
docker compose up -d --build

# Verify
curl http://localhost:3030/health
# {"status": "ok", "version": "0.1.0"}
```

Services started:
| Service | Port | Description |
|---------|------|-------------|
| prometheus-server | 3030 | Web UI + REST API + WebSocket |
| aegis-db | 9091 | Multi-paradigm database |

### Option 2: Manual Build

```bash
# Terminal 1 -- Start Aegis-DB
cd /opt/Aegis-DB && cargo run --release

# Terminal 2 -- Build and run Prometheus
cd /opt/Prometheus
rustup target add wasm32-unknown-unknown
cargo build --release --bin prometheus-server
./target/release/prometheus-server
```

The application will be available at **http://localhost:3030**.

---

## 6-Stage Pipeline Walkthrough

### Stage 1: Ingest

Bring data into Prometheus via any of three methods:

**CSV Upload** -- Upload sensor data directly through the web UI or API:
```bash
curl -X POST http://localhost:3030/api/v1/datasets \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@sensor_data.csv" \
  -F "name=Warren AHU-1" \
  -F "equipment_type=air_handler"
```

**Aegis-to-Aegis (default edge path)** -- Connect directly to an edge controller's Aegis-DB via AegisControlBridge:
```bash
curl -X POST http://localhost:3030/api/v1/datasets/connect \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "aegis_bridge",
    "controller_ip": "100.124.76.93",
    "collection": "hardware_metrics",
    "name": "Warren AHU-1 Edge Metrics",
    "equipment_type": "air_handler"
  }'
```

**External database** -- Import from InfluxDB, PostgreSQL, TiDB, SQLite3, MongoDB, or SpaceTimeDB:
```bash
curl -X POST http://localhost:3030/api/v1/datasets/connect \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "influxdb",
    "url": "http://influxdb:8086",
    "database": "building_sensors",
    "measurement": "ahu_readings",
    "name": "InfluxDB AHU Data",
    "equipment_type": "air_handler"
  }'
```

Prometheus automatically detects column types, computes statistics, and identifies the time column for all data sources.

### Stage 2: Analyze (Gradient AI)

The **PrometheusForge** agent (deployed on DigitalOcean Gradient AI) analyzes data patterns and recommends the optimal architecture from 13 options:

- **Time-Series**: LSTM Autoencoder, GRU Predictor, RNN, Sentinel MLP, Conv1d
- **Vision**: Conv2d, VGG-11/16, ResNet, ViT
- **NLP**: BERT (encoder), GPT-2 (decoder)
- **Specialized**: Nexus (multi-modal fusion), Phantom (edge-optimized)

### Stage 3: Train (AxonML)

Pure-Rust training via AxonML 0.4.1's autograd engine with real backpropagation (single forward + backward pass per batch). Each of the 13 architectures implements the `TrainableModel` trait (`forward(&Variable) -> Variable`, `parameters() -> Vec<Parameter>`) using AxonML's nn layers (Linear, LSTM, GRU, RNN, Conv1d, Conv2d, BatchNorm2d, TransformerEncoder, TransformerDecoder, ResidualBlock, MultiHeadAttention, CrossAttention, Sequential, etc.) with built-in optimizers (Adam, AdamW with weight decay) and loss functions (MSELoss, BCELoss, CrossEntropyLoss).

- Real-time loss curves updated every epoch via WebSocket
- Early stopping with configurable patience
- Z-score normalization (stats computed from training set only)
- Configurable hyperparameters (learning rate, batch size, hidden dim, etc.)
- 5-stage training pipeline: validate -> preprocess -> train -> evaluate -> export

### Stage 4: Evaluate

19-metric evaluation including precision, recall, F1, AUC-ROC, confusion matrix, and Gradient AI-powered qualitative assessment.

### Stage 5: Convert

Export trained models from `.axonml` to industry-standard formats for hardware-accelerated inference:

**ONNX** -- Universal format for ONNX Runtime, TensorRT, OpenVINO, and more:
```bash
curl -X POST http://localhost:3030/api/v1/models/$MODEL_ID/convert?format=onnx \
  -H "Authorization: Bearer $TOKEN"
# {"file_size": 245760, "download_url": "/api/v1/models/.../download?format=onnx"}
```

**HEF (Hailo Execution Format)** -- Optimized binary for Hailo-8/8L AI accelerators:
```bash
curl -X POST http://localhost:3030/api/v1/models/$MODEL_ID/convert?format=hef \
  -H "Authorization: Bearer $TOKEN"
# {"file_size": 102400, "download_url": "/api/v1/models/.../download?format=hef"}
```

All 13 architectures are supported. The converter faithfully reconstructs PyTorch models from `.axonml` weights, exports to ONNX, and validates output matching via ONNX Runtime before saving.

### Stage 6: Deploy

Deploy converted models to any edge device. Models run on Hailo-8/8L NPUs for hardware-accelerated inference -- commonly on Raspberry Pi 5, but any edge device with a Hailo AI accelerator is supported:

```bash
# On an edge device (e.g., Raspberry Pi 5 with Hailo-8L)
./prometheus-edge --model models/warren-ahu1.hef --port 6200

# Query predictions via the inference daemon
curl http://192.168.1.100:6200/predict
# {"anomaly_score": 0.12, "health_score": 0.94, "predictions": [...]}
```

The edge daemon (`prometheus-edge`) supports `.axonml` (CPU inference via AxonML), `.onnx` (ONNX Runtime), and `.hef` (Hailo NPU) model formats.

---

## Model Architectures (13)

| Architecture | Parameters | Use Case | Output |
|-------------|-----------|----------|--------|
| **LSTM Autoencoder** | 32K-128K | Anomaly detection | Reconstruction error |
| **GRU Predictor** | 64K-192K | Failure prediction | 3 horizon probabilities |
| **Sentinel MLP** | 16K-64K | Health scoring | Single 0.0-1.0 score |
| **RNN** | 32K-96K | Sequence classification | Class probabilities |
| **Conv1d** | 16K-64K | Temporal feature extraction | Sigmoid score |
| **Conv2d** | 64K-512K | Spatial classification (images) | Softmax classes |
| **VGG** (11/16) | 512K-4M | Image classification | Softmax classes |
| **ResNet** | 256K-2M | Image classification (skip connections) | Softmax classes |
| **ViT** | 128K-1M | Vision Transformer | Softmax classes |
| **BERT** | 256K-2M | NLP encoder (text classification) | Softmax classes |
| **GPT-2** | 256K-2M | NLP decoder (text generation) | Softmax tokens |
| **Nexus** | 64K-256K | Multi-modal fusion (sensor + context) | Multi-head output |
| **Phantom** | 32K-128K | Edge-optimized lightweight model | Sigmoid score |

All models export to `.axonml` binary format with optional INT8 quantization. Models can be converted to **ONNX** (universal) or **HEF** (Hailo-8 NPU) for hardware-accelerated edge deployment.

---

## Subscription Tiers

Prometheus uses Stripe for subscription billing with server-side enforcement of all resource limits.

| Resource | Free | Pro ($49/mo) | Enterprise |
|----------|------|-------------|------------|
| AI tokens/month | 1,000 | 50,000 | 500,000 |
| Datasets | 3 | 50 | 500 |
| Models | 2 | 25 | 200 |
| Deployments | 1 | 10 | 100 |
| Concurrent trainings | 1 | 5 | 20 |
| Max dataset size | 50 MB | 500 MB | 10 GB |
| Priority GPU | -- | -- | Yes |
| SSO | -- | -- | Yes |

All limits are enforced at the API layer. Exceeding a limit returns `403 Forbidden` with upgrade guidance.

**Billing flow:** Stripe Checkout -> webhook (`checkout.session.completed`) -> subscription created in Aegis-DB -> tier limits applied. Subscription changes sync automatically via Stripe webhooks.

---

## Authentication & User Lifecycle

Prometheus provides a complete auth system powered by Aegis-DB (Argon2id password hashing, opaque bearer tokens):

| Flow | Description |
|------|-------------|
| **Signup** | `POST /auth/signup` -- creates user + Free tier subscription |
| **Email verification** | 6-digit code sent via email, 15-minute expiry |
| **Login** | Returns token + `email_verified` and `mfa_required` flags |
| **MFA** | Optional TOTP-based 2FA (setup, verify, disable) |
| **Password reset** | Token-based, 30-minute expiry, anti-enumeration |
| **Change password** | Authenticated, requires current password |

Login is blocked until email is verified (`emailPending` status). MFA validation is required before a full session token is issued when enabled.

---

## Mobile App

Prometheus includes an **Expo SDK 55** (React Native) mobile app for iOS and Android.

**Features:**
- Full training pipeline management (start, monitor, stop)
- Real-time push notifications (training complete/failed, security alerts)
- Model and dataset browsing
- OTA updates via EAS Update (no App Store resubmission)
- TOTP-based MFA enrollment

**Push notification types:**
- `training_complete` / `training_failed` / `training_epoch_milestone`
- `training_queued` (default priority) / `training_started_from_queue` (high priority)
- `deployment_ready`
- `security_alert` / `account_verified` / `subscription_changed`

**Build and deploy:**
```bash
cd mobile/prometheus-mobile
npm install
npx expo start                          # Development
eas build --platform ios --profile production   # iOS build
eas build --platform android --profile production  # Android build
eas update --branch production            # OTA update
```

---

## API Reference

See [docs/API_REFERENCE.md](docs/API_REFERENCE.md) for complete documentation. Summary of all endpoints:

### Public Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `POST` | `/auth/login` | Authenticate, receive bearer token |
| `POST` | `/auth/logout` | Invalidate session |
| `GET` | `/auth/session` | Validate current session |
| `GET` | `/auth/me` | Get current user info |
| `POST` | `/auth/signup` | Register new account |
| `POST` | `/auth/verify-email` | Verify email with 6-digit code |
| `POST` | `/auth/resend-verification` | Resend verification code |
| `POST` | `/auth/forgot-password` | Request password reset email |
| `POST` | `/auth/reset-password` | Reset password with token |
| `POST` | `/auth/mfa/validate` | Validate TOTP code during login |
| `POST` | `/billing/webhook` | Stripe webhook (signature-verified) |
| `POST` | `/api/v1/auth/cli/init` | Init CLI auth session (returns QR code URL) |
| `GET` | `/api/v1/auth/cli/poll/:code` | Poll CLI auth session status |
| `POST` | `/api/v1/auth/cli/verify` | Verify CLI auth from browser |

### Protected Endpoints (Bearer Token Required)

**Datasets**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/datasets` | List all datasets |
| `POST` | `/api/v1/datasets` | Upload CSV dataset |
| `POST` | `/api/v1/datasets/connect` | Connect data source (Aegis, InfluxDB, PostgreSQL, TiDB, SQLite3, MongoDB, SpaceTimeDB) |
| `GET` | `/api/v1/datasets/{id}` | Get dataset details |
| `DELETE` | `/api/v1/datasets/{id}` | Delete dataset |
| `GET` | `/api/v1/datasets/{id}/preview` | Preview dataset rows (paginated, sortable) |
| `POST` | `/api/v1/datasets/{id}/validate` | Validate dataset (type consistency, missing values, schema); locks and auto-compresses with zstd on success. Sets `is_validated` field |
| `POST` | `/api/v1/datasets/{id}/unlock` | Unlock a validated dataset; auto-decompresses |
| `GET` | `/api/v1/datasets/{id}/recommend` | AI-powered model recommendations based on dataset analysis |

The preview endpoint supports pagination and sorting via query params: `page` (default 1), `page_size` (default 100), `sort_col`, `sort_dir` (`asc`/`desc`). Response includes `headers`, `rows`, `total_rows`, `page`, and `total_pages`.

Datasets have an `is_validated` field. A dataset must be validated before training can start. Validation checks type consistency, missing values, and schema issues. On success the dataset is locked and auto-compressed with zstd. Use the unlock endpoint to reverse this.

**Training**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/training` | List training runs |
| `POST` | `/api/v1/training/start` | Start a training run (queued if at max capacity) |
| `GET` | `/api/v1/training/{id}` | Get training run status |
| `POST` | `/api/v1/training/{id}/stop` | Stop a training run |
| `GET` | `/api/v1/training/queue` | Queue status: `active_trainings`, `max_concurrent`, `queued`, `capacity_available` |

When the server is at max concurrent training capacity, new runs are queued (status: `queued`) instead of rejected. Users receive a push notification (and optional email) when training is queued. When a slot opens, the next queued run auto-starts and the user is notified. Max concurrent trainings is controlled by the `PROMETHEUS_MAX_TRAININGS` env var (defaults to CPU core count).

Valid training statuses: `queued`, `running`, `completed`, `failed`, `cancelled`, `stopping`.

**Models**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/models` | List trained models |
| `GET` | `/api/v1/models/{id}` | Get model details |
| `DELETE` | `/api/v1/models/{id}` | Delete model |
| `GET` | `/api/v1/models/{id}/download` | Download model (format=axonml\|onnx\|hef) |
| `POST` | `/api/v1/models/{id}/convert` | Convert model (format=onnx\|hef) |
| `POST` | `/api/v1/models/{id}/compare` | Compare two models |

**Deployment**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/deployments` | List deployments |
| `POST` | `/api/v1/deployments` | Deploy model to edge target |
| `GET` | `/api/v1/deployments/targets` | List available edge targets |
| `GET` | `/api/v1/deployments/{id}` | Get deployment details |
| `GET` | `/api/v1/deployments/{id}/binary` | Download ARM binary |

**Agent (Gradient AI)**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/agent/chat` | Chat with PrometheusForge agent |
| `POST` | `/api/v1/agent/analyze` | Trigger data analysis |
| `GET` | `/api/v1/agent/history` | Get conversation history |

**Evaluation**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/evaluations` | List evaluations |
| `GET` | `/api/v1/evaluations/{id}` | Get evaluation details |
| `POST` | `/api/v1/evaluations/{id}/gradient` | Trigger Gradient AI evaluation |

**Billing & Subscriptions**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/billing/subscription` | Get current subscription |
| `POST` | `/billing/checkout` | Create Stripe checkout session |
| `POST` | `/billing/portal` | Create Stripe customer portal |
| `GET` | `/billing/usage` | Get usage stats and tier limits |

**User Profile & MFA**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/profile` | Get user profile |
| `GET/PUT` | `/profile/preferences` | Get/update user preferences |
| `PUT` | `/auth/change-password` | Change password (authenticated) |
| `POST` | `/mfa/setup` | Generate TOTP secret + QR |
| `POST` | `/mfa/verify` | Verify and enable MFA |
| `POST` | `/mfa/disable` | Disable MFA |

**Push Notifications**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/push/register` | Register Expo push token |

**Admin**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/admin/users` | List all users |
| `POST` | `/admin/users` | Create user |
| `GET/PUT/DELETE` | `/admin/users/:username` | Get/update/delete user |

**System**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/system/metrics` | System health metrics |

**WebSocket**

| Protocol | Endpoint | Description |
|----------|----------|-------------|
| `WS` | `/ws/training/{id}` | Live training progress stream |

---

## Testing

Prometheus has **767 tests** across 5 testing layers:

```
+-----------------------------------------------------------+
|                    Test Pyramid                            |
|                                                           |
|                    +---------+                             |
|                    | 63 PDF  |  Puppeteer                 |
|                    | Visual  |  (5 files)                 |
|                  +-+---------+-+                           |
|                  |  204 E2E    |  Playwright               |
|                  |  Browser    |  (15 specs)               |
|                +--+------------+-+                         |
|                | 123 Integration  |  Rust async             |
|                | API Tests        |  (8 files)              |
|              +--+-----------------+-+                      |
|              |  364 Unit Tests      |  cargo test            |
|              |  (all passing)       |  (4 crates)            |
|              +----------------------+                      |
+-----------------------------------------------------------+
```

### Layer 1: Unit Tests -- 364 passing

```bash
cargo test --workspace
```

| Crate | Tests | Coverage |
|-------|-------|----------|
| `prometheus-training` | 130 | Preprocessing, normalization, metrics, export, cross-compile, architectures, pipeline |
| `prometheus-reports` | 77 | HTML generation, training reports, deployment certs, config, file utilities |
| `prometheus-server` | 142 | Config, error mapping, auth, billing tiers, Stripe signatures, push notifications, user lifecycle, state |
| `prometheus-edge` | 15 | Config loading, inference engine, sensor polling |

```bash
# Run tests for a specific crate
cargo test -p prometheus-training
cargo test -p prometheus-server
cargo test -p prometheus-reports
cargo test -p prometheus-edge
```

### Layer 2: Integration Tests -- 123 test functions

Require a running Prometheus server + Aegis-DB.

```bash
# Start services first, then:
cargo test --test auth_tests           # 17 tests -- login, session, RBAC, rate limiting
cargo test --test deployment_tests     # 20 tests -- deployment CRUD, binary download
cargo test --test edge_cases_tests     # 18 tests -- validation, error handling, edge cases
cargo test --test api_tests            # 15 tests -- dataset + model API lifecycle
cargo test --test agent_tests          # 15 tests -- agent chat + analysis
cargo test --test evaluation_tests     # 15 tests -- evaluation metrics API
cargo test --test websocket_tests      # 13 tests -- WebSocket connection + messages
cargo test --test training_tests       # 10 tests -- training pipeline lifecycle
```

### Layer 3: E2E Browser Tests -- 204 test cases

```bash
cd tests/e2e
npm install
npx playwright install --with-deps
npx playwright test
```

| Spec | Tests | Scope |
|------|-------|-------|
| `accessibility.spec.ts` | 19 | ARIA labels, keyboard nav, color contrast |
| `auth.spec.ts` | 18 | Login, logout, session expiry, redirects |
| `navigation.spec.ts` | 18 | Sidebar routing, breadcrumbs, back/forward |
| `dashboard.spec.ts` | 15 | Metric cards, pipeline visualization |
| `agent.spec.ts` | 14 | Chat interface, message history |
| `responsive.spec.ts` | 14 | Mobile, tablet, desktop breakpoints |
| `settings.spec.ts` | 14 | API key management, system config |
| `datasets.spec.ts` | 13 | Upload, preview, delete, source connect |
| `error-handling.spec.ts` | 13 | 404, 500, network errors, retry |
| `evaluation.spec.ts` | 13 | Metrics display, confusion matrix |
| `deployment.spec.ts` | 12 | Target selection, binary download |
| `models.spec.ts` | 12 | Model gallery, comparison, download |
| `training.spec.ts` | 11 | Start training, live progress |
| `pipeline-workflow.spec.ts` | 10 | Full ingest->deploy workflow |
| `websocket.spec.ts` | 8 | Live updates, reconnection |

### Layer 4: PDF & Visual Tests -- 63 test cases

```bash
cd tests/puppeteer
npm install
npm test
```

| File | Tests | Scope |
|------|-------|-------|
| `visual_regression.test.js` | 17 | Screenshot comparison, layout consistency |
| `report_generation.test.js` | 13 | Training report PDF, structure, content |
| `chart_rendering.test.js` | 12 | SVG chart rendering, data accuracy |
| `multi_page_report.test.js` | 11 | Multi-page PDF layout, page breaks |
| `performance.test.js` | 10 | Page load times, memory, JS errors |


### Layer 5: Model Converter E2E Tests -- 13 architectures

End-to-end `.axonml` → ONNX → HEF pipeline tests. Creates synthetic `.axonml` files for every supported architecture, converts to ONNX, validates with ONNX Runtime, and (with Hailo DFC installed) compiles to HEF.

```bash
cd tools/model_converter
python test_convert.py
```

| Architecture | Parameters | ONNX Validated | Max Diff |
|-------------|-----------|----------------|----------|
| Sentinel | 9,729 | Yes | 0.00e+00 |
| LSTM Autoencoder | 8,365 | Yes | 1.61e-06 |
| GRU Predictor | 12,675 | Yes | 5.96e-08 |
| RNN | 945 | Yes | 5.59e-08 |
| Phantom | 249 | Yes | 4.47e-08 |
| Conv1D | 1,009 | Yes | 2.98e-07 |
| Conv2D | 102,917 | Yes | 0.00e+00 |
| ResNet-18 | 11,180,170 | Yes | 0.00e+00 |
| VGG-11 | 9,747,205 | Yes | 0.00e+00 |
| BERT | 132,930 | Yes | 1.04e-07 |
| GPT-2 | 133,320 | Yes | 9.83e-07 |
| ViT | 400,645 | Yes | 0.00e+00 |
| Nexus | 34,433 | Yes | 0.00e+00 |


---

## NexusEdge Design System

Prometheus uses the **NexusEdge** design system -- a warm, professional aesthetic for industrial applications.

### Color Palette

| Token | Hex | Usage |
|-------|-----|-------|
| `cream` | `#FFFDF7` | Page backgrounds |
| `bg-off-white` | `#FAF8F5` | Card surfaces |
| `border-tan` | `#E8D4C4` | Borders, dividers |
| `primary` | `#14b8a6` | Buttons, links, active states |
| `terracotta` | `#C4A484` | Icons, secondary accents |
| `russet` | `#C2714F` | Warm highlights |
| `text` | `#111827` | Body text |
| `muted` | `#6b7280` | Secondary text |

### Equipment-Specific Colors

| Equipment | Color | Token |
|-----------|-------|-------|
| Air Handler | `#3b82f6` | `equip-air-handler` |
| Boiler | `#ef4444` | `equip-boiler` |
| Pump | `#8b5cf6` | `equip-pump` |
| Chiller | `#06b6d4` | `equip-chiller` |
| Fan Coil | `#22c55e` | `equip-fan-coil` |
| Steam | `#f97316` | `equip-steam` |

### Typography

- **Sans**: Inter (system-ui fallback)
- **Mono**: JetBrains Mono (Fira Code, Consolas fallback)

### Custom Components (30+)

Cards, buttons (primary/ghost/danger), inputs, modals, tables with pagination, line/bar charts (SVG), metric cards with trends, 7 badge variants, toast notifications, skeleton loaders, drag-and-drop file upload, pipeline visualization, chat bubbles, code blocks, and more -- all defined in `input.css` as Tailwind `@layer` components.

---

## Model Converter (.axonml → ONNX → HEF)

The model converter translates Prometheus `.axonml` binary model files into standard ONNX format, and optionally compiles to Hailo HEF for edge deployment on Hailo-8 accelerators.

### Pipeline

```
.axonml (binary) → PyTorch reconstruction → ONNX export → (optional) Hailo DFC → .hef
```

### Usage

```bash
# Convert to ONNX
python tools/model_converter/convert.py model.axonml --format onnx --output model.onnx

# Convert to HEF (requires Hailo DFC SDK)
python tools/model_converter/convert.py model.axonml --format hef --output model.hef
```

### Hailo DFC Setup

The Hailo Dataflow Compiler is required for ONNX → HEF conversion. Installed on the build server at `/opt/hailo-dfc-env/bin/python` (Python 3.10, DFC 3.30.0).

```bash
# Verify DFC installation
/opt/hailo-dfc-env/bin/python -c "from hailo_sdk_client import ClientRunner; print('DFC ready')"
```

### Supported Architectures

All 13 architectures are verified for end-to-end conversion:

Sentinel, LSTM Autoencoder, GRU Predictor, RNN, Phantom, Conv1D, Conv2D, ResNet-18, VGG-11, BERT, GPT-2, ViT, Nexus

---


## Edge Deployment

### Cross-Compile for Raspberry Pi

```bash
# Install cross-compilation tool
cargo install cross
rustup target add armv7-unknown-linux-musleabihf

# Build the edge daemon
cross build --release --target armv7-unknown-linux-musleabihf --bin prometheus-edge

# Deploy to Pi
scp target/armv7-unknown-linux-musleabihf/release/prometheus-edge pi@192.168.1.100:/opt/prometheus/
scp models/warren-ahu1.axonml pi@192.168.1.100:/opt/prometheus/models/
```

### Edge Daemon Endpoints

```bash
# Health check
curl http://192.168.1.100:6200/health
# {"status": "ok", "model": "warren-ahu1", "uptime": 86400}

# Run prediction
curl http://192.168.1.100:6200/predict
# {"anomaly_score": 0.12, "health_score": 0.94, "predictions": [...]}

# View metrics
curl http://192.168.1.100:6200/metrics
# {"predictions_total": 2880, "anomaly_score_avg": 0.08, "health_score_avg": 0.96}
```

---

## Gradient AI Agent (PrometheusForge)

PrometheusForge is the AI facilities engineering agent, deployed via the DigitalOcean Gradient AI ADK.

### Capabilities

| Capability | Description |
|------------|-------------|
| **Data Analysis** | Statistical profiling, quality scoring, pattern detection |
| **Architecture Selection** | Rule-based + LLM-powered model selection from 13 architectures |
| **Training Plans** | Complete training configurations with hyperparameters |
| **Model Evaluation** | Metric interpretation against HVAC equipment benchmarks |
| **Natural Language Chat** | Answer HVAC engineering and ML questions |

### Agent Architecture

```
PrometheusForge (Orchestrator)
+-- Data Analyst    -- sensor profiling, quality checks
+-- Architect       -- model architecture selection
+-- Evaluator       -- post-training metric analysis
```

### Setup

```bash
cd crates/prometheus-agent
pip install -r requirements.txt
cp .env.example .env
# Edit .env with your Gradient + LLM credentials
gradient agent deploy
```

### Python Dependencies

- `gradient-adk` -- DigitalOcean Gradient AI Agent Development Kit
- `langchain` + `langchain-community` -- LLM orchestration
- `pandas` + `numpy` -- data analysis
- `httpx` -- async HTTP client
- `pyyaml` + `python-dotenv` -- configuration

See [docs/GRADIENT_SETUP.md](docs/GRADIENT_SETUP.md) for detailed instructions.

---

## CLI / TUI

Prometheus includes a full-featured terminal interface with branded NexusEdge colors.

### Installation

```bash
cargo install --path crates/prometheus-cli
```

### Subcommands

```bash
prometheus login           # QR code + browser auth
prometheus datasets        # List datasets
prometheus upload data.csv # Upload CSV
prometheus train <ds_id> --arch resnet --epochs 100
prometheus monitor <id>    # Open TUI training monitor
prometheus compress data.csv       # OpenZL compression
prometheus decompress data.ozl     # OpenZL decompression
prometheus agent "What architecture should I use?"
```

### Interactive REPL

Run `prometheus` without arguments for an interactive REPL with 22+ slash commands:

```
prometheus> /datasets
prometheus> /train ds_abc123 gru_predictor
prometheus> /monitor
prometheus> What's the best architecture for time-series anomaly detection?
```

Non-slash input is sent directly to the PrometheusForge AI agent.

### TUI Training Monitor

```bash
prometheus monitor              # Dashboard view
prometheus monitor <run_id>     # Focus on specific training run
```

Features: live loss chart (Braille markers), epoch metrics, run info panel, NexusEdge color theme.

### OpenZL Compression

Custom `.ozl` format wrapping zstd with SHA-256 integrity verification:

```bash
prometheus compress dataset.csv              # -> dataset.csv.ozl
prometheus decompress dataset.csv.ozl        # -> dataset.csv
prometheus train-compressor *.csv --output my.ozl-dict  # Train custom dictionary
prometheus compress data.csv --dict my.ozl-dict         # Use trained dictionary
```

### QR Code Authentication

```bash
prometheus login
# Displays QR code + URL in terminal
# Scan QR or paste URL in browser -> authenticate -> CLI auto-detects
# Token saved to ~/.prometheus/credentials (0600 perms)
```

---

## Docker

### Multi-Stage Build (6 stages)

```dockerfile
# Base:     rust:1.75-bookworm + cargo-chef
# Stage 1:  Dependency recipe (cargo-chef)
# Stage 2:  Leptos UI -> WebAssembly (wasm32-unknown-unknown)
# Stage 3:  Tailwind CSS build (node:20-slim)
# Stage 4:  Rust server binary compilation
# Stage 5:  Runtime image (debian:bookworm-slim)
#           - ca-certificates, libssl3, chromium, fonts-inter
#           - Non-root user: prometheus (uid 1000)
```

### Docker Compose Services

```yaml
services:
  prometheus-server:   # :3030 -- Web UI + API
    depends_on: aegis-db
    volumes: [prometheus-data, prometheus-models]

  aegis-db:            # :9091 -- Database
    volumes: [aegis-data]

networks: [prometheus-net]
```

---

## Environment Variables

### Prometheus Server

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `PROMETHEUS_HOST` | No | `0.0.0.0` | Bind address |
| `PROMETHEUS_PORT` | No | `3030` | Server listen port |
| `AEGIS_DB_URL` | No | `http://localhost:9091` | Aegis-DB connection URL |
| `GRADIENT_MODEL_ACCESS_KEY` | No | -- | Gradient AI model access key |
| `GRADIENT_AGENT_ID` | No | -- | Deployed Gradient agent ID |
| `PROMETHEUS_DATA_DIR` | No | `/tmp/prometheus-data` | Data storage directory |
| `STRIPE_SECRET_KEY` | No | -- | Stripe API secret key |
| `STRIPE_WEBHOOK_SECRET` | No | -- | Stripe webhook signing secret |
| `STRIPE_PRO_PRICE_ID` | No | -- | Stripe Price ID for Pro tier |
| `STRIPE_ENTERPRISE_PRICE_ID` | No | -- | Stripe Price ID for Enterprise tier |
| `RESEND_API_KEY` | No | -- | Resend transactional email API key |
| `PROMETHEUS_MAX_TRAININGS` | No | CPU core count | Max concurrent training runs (excess runs are queued) |
| `RUST_LOG` | No | `info` | Logging level filter |

### Gradient AI Agent

| Variable | Required | Description |
|----------|----------|-------------|
| `DO_API_TOKEN` | Yes | DigitalOcean API token |
| `GRADIENT_APP_ID` | Yes | Gradient application ID |
| `GRADIENT_WORKSPACE_ID` | Yes | Gradient workspace ID |
| `LLM_API_KEY` | Yes | LLM provider API key |
| `LLM_MODEL_NAME` | No | Model name (default: gpt-4o) |
| `PROMETHEUS_API_URL` | Yes | Prometheus server URL |
| `PROMETHEUS_API_KEY` | Yes | Prometheus API key |

---

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for development setup and guidelines.

```bash
# Development workflow
cargo fmt --check                         # Format check
cargo clippy --workspace                  # Lint
cargo test --workspace                    # 364 unit tests
cargo test --test auth_tests              # Integration (requires server)
cd tests/e2e && npx playwright test       # 204 E2E browser tests
cd tests/puppeteer && npm test            # 63 visual + PDF tests
```

### Build Profiles

| Profile | LTO | Codegen Units | Opt Level |
|---------|-----|---------------|-----------|
| `dev` | off | default | 0 |
| `release` | thin | 1 | 3 |

---

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/ARCHITECTURE.md) | System architecture, data flow, crate structure |
| [API Reference](docs/API_REFERENCE.md) | Complete REST API with request/response examples |
| [Deployment Guide](docs/DEPLOYMENT.md) | Docker, manual, production, edge deployment |
| [Gradient Setup](docs/GRADIENT_SETUP.md) | Gradient AI agent configuration and deployment |
| [Contributing](docs/CONTRIBUTING.md) | Development setup, coding standards, PR process |
| [PRD](PRD.md) | Full Product Requirements Document |

---

## License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE).

---

<p align="center">
  Built with Rust, AxonML, Aegis-DB, and DigitalOcean Gradient AI<br>
  for the <strong>DigitalOcean Gradient AI Hackathon 2026</strong>
</p>
