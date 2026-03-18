// ============================================================================
// File: agents.rs
// Description: Gradient AI agent chat and interaction endpoints
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{
    extract::State,
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub dataset_id: Option<String>,
    #[serde(default)]
    pub conversation_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct AnalyzeRequest {
    pub dataset_id: String,
}

// ---------------------------------------------------------------------------
// System prompt for PrometheusForge
// ---------------------------------------------------------------------------

const FORGE_SYSTEM_PROMPT: &str = r#"You are PrometheusForge, an AI engineering assistant built into the Prometheus ML platform. You help users analyze data, design ML models, and deploy solutions to edge devices.

Your capabilities:
1. **Data Analysis** — Examine datasets for patterns, anomalies, seasonality, and data quality issues
2. **Architecture Recommendation** — Recommend from 13 neural network architectures based on data characteristics:
   - LSTM Autoencoder: Anomaly detection via reconstruction error (any time-series/sequential data)
   - GRU Predictor: Multi-horizon prediction/forecasting
   - RNN: Simple sequence modeling
   - Sentinel: Multi-feature scoring/regression (high-dimensional tabular data)
   - ResNet: Image classification with residual connections (medical scans, photos, satellite, etc.)
   - VGG: Deep CNN image classification
   - ViT: Vision Transformer for image classification with global attention
   - BERT: Text classification, sentiment, intent detection (any language/domain)
   - GPT-2: Text generation, language modeling
   - Nexus: Multi-modal fusion (combining different data types via cross-attention)
   - Phantom: Ultra-lightweight edge model
   - Conv1d: 1D CNN for temporal/sequential feature extraction
   - Conv2d: 2D CNN for spatial/image data
3. **Training Plans** — Generate training configurations with optimal hyperparameters for AxonML
4. **Model Evaluation** — Interpret training results, compare models, suggest improvements
5. **Deployment** — Guide deployment to edge devices (ARM cross-compilation, INT8 quantization)

Prometheus is a GENERAL-PURPOSE ML platform. Users upload ANY type of data — medical images, financial time series, NLP corpora, game analytics, genomics, industrial sensors, satellite imagery, DNA sequences, and more. Never assume a specific domain. Analyze the actual data characteristics to make recommendations.

When given dataset context, use the actual column names, statistics, and data shape to make specific, data-driven recommendations. Be concise and actionable.

Important: The ML framework is AxonML (pure Rust, GPU/CUDA). Training runs via AxonML's autograd engine with real backpropagation."#;

// ---------------------------------------------------------------------------
// Chat endpoint
// ---------------------------------------------------------------------------

pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Store user message in history
    let conv_id = req.conversation_id.clone()
        .unwrap_or_else(|| format!("conv_{}", &Uuid::new_v4().to_string()[..8]));
    let turn_id = format!("msg_{}", &Uuid::new_v4().to_string()[..8]);
    let _ = state.aegis_create_doc("agent_history", json!({
        "id": turn_id,
        "conversation_id": conv_id,
        "role": "user",
        "message": req.message,
        "dataset_id": req.dataset_id,
        "timestamp": Utc::now().to_rfc3339(),
    })).await;

    // Guardrail: reject off-topic requests before hitting the API
    if let Some(rejection) = check_guardrails(&req.message) {
        let resp_id = format!("msg_{}", &Uuid::new_v4().to_string()[..8]);
        let _ = state.aegis_create_doc("agent_history", json!({
            "id": resp_id,
            "conversation_id": conv_id,
            "role": "assistant",
            "message": &rejection,
            "source": "guardrail",
            "timestamp": Utc::now().to_rfc3339(),
        })).await;

        return Ok(Json(json!({
            "response": rejection,
            "agent": "prometheus_forge",
            "source": "guardrail",
            "conversation_id": conv_id,
        })));
    }

    // Load dataset context if provided
    let dataset_context = if let Some(ref ds_id) = req.dataset_id {
        build_dataset_context(&state, ds_id).await
    } else {
        // Try to find the main dataset to provide context automatically
        auto_detect_dataset_context(&state).await
    };

    // Load recent conversation history
    let history = load_conversation_history(&state, &conv_id).await;

    // Try DO GenAI API first
    let response = if let Some(ref endpoint) = state.config.gradient_endpoint {
        if let Some(ref api_key) = state.config.gradient_api_key {
            match call_do_genai(&state, endpoint, api_key, &req.message, &dataset_context, &history).await {
                Ok(text) => text,
                Err(e) => {
                    tracing::warn!("DO GenAI call failed, using local fallback: {e}");
                    generate_smart_response(&req.message, &dataset_context)
                }
            }
        } else {
            generate_smart_response(&req.message, &dataset_context)
        }
    } else {
        generate_smart_response(&req.message, &dataset_context)
    };

    let source = if state.config.gradient_endpoint.is_some() && state.config.gradient_api_key.is_some() {
        "do_genai"
    } else {
        "local"
    };

    // Store response
    let resp_id = format!("msg_{}", &Uuid::new_v4().to_string()[..8]);
    let _ = state.aegis_create_doc("agent_history", json!({
        "id": resp_id,
        "conversation_id": conv_id,
        "role": "assistant",
        "message": &response,
        "source": source,
        "timestamp": Utc::now().to_rfc3339(),
    })).await;

    Ok(Json(json!({
        "response": response,
        "agent": "prometheus_forge",
        "source": source,
        "conversation_id": conv_id,
    })))
}

// ---------------------------------------------------------------------------
// DO GenAI integration (OpenAI-compatible chat completions)
// ---------------------------------------------------------------------------

async fn call_do_genai(
    state: &AppState,
    endpoint: &str,
    api_key: &str,
    message: &str,
    dataset_context: &str,
    history: &[(String, String)],
) -> Result<String, String> {
    // Send PrometheusForge's system prompt so the model has full context
    let mut messages = Vec::new();
    messages.push(json!({ "role": "system", "content": FORGE_SYSTEM_PROMPT }));
    if !dataset_context.is_empty() {
        messages.push(json!({ "role": "system", "content": dataset_context }));
    }

    // Add conversation history
    for (role, content) in history {
        messages.push(json!({ "role": role, "content": content }));
    }

    // Add current message
    messages.push(json!({ "role": "user", "content": message }));

    let body = json!({
        "model": "agent",
        "messages": messages,
        "max_tokens": 16000,
    });

    let resp = state
        .http_client
        .post(endpoint)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("API returned {status}: {text}"));
    }

    let data: serde_json::Value = resp.json().await
        .map_err(|e| format!("Parse error: {e}"))?;

    // OpenAI-compatible response: { "choices": [{ "message": { "content": "..." } }] }
    data.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            // Try alternative response formats
            data.get("response")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Unexpected response format: {}", data))
        })
}

// ---------------------------------------------------------------------------
// Dataset context builder
// ---------------------------------------------------------------------------

async fn build_dataset_context(state: &AppState, dataset_id: &str) -> String {
    match state.aegis_get_doc("datasets", dataset_id).await {
        Ok(ds) => format_dataset_context(&ds),
        Err(_) => String::new(),
    }
}

async fn auto_detect_dataset_context(state: &AppState) -> String {
    match state.aegis_list_docs("datasets").await {
        Ok(docs) if !docs.is_empty() => {
            // Find the largest/most active dataset
            let best = docs.iter()
                .max_by_key(|d| d.get("row_count").and_then(|v| v.as_u64()).unwrap_or(0));
            if let Some(ds) = best {
                format_dataset_context(ds)
            } else {
                String::new()
            }
        }
        _ => "No datasets available yet.".to_string(),
    }
}

fn format_dataset_context(ds: &serde_json::Value) -> String {
    let name = ds.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let id = ds.get("id").and_then(|v| v.as_str())
        .or_else(|| ds.get("_id").and_then(|v| v.as_str()))
        .unwrap_or("unknown");
    let rows = ds.get("row_count").and_then(|v| v.as_u64()).unwrap_or(0);
    let domain = ds.get("domain")
        .or_else(|| ds.get("equipment_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("general");
    let source = ds.get("source").and_then(|v| v.as_str()).unwrap_or("upload");
    let status = ds.get("status").and_then(|v| v.as_str()).unwrap_or("active");
    let file_size = ds.get("file_size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);

    let columns: Vec<String> = ds.get("columns")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|c| c.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let mut ctx = format!(
        "ACTIVE DATASET:\n- Name: {name}\n- ID: {id}\n- Domain: {domain}\n- Rows: {rows}\n- Columns ({count}): {cols}\n- Size: {size}\n- Status: {status}",
        count = columns.len(),
        cols = columns.join(", "),
        size = format_bytes(file_size),
    );

    if !source.is_empty() && source != "upload" {
        ctx.push_str(&format!("\n- Source: {source}"));
    }

    // Add column statistics
    if let Some(stats) = ds.get("column_stats").and_then(|v| v.as_object()) {
        ctx.push_str("\n\nColumn Statistics:");
        for (col, stat) in stats {
            let min = stat.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let max = stat.get("max").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let mean = stat.get("mean").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let std = stat.get("std").and_then(|v| v.as_f64()).unwrap_or(0.0);
            ctx.push_str(&format!("\n  {col}: min={min:.2}, max={max:.2}, mean={mean:.2}, std={std:.2}"));
        }
    }

    ctx
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ---------------------------------------------------------------------------
// Conversation history
// ---------------------------------------------------------------------------

async fn load_conversation_history(state: &AppState, conv_id: &str) -> Vec<(String, String)> {
    let docs = state.aegis_list_docs("agent_history").await.unwrap_or_default();
    let mut turns: Vec<(String, String, String)> = docs.iter()
        .filter(|d| d.get("conversation_id").and_then(|v| v.as_str()) == Some(conv_id))
        .filter_map(|d| {
            let role = d.get("role").and_then(|v| v.as_str())?.to_string();
            let msg = d.get("message").and_then(|v| v.as_str())?.to_string();
            let ts = d.get("timestamp").and_then(|v| v.as_str()).unwrap_or("").to_string();
            Some((ts, role, msg))
        })
        .collect();
    turns.sort_by(|a, b| a.0.cmp(&b.0));
    // Keep last 20 turns max
    turns.into_iter()
        .rev()
        .take(20)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|(_, role, msg)| (role, msg))
        .collect()
}

// ---------------------------------------------------------------------------
// Guardrails — keep PrometheusForge focused on Prometheus/AxonML tasks
// ---------------------------------------------------------------------------

const GUARDRAIL_RESPONSE: &str = "I'm PrometheusForge, the Prometheus ML engineering assistant. \
I'm designed to help with **data analysis, model training, and edge deployment** using AxonML.\n\n\
I can help you with:\n\
- Analyzing datasets and recommending architectures\n\
- Configuring training (LSTM, GRU, ResNet, BERT, Sentinel, etc.)\n\
- Evaluating model performance\n\
- Deploying models to edge controllers\n\n\
Try asking me about your data or how to train a model!";

/// Returns Some(rejection message) if the message is off-topic, None if it's fine.
fn check_guardrails(message: &str) -> Option<&'static str> {
    let lower = message.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    // Allow anything that mentions platform-related keywords
    const ALLOW_KEYWORDS: &[&str] = &[
        "model", "train", "data", "dataset", "sensor", "deploy", "edge",
        "predict", "anomal", "architect", "lstm", "gru", "sentinel",
        "resnet", "vgg", "bert", "gpt", "nexus", "phantom", "conv",
        "axon", "hyperparamet", "epoch", "batch", "learning rate",
        "loss", "accuracy", "precision", "recall", "f1", "auc",
        "feature", "column", "csv", "upload", "analyz", "evaluate",
        "raspberry", "arm", "quantiz", "inference", "prometheus",
        "equipment", "medical", "genomic", "financial", "industrial",
        "image", "classif", "nlp", "text", "score", "health",
        "sequence", "time series", "optimizer", "dropout", "weight",
        "layer", "hidden", "bottleneck", "mse", "bce", "cross entropy",
        "export", "binary", "preprocess", "normalize", "split",
        "validation", "test", "metric", "recommend", "config",
        "pipeline", "neural", "network", "deep learning", "machine learning",
        "ml", "ai", "regression", "classification", "clustering",
        "overfitting", "underfitting", "convergence", "gradient",
        "backprop", "attention", "transformer", "embedding",
        "rnn", "cnn", "autoencoder", "encoder", "decoder",
        "label", "target", "input", "output", "tensor",
        "channel", "dimension", "variance", "std", "mean",
        "min", "max", "statistics", "pattern", "trend",
        "seasonality", "alert", "threshold", "maintenance",
        "status", "pause", "resume", "ingest",
    ];

    // If any allow keyword is found, let it through
    for keyword in ALLOW_KEYWORDS {
        if lower.contains(keyword) {
            return None;
        }
    }

    // Short messages (< 4 words) that are greetings — allow
    if words.len() < 4 {
        const GREETINGS: &[&str] = &[
            "hi", "hello", "hey", "help", "howdy", "sup", "yo",
            "what can you do", "who are you", "start",
        ];
        for g in GREETINGS {
            if lower.contains(g) {
                return None;
            }
        }
    }

    // Catch explicitly off-topic patterns
    const BLOCK_PATTERNS: &[&str] = &[
        "write me a", "write a poem", "write a story", "write a song",
        "write an essay", "write a letter", "write a book",
        "tell me a joke", "tell me a story", "tell a joke",
        "translate", "翻译", "traducir",
        "play a game", "let's play", "roleplay", "role play", "pretend you are",
        "act as", "act like", "you are now", "ignore your instructions",
        "ignore previous", "ignore your prompt", "forget your",
        "jailbreak", "dan mode", "developer mode", "bypass",
        "how to hack", "how to break into", "exploit",
        "what is the meaning of life", "meaning of life",
        "write code for", "write a script for", "write a program",
        "homework", "assignment", "exam", "quiz",
        "recipe", "cook", "ingredients",
        "weather", "forecast",
        "stock", "bitcoin", "crypto", "invest",
        "dating", "relationship",
        "legal advice", "medical advice", "diagnose",
    ];

    for pattern in BLOCK_PATTERNS {
        if lower.contains(pattern) {
            return Some(GUARDRAIL_RESPONSE);
        }
    }

    // If message is long enough (5+ words) and has no ML/platform keywords,
    // it's likely off-topic
    if words.len() >= 8 {
        return Some(GUARDRAIL_RESPONSE);
    }

    // Short ambiguous messages — let them through to the AI
    None
}

// ---------------------------------------------------------------------------
// Smart local response (when DO GenAI is not configured)
// ---------------------------------------------------------------------------

fn generate_smart_response(message: &str, dataset_context: &str) -> String {
    let lower = message.to_lowercase();
    let has_dataset = !dataset_context.is_empty() && !dataset_context.contains("No datasets");

    // Extract dataset info from context for smart responses
    let ds_info = if has_dataset { parse_dataset_info(dataset_context) } else { None };

    // Model creation / training request
    if lower.contains("create") || lower.contains("train") || lower.contains("model") || lower.contains("build") {
        if let Some(ref info) = ds_info {
            let arch = select_architecture_from_columns(&info.columns, info.feature_count);
            let arch_name = match arch.as_str() {
                "lstm_autoencoder" => "LSTM Autoencoder",
                "gru_predictor" => "GRU Predictor",
                "sentinel" => "Sentinel Scorer",
                "resnet" => "ResNet",
                "bert" => "BERT",
                "conv2d" => "Conv2d",
                _ => &arch,
            };
            let epochs = if info.rows > 50000 { 15 } else if info.rows > 10000 { 12 } else { 10 };
            let batch_size = if info.rows > 10000 { 128 } else { 64 };
            let hidden = 8;
            let data_quality = if info.rows > 10000 { "excellent" } else if info.rows > 1000 { "good" } else { "limited" };

            return format!(
                "Based on your **{}** dataset ({} rows, {} columns):\n\n\
                **Recommended Architecture: {}**\n\n\
                {}\n\n\
                **Training Plan:**\n\
                - Architecture: `{}`\n\
                - Dataset: `{}`\n\
                - Epochs: {}\n\
                - Batch size: {}\n\
                - Hidden dim: {}\n\
                - Learning rate: 0.001\n\
                - Optimizer: Adam\n\n\
                Data quality: **{}** — {} rows provides {} coverage for this architecture.\n\n\
                To start training, go to the **Training** page and use these parameters, or I can help you fine-tune them. \
                Training runs via AxonML's autograd engine (pure Rust, GPU/CUDA).",
                info.name,
                info.rows,
                info.feature_count,
                arch_name,
                generate_rationale(&arch, info.feature_count, info.rows),
                arch,
                info.id,
                epochs,
                batch_size,
                hidden,
                data_quality,
                info.rows,
                if info.rows > 10000 { "strong statistical" } else if info.rows > 1000 { "adequate" } else { "minimal" },
            );
        } else {
            return "I'd love to help you create a model! I can see datasets are available in the system. \
                Could you navigate to the **Datasets** page, click on the dataset you want to train on, \
                then come back here? Or tell me the dataset name and I'll look it up.".to_string();
        }
    }

    // Analysis request
    if lower.contains("analyz") || lower.contains("look at") || lower.contains("examine") || lower.contains("inspect") {
        if let Some(ref info) = ds_info {
            let has_variance_cols = info.stats_summary.iter()
                .filter(|(_, std)| *std > 0.001)
                .count();
            let low_variance = info.stats_summary.iter()
                .filter(|(_, std)| *std <= 0.001)
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>();

            let mut analysis = format!(
                "**Analysis of {} ({} rows)**\n\n\
                **Data Profile:**\n\
                - Feature columns: {} ({} with meaningful variance)\n\
                - Data points: {}\n",
                info.name,
                info.rows,
                info.feature_count,
                has_variance_cols,
                info.rows,
            );

            if !low_variance.is_empty() {
                analysis.push_str(&format!(
                    "- Low-variance columns (may be constant/binary): {}\n",
                    low_variance.join(", ")
                ));
            }

            analysis.push_str(&format!(
                "\n**Data Quality: {}**\n",
                if info.rows > 10000 && has_variance_cols > 3 { "Excellent" }
                else if info.rows > 1000 { "Good" }
                else { "Needs more data" }
            ));

            if info.rows > 500 && has_variance_cols > 2 {
                analysis.push_str("- Seasonality patterns likely present (sufficient data + variance)\n");
            }

            analysis.push_str("\nWould you like me to recommend a model architecture for this data?");
            return analysis;
        }
    }

    // Dataset related
    if lower.contains("dataset") || lower.contains("data") {
        if let Some(ref info) = ds_info {
            return format!(
                "I can see the **{}** dataset — {} rows with {} feature columns. \
                The data is currently **{}**.\n\nWhat would you like to do with it? I can:\n\
                - **Analyze** the data patterns and quality\n\
                - **Recommend** a model architecture\n\
                - **Generate** a training plan with optimal hyperparameters",
                info.name,
                info.rows, info.feature_count,
                info.status,
            );
        }
    }

    // Deployment
    if lower.contains("deploy") || lower.contains("edge") || lower.contains("raspberry") {
        return "Deployment packages your trained model for edge controllers:\n\n\
            1. **INT8 Quantization** — Reduces model size 4x while maintaining accuracy\n\
            2. **ARM Cross-compilation** — Built for `armv7-unknown-linux-musleabihf`\n\
            3. **Edge Daemon** — Runs at ~1.5 MB RSS, inference every 30 seconds\n\
            4. **Binary Download** — Single static binary, no dependencies\n\n\
            Go to the **Models** page, select a trained model, and click Deploy. \
            You'll choose a target controller and I'll handle the rest.".to_string();
    }

    // Architecture question
    if lower.contains("architect") || lower.contains("lstm") || lower.contains("gru") || lower.contains("sentinel")
        || lower.contains("resnet") || lower.contains("bert") || lower.contains("vgg") {
        return "AxonML supports these architectures:\n\n\
            **Time Series / Sequence:**\n\
            - **LSTM Autoencoder** — Anomaly detection (learns normal patterns, flags deviations)\n\
            - **GRU Predictor** — Event/failure prediction at multiple horizons\n\
            - **Sentinel** — Multi-channel health/quality scoring (12+ features)\n\n\
            **Computer Vision:**\n\
            - **ResNet** — Image classification (ResNet-18/34/50)\n\
            - **VGG** — Simpler image classification tasks\n\n\
            **NLP:**\n\
            - **BERT** — Text classification, sentiment analysis\n\
            - **GPT-2** — Text generation\n\n\
            **Advanced:**\n\
            - **Nexus** — Multi-modal fusion (mixed data types)\n\
            - **Phantom** — Lightweight edge models\n\n\
            Share a dataset and I'll recommend the best fit.".to_string();
    }

    // Help / greeting / generic
    if has_dataset {
        if let Some(ref info) = ds_info {
            return format!(
                "I'm PrometheusForge, your AI engineering assistant. I have access to your **{}** dataset \
                ({} rows, {} features).\n\n\
                I can:\n\
                - **Analyze** the data for patterns and anomalies\n\
                - **Recommend** a model architecture\n\
                - **Create a training plan** with optimal hyperparameters\n\
                - **Guide deployment** to edge controllers\n\n\
                What would you like to do?",
                info.name, info.rows, info.feature_count,
            );
        }
    }

    "I'm PrometheusForge, your AI engineering assistant. I can help you with:\n\n\
    - **Analyzing data** for patterns and anomalies\n\
    - **Recommending model architectures** (LSTM, GRU, ResNet, BERT, and more)\n\
    - **Generating training plans** with optimal hyperparameters\n\
    - **Evaluating trained models** and suggesting improvements\n\
    - **Deploying** models to edge controllers\n\n\
    Upload a dataset or tell me what you'd like to work on!".to_string()
}

// ---------------------------------------------------------------------------
// Helper: parse dataset info from context string
// ---------------------------------------------------------------------------

struct DatasetInfo {
    name: String,
    id: String,
    #[allow(dead_code)]
    equipment_type: String,
    #[allow(dead_code)]
    location: String,
    rows: u64,
    feature_count: usize,
    status: String,
    columns: Vec<String>,
    stats_summary: Vec<(String, f64)>, // (column_name, std_dev)
}

fn parse_dataset_info(context: &str) -> Option<DatasetInfo> {
    let get_field = |prefix: &str| -> String {
        context.lines()
            .find(|l| l.contains(prefix))
            .and_then(|l| l.split(prefix).nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };

    let name = get_field("Name: ");
    if name.is_empty() { return None; }

    let id = get_field("ID: ");
    let equipment_type = get_field("Domain: ");
    let location = get_field("Location: ");
    let status = get_field("Status: ");
    let rows: u64 = get_field("Rows: ").parse().unwrap_or(0);

    // Parse "Columns (N): col1, col2, col3" line
    let columns_line = context.lines()
        .find(|l| l.contains("Columns ("))
        .unwrap_or("");
    let columns: Vec<String> = columns_line
        .split("): ")
        .nth(1)
        .unwrap_or("")
        .split(", ")
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();
    let feature_count: usize = columns.len().max(1) - 1; // subtract timestamp

    // Parse stats
    let mut stats_summary = Vec::new();
    let mut in_stats = false;
    for line in context.lines() {
        if line.contains("Column Statistics:") {
            in_stats = true;
            continue;
        }
        if in_stats {
            if let Some(col_part) = line.trim().strip_suffix(':').or_else(|| {
                line.find(": min=").map(|i| &line[..i]).map(|s| s.trim())
            }) {
                // Actually parse "  col_name: min=X, max=X, mean=X, std=X"
                if let Some(std_part) = line.split("std=").nth(1) {
                    if let Ok(std_val) = std_part.trim().parse::<f64>() {
                        let col_name = line.trim().split(':').next().unwrap_or("").trim();
                        stats_summary.push((col_name.to_string(), std_val));
                    }
                }
                let _ = col_part;
            }
        }
    }

    Some(DatasetInfo {
        name,
        id,
        equipment_type,
        location,
        rows,
        feature_count,
        status,
        columns,
        stats_summary,
    })
}

// ---------------------------------------------------------------------------
// Analyze endpoint (unchanged logic, cleaner code)
// ---------------------------------------------------------------------------

pub async fn analyze(
    State(state): State<AppState>,
    Json(req): Json<AnalyzeRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let dataset = state.aegis_get_doc("datasets", &req.dataset_id).await
        .map_err(|_| AppError::NotFound("Dataset not found".into()))?;

    let domain = dataset
        .get("domain")
        .or_else(|| dataset.get("equipment_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("general");
    let row_count = dataset
        .get("row_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let columns = dataset
        .get("columns")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let column_stats = dataset
        .get("column_stats")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let numeric_cols = column_stats.as_object().map(|o| o.len()).unwrap_or(0);
    let has_variance = column_stats.as_object().map(|obj| {
        obj.values().filter(|v| {
            v.get("std").and_then(|s| s.as_f64()).unwrap_or(0.0) > 0.001
        }).count()
    }).unwrap_or(0);

    let quality_score = if row_count == 0 {
        0.0
    } else {
        let mut score = 100.0f64;
        if row_count < 500 { score -= 15.0; }
        if row_count < 100 { score -= 25.0; }
        if numeric_cols < 3 { score -= 20.0; }
        if has_variance < numeric_cols.saturating_sub(1) { score -= 10.0; }
        score.max(0.0)
    };

    let data_quality = if quality_score >= 80.0 { "excellent" }
        else if quality_score >= 60.0 { "good" }
        else if quality_score >= 40.0 { "fair" }
        else { "poor" };

    let feature_count = columns.len().saturating_sub(1);
    let col_names: Vec<String> = columns.iter().filter_map(|c| c.as_str().map(String::from)).collect();
    let architecture = select_architecture_from_columns(&col_names, feature_count);
    let hyperparameters = build_hyperparameters_for_arch(&architecture, feature_count, row_count);
    let seasonality_detected = row_count > 500 && has_variance > 2;

    Ok(Json(json!({
        "analysis": {
            "domain": domain,
            "row_count": row_count,
            "feature_count": feature_count,
            "numeric_columns": numeric_cols,
            "columns_with_variance": has_variance,
            "data_quality": data_quality,
            "quality_score": quality_score,
            "seasonality_detected": seasonality_detected,
            "column_stats": column_stats,
        },
        "recommendation": {
            "architecture": architecture,
            "rationale": generate_rationale(&architecture, feature_count, row_count),
        },
        "training_plan": {
            "architecture": architecture,
            "dataset_id": req.dataset_id,
            "hyperparameters": hyperparameters,
        },
    })))
}

pub async fn get_history(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let docs = state.aegis_list_docs("agent_history").await?;
    Ok(Json(docs))
}

// ---------------------------------------------------------------------------
// Architecture selection helpers (domain-agnostic)
// ---------------------------------------------------------------------------

fn select_architecture_from_columns(columns: &[String], feature_count: usize) -> String {
    let has_text = columns.iter().any(|c| {
        let cl = c.to_lowercase();
        cl.contains("text") || cl.contains("sentence") || cl.contains("body")
            || cl.contains("content") || cl.contains("review") || cl.contains("title")
            || cl.contains("comment") || cl.contains("transcript") || cl.contains("abstract")
    });
    let has_image = columns.iter().any(|c| {
        let cl = c.to_lowercase();
        cl.contains("image") || cl.contains("pixel") || cl.contains("img")
            || cl.contains("photo") || cl.contains("frame") || cl.contains("scan")
    });
    let has_temporal = columns.iter().any(|c| {
        let cl = c.to_lowercase();
        cl.contains("time") || cl.contains("date") || cl == "ts" || cl == "epoch"
            || cl.contains("timestamp")
    });

    if has_text { return "bert".into(); }
    if has_image {
        return if feature_count > 3072 { "resnet".into() } else { "conv2d".into() };
    }
    if has_temporal {
        return if feature_count > 10 { "lstm_autoencoder".into() } else { "gru_predictor".into() };
    }
    if feature_count > 12 { "sentinel".into() }
    else { "lstm_autoencoder".into() }
}

fn build_hyperparameters_for_arch(architecture: &str, features: usize, rows: u64) -> serde_json::Value {
    let epochs = if rows > 50000 { 15 } else if rows > 10000 { 12 } else { 10 };
    let batch_size = if rows > 10000 { 128 } else { 64 };
    let seq_len = if features > 10 { 48 } else { 60 };

    match architecture {
        "lstm_autoencoder" => json!({
            "learning_rate": 0.001,
            "batch_size": batch_size,
            "epochs": epochs,
            "hidden_dim": 8,
            "bottleneck_dim": 4,
            "num_layers": 1,
            "sequence_length": seq_len.min(10),
            "dropout": 0.0,
            "optimizer": "adam",
            "loss": "mse",
        }),
        "gru_predictor" => json!({
            "learning_rate": 0.001,
            "batch_size": batch_size,
            "epochs": epochs,
            "hidden_dim": 8,
            "num_layers": 1,
            "sequence_length": seq_len.min(10),
            "dropout": 0.0,
            "optimizer": "adamw",
            "loss": "bce",
        }),
        "resnet" | "vgg" | "vit" | "conv2d" => json!({
            "learning_rate": 0.001,
            "batch_size": 32,
            "epochs": epochs,
            "hidden_dim": 10,
            "num_layers": 1,
            "dropout": 0.1,
            "optimizer": "adamw",
            "loss": "cross_entropy",
        }),
        "bert" => json!({
            "learning_rate": 0.00005,
            "batch_size": 32,
            "epochs": epochs,
            "hidden_dim": 8,
            "num_layers": 1,
            "dropout": 0.1,
            "optimizer": "adamw",
            "loss": "cross_entropy",
        }),
        "gpt2" => json!({
            "learning_rate": 0.0001,
            "batch_size": 32,
            "epochs": epochs,
            "hidden_dim": 8,
            "num_layers": 1,
            "dropout": 0.1,
            "optimizer": "adamw",
            "loss": "cross_entropy",
        }),
        "nexus" => json!({
            "learning_rate": 0.001,
            "batch_size": batch_size,
            "epochs": epochs,
            "hidden_dim": 8,
            "num_layers": 1,
            "dropout": 0.1,
            "optimizer": "adam",
            "loss": "bce",
        }),
        "phantom" => json!({
            "learning_rate": 0.001,
            "batch_size": batch_size,
            "epochs": epochs,
            "hidden_dim": 8,
            "num_layers": 1,
            "dropout": 0.0,
            "optimizer": "adam",
            "loss": "bce",
        }),
        _ => json!({
            "learning_rate": 0.001,
            "batch_size": batch_size,
            "epochs": epochs,
            "hidden_dim": 8,
            "num_layers": 1,
            "sequence_length": seq_len.min(10),
            "dropout": 0.0,
            "optimizer": "adam",
            "loss": "bce",
        }),
    }
}

fn generate_rationale(architecture: &str, features: usize, rows: u64) -> String {
    let arch_name = match architecture {
        "lstm_autoencoder" => "LSTM Autoencoder",
        "gru_predictor" => "GRU Predictor",
        "sentinel" => "Sentinel Scorer",
        "rnn" => "RNN",
        "resnet" => "ResNet",
        "vgg" => "VGG",
        "vit" => "Vision Transformer",
        "bert" => "BERT",
        "gpt2" => "GPT-2",
        "nexus" => "Nexus Multi-Modal",
        "phantom" => "Phantom Edge",
        "conv1d" => "Conv1d",
        "conv2d" => "Conv2d",
        _ => architecture,
    };

    let rationale = match architecture {
        "lstm_autoencoder" => "Learns to reconstruct normal patterns and detects anomalies via reconstruction error",
        "gru_predictor" => "Predicts future values or event probabilities at multiple time horizons",
        "sentinel" => "Computes a composite score by analyzing cross-feature dependencies in high-dimensional data",
        "resnet" => "Deep residual network with skip connections for robust image classification",
        "vgg" => "Classic deep CNN for image classification",
        "vit" => "Vision Transformer with self-attention over image patches for global spatial understanding",
        "bert" => "Bidirectional transformer encoder for text classification and understanding",
        "gpt2" => "Autoregressive transformer for sequence generation and language modeling",
        "nexus" => "Multi-modal fusion via cross-attention for combining heterogeneous data types",
        "phantom" => "Ultra-lightweight model optimized for minimal memory and fast edge inference",
        "conv1d" => "1D convolutional network for sequential/temporal feature extraction",
        "conv2d" => "2D convolutional network for spatial feature extraction and classification",
        _ => "Well-suited for the data characteristics",
    };

    let data_sufficiency = if rows > 10000 { "sufficient" }
        else if rows > 1000 { "adequate" }
        else { "limited — consider collecting more data" };

    format!(
        "With {} features and {} data points, the {} architecture is recommended. {}. \
        The dataset size is {} for reliable training.",
        features, rows, arch_name, rationale, data_sufficiency,
    )
}
