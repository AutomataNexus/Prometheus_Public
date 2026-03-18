# Prometheus — Demo Video Narration Script
**Speaker: Andrew Jewell Sr. | AutomataNexus LLC**
**Target: Under 3 minutes**

---

### [0:00 — Landing Page]
"Hi, I'm Andrew Jewell Sr. This is Prometheus — a full-stack AI training orchestrator built entirely in Rust, powered by DigitalOcean Gradient AI.

Prometheus takes raw data from any domain — medical, financial, industrial, satellite, NLP — and turns it into trained, quantized, edge-deployable ML models. All from one platform."

*[scrolling through landing page sections]*

"Here you can see our feature overview, the 6-stage pipeline from data ingestion through edge deployment, and our subscription tiers — Free through Enterprise at $249 a month, all handled through Stripe."

### [0:25 — Login]
"Let's log in. Authentication is handled by Aegis-DB with Argon2id password hashing. We support MFA via TOTP as well."

### [0:35 — Dashboard]
"The dashboard gives an overview — pipeline status, active training jobs, server capacity, and resource limits for the current tier."

### [0:45 — Datasets Page]
"On the Datasets page, users can upload CSV files, browse our pre-loaded catalog with over 2,600 datasets across 30 domains..."

*[Browse Catalog opens]*

"...or connect an external data source — InfluxDB, PostgreSQL, MongoDB, SpaceTimeDB, and more. All connection credentials are encrypted with AES-256-GCM through our Shield security engine. Prometheus cannot read your raw credentials."

*[Connect Source opens and closes]*

"Down here are Ingestion Keys for programmatic data uploads via API."

*[creates Demo Ingestion Key]*

### [1:10 — Dataset Detail: Validate & Analyze]
"Let's open our HVAC dataset — 33,000 rows, 11 features. I'll unlock it, run validation, and then ask PrometheusForge to analyze it."

*[Unlock, Validate, Analyze clicks — wait for AI]*

"PrometheusForge is our Gradient AI agent — it's powered by Anthropic Claude through DigitalOcean's Gradient platform, backed by a 24-document knowledge base with RAG retrieval. It analyzes the dataset and recommends specific architectures with tuned hyperparameters."

*[recommendation cards appear — LSTM, GRU, Sentinel, Phantom]*

"It's recommending LSTM Autoencoder as the top pick for this time-series data. Let's train it."

*[clicks LSTM to start training]*

### [1:55 — Training Page]
"The Training page shows our new job running. The Start Training modal lets you select a dataset, pick from all 13 architectures, adjust every hyperparameter, and resume from an existing model checkpoint."

### [2:05 — Training Detail (WebSocket)]
"Clicking into the active run connects via WebSocket for instant updates — watch the epochs tick, loss values update, and charts draw in real-time."

### [2:15 — Monitor]
"The Training Monitor gives a full overview of all active and completed runs with auto-refresh."

### [2:20 — Agent Chat]
"PrometheusForge also has a full chat interface. Users can ask natural language questions about architectures, data, or training strategies — and get concrete, actionable recommendations."

### [2:35 — Models]
"Trained models show precision, recall, F1, validation loss — every metric has a tooltip. Models can be renamed, retrained from checkpoint, evaluated, or exported."

### [2:40 — Evaluation, Convert, Quantize]
"Evaluation runs quality assessment. The Convert page exports to ONNX or HEF for Hailo-8 NPU acceleration. Quantization compresses models — Q8 gives 3.8x, Q4 gives over 7x."

### [2:48 — Deployment, Billing, Admin]
"Deployment pushes models to edge controllers with encrypted SSH credentials.

Billing is Stripe-integrated — four tiers with usage-based token billing. And you can sponsor the open-source projects behind Prometheus right here.

The Admin panel shows all users with tier, tokens, storage, MFA, and full CRUD."

### [2:58 — Close]
"Prometheus — 8 Rust crates, 13 architectures, real AxonML autograd training, Gradient AI agent with RAG, Hailo NPU deployment, AES-256-GCM security, and Stripe billing. All open source.

Built for the DigitalOcean Gradient AI Hackathon. Thank you."
