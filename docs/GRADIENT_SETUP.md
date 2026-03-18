# Prometheus -- Gradient AI Configuration Guide

This document describes how to set up and configure the DigitalOcean Gradient AI integration for Prometheus.

## Prerequisites

- A DigitalOcean account with Gradient AI access
- Python 3.10+ (for the ADK agent)
- The `gradient-adk` Python package

## Step 1: Obtain API Credentials

1. Log in to the [DigitalOcean Cloud Console](https://cloud.digitalocean.com)
2. Navigate to **API** > **Tokens/Keys**
3. Generate a new Personal Access Token with read/write scope
4. Save the token securely -- it will be used as `DIGITALOCEAN_API_TOKEN`

## Step 2: Set Up Gradient AI Workspace

1. Navigate to **Gradient AI** in the DigitalOcean console
2. Create a new workspace (or use an existing one)
3. Note the **Workspace ID** -- used as `GRADIENT_WORKSPACE_ID`

## Step 3: Create the Knowledge Base

Upload the following documents to a Gradient Knowledge Base for RAG:

| Document | Path | Purpose |
|----------|------|---------|
| HVAC Fault Catalog | `crates/prometheus-agent/knowledge/hvac_fault_catalog.md` | 50+ common failure modes with detection signatures |
| Equipment Specs | `crates/prometheus-agent/knowledge/equipment_specs.md` | Sensor ranges and normal operating parameters |
| AxonML API Reference | `crates/prometheus-agent/knowledge/axonml_reference.md` | Model architecture construction guide |
| Training History | `crates/prometheus-agent/knowledge/training_history.md` | Past results for RAG context |

Steps:
1. In Gradient AI, go to **Knowledge Bases**
2. Create a new knowledge base named `prometheus-ml`
3. Upload each markdown file from the `knowledge/` directory
4. Note the **Knowledge Base ID** -- used as `PROMETHEUS_KNOWLEDGE_BASE_ID`

## Step 4: Install the ADK

```bash
pip install gradient-adk
```

## Step 5: Deploy the PrometheusForge Agent

```bash
cd /opt/Prometheus/crates/prometheus-agent

# Copy and configure environment
cp .env.example .env
# Edit .env with your actual credentials (see below)

# Initialize the agent
gradient agent init

# Deploy to Gradient
gradient agent deploy
```

After deployment, note the **Agent ID** from the deploy output -- used as `GRADIENT_AGENT_ID`.

## Step 6: Configure Environment Variables

Edit `/opt/Prometheus/crates/prometheus-agent/.env`:

```bash
# DigitalOcean / Gradient
DO_API_TOKEN=dop_v1_your_token_here
GRADIENT_APP_ID=your_gradient_app_id
GRADIENT_WORKSPACE_ID=your_gradient_workspace_id

# LLM provider (via Gradient Serverless Inference)
LLM_API_KEY=your_llm_api_key
LLM_MODEL_NAME=gpt-4o
LLM_TEMPERATURE=0.2
LLM_MAX_TOKENS=4096

# Prometheus platform connection
PROMETHEUS_API_URL=http://localhost:3030
PROMETHEUS_API_KEY=your_prometheus_api_key
PROMETHEUS_KNOWLEDGE_BASE_ID=your_knowledge_base_id

# Training service
TRAINING_SERVICE_URL=http://localhost:3030
TRAINING_API_KEY=your_training_api_key

# Evaluation / metrics
METRICS_SERVICE_URL=http://localhost:3030
METRICS_API_KEY=your_metrics_api_key

# Logging
LOG_LEVEL=INFO
LOG_FORMAT=json
```

## Step 7: Configure Prometheus Server

Set the following environment variables for the Prometheus server process:

```bash
export DIGITALOCEAN_API_TOKEN=dop_v1_your_token_here
export GRADIENT_MODEL_ACCESS_KEY=your_model_access_key
export GRADIENT_AGENT_ID=your_deployed_agent_id
```

Or configure via the Prometheus UI:
1. Log in as admin
2. Navigate to **Settings**
3. Under **Gradient AI Configuration**, enter your API token and Agent ID
4. Click **Test Connection** to verify connectivity
5. Click **Save**

## Agent Architecture

The PrometheusForge agent uses a multi-agent architecture deployed via the Gradient ADK:

```
PrometheusForge Orchestrator
  |
  +-- Data Analyst Agent
  |     Analyzes sensor data patterns, detects seasonality, spots anomalies
  |
  +-- Architect Agent
  |     Selects from 13 architectures (LSTM, GRU, RNN, ResNet, VGG, ViT, BERT, GPT-2, Conv1D/2D, Nexus, Phantom, Sentinel)
  |
  +-- Evaluator Agent
        Evaluates trained model quality, suggests improvements
```

## Agent Capabilities

| Capability | Description |
|-----------|-------------|
| **Data Analysis** | Analyzes uploaded sensor CSV data for patterns, seasonality, and anomalies |
| **Architecture Selection** | Recommends from 13 architectures spanning time-series, vision, NLP, multimodal, and edge workloads |
| **Hyperparameter Tuning** | Suggests learning rate, hidden dimensions, sequence length, dropout |
| **Training Plan Generation** | Produces complete JSON training configurations for the AxonML pipeline |
| **Result Evaluation** | Interprets training metrics, suggests improvements, decides deploy-readiness |

## Running Agent Evaluations

Prometheus uses Gradient's agent evaluation framework to assess the PrometheusForge agent's recommendation quality:

1. Navigate to **Evaluation** in the Prometheus UI
2. Select a completed evaluation
3. Click **Run Gradient Evaluation**
4. Results include 19 evaluation metrics covering accuracy, relevance, and consistency

## Gradient Serverless Inference

The agent's LLM backbone uses Gradient's Serverless Inference API. This is configured via:
- `LLM_API_KEY` -- Access key for the inference endpoint
- `LLM_MODEL_NAME` -- Model to use (default: `gpt-4o`)

## Troubleshooting

| Issue | Solution |
|-------|---------|
| Agent not responding | Verify `GRADIENT_AGENT_ID` is correct and the agent is deployed (`gradient agent status`) |
| Knowledge Base errors | Check that all documents are uploaded and the KB ID matches your config |
| Rate limit errors | Gradient enforces request limits; reduce agent query frequency or contact support |
| Authentication failures | Regenerate `DIGITALOCEAN_API_TOKEN` in the cloud console and update `.env` |
| Timeout errors | Increase `LLM_MAX_TOKENS` or simplify the query |
| Agent returning generic responses | Verify Knowledge Base documents are properly indexed (re-upload if needed) |
