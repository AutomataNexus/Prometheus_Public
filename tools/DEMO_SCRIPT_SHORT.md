# Prometheus — Demo Video Narration (SHORT — Under 3 Minutes)
**Speaker: Andrew Jewell Sr. | AutomataNexus LLC**

---

Hi, I'm Andrew Jewell Sr. This is Prometheus — a full-stack AI training orchestrator built in Rust, powered by DigitalOcean Gradient AI.

Upload data from any domain. Get AI-powered architecture recommendations. Train across 13 neural network architectures. Quantize. Convert to ONNX or Hailo HEF. Deploy to edge devices. All from one platform.

Here's our landing page with features, the 6-stage pipeline, and Stripe-integrated subscription tiers.

Logging in — authentication uses Aegis-DB with Argon2id hashing and optional MFA.

The dashboard shows pipeline status, active training, and server capacity.

On Datasets, users upload CSV files, browse our 30-domain catalog with over 2,600 datasets, or connect external sources like InfluxDB, PostgreSQL, and MongoDB. All credentials encrypted with AES-256-GCM.

Opening our HVAC dataset — 33,000 rows, 11 features. We validate it and ask PrometheusForge to analyze.

PrometheusForge is our Gradient AI agent powered by Claude through DigitalOcean, with a 24-document RAG knowledge base. It recommends LSTM Autoencoder as the top pick. Let's train it.

Training uses AxonML — pure Rust, real autograd backpropagation. Users adjust every hyperparameter and can resume from checkpoints.

The training detail page connects via WebSocket for instant epoch updates.

Trained models show precision, recall, F1, with tooltips explaining every metric. Models can be renamed, retrained, evaluated, or exported.

The Convert page exports to ONNX or HEF for Hailo-8 NPU acceleration. Quantization compresses models — Q8 gives 3.8x, Q4 gives over 7x.

Deployment pushes models to edge controllers with encrypted SSH credentials. LSTM and GRU models run natively on Raspberry Pi at 1.8 megabytes RSS. CNN models accelerate via Hailo-8.

Billing is Stripe-integrated with four tiers and usage-based token billing.

The Admin panel shows all users with tier, tokens, storage, and full management.

Prometheus — 8 Rust crates, 13 architectures, real training, Gradient AI agent with RAG, Hailo NPU deployment, AES-256-GCM security, and Stripe billing. All open source. Built for the DigitalOcean Gradient AI Hackathon. Thank you.
