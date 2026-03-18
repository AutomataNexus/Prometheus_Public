// ============================================================================
// File: training.rs
// Description: Training job orchestration — start, stop, list, and queue management endpoints
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct StartTrainingRequest {
    pub dataset_id: String,
    #[serde(default)]
    pub architecture: Option<String>,
    #[serde(default)]
    pub hyperparameters: Option<serde_json::Value>,
    /// Model ID to resume training from (loads pre-trained weights).
    #[serde(default)]
    pub resume_from_model: Option<String>,
}

pub async fn list_training_runs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let docs = state.aegis_list_docs("training_plans").await?;
    if auth.is_admin() {
        return Ok(Json(docs));
    }
    let filtered = docs.into_iter().filter(|d| {
        d.get("user_id").and_then(|v| v.as_str()) == Some(&auth.user_id)
    }).collect();
    Ok(Json(filtered))
}

/// Get training queue status — active count, max capacity, queue depth.
pub async fn get_queue_status(
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let active_count = state.active_trainings.read().await.len();
    let queue_depth = state.training_queue.read().await.len();
    let max = state.config.max_concurrent_trainings;
    Ok(Json(json!({
        "active_trainings": active_count,
        "max_concurrent": max,
        "queued": queue_depth,
        "capacity_available": (max as usize).saturating_sub(active_count),
    })))
}

pub async fn start_training(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<StartTrainingRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Admins bypass all billing limits
    if !auth.is_admin() {
        // Enforce per-user concurrent training limit
        let tier = crate::api::billing::get_user_tier(&state, &auth.user_id).await;
        let max_trainings = tier.max_concurrent_trainings();
        if max_trainings != u32::MAX {
            let plans = state.aegis_list_docs("training_plans").await.unwrap_or_default();
            let user_active = plans.iter().filter(|p| {
                p.get("user_id").and_then(|v| v.as_str()) == Some(&auth.user_id)
                    && matches!(
                        p.get("status").and_then(|v| v.as_str()),
                        Some("running") | Some("queued")
                    )
            }).count() as u32;
            if user_active >= max_trainings {
                return Err(AppError::Forbidden(format!(
                    "Concurrent training limit reached ({}/{}). Upgrade your plan for more.",
                    user_active, max_trainings
                )));
            }
        }

        // Enforce model limit
        crate::api::billing::enforce_limit(
            &state, &auth.user_id, "models", "created_by",
            |t| t.max_models(), "Model",
        ).await?;

        // Deduct tokens for training (1 token per training run start)
        crate::api::billing::deduct_tokens(&state, &auth.user_id, 1).await?;
    }

    // Verify dataset exists and is validated
    let dataset_doc = state.aegis_get_doc("datasets", &req.dataset_id).await?;
    let is_validated = dataset_doc.get("is_validated").and_then(|v| v.as_bool()).unwrap_or(false);
    if !is_validated {
        return Err(AppError::BadRequest(
            "Dataset has not been validated. Please run validation from the dataset detail page before training.".into()
        ));
    }

    // Resolve resume-from model path if retraining
    let resume_from_path: Option<String> = if let Some(ref model_id_resume) = req.resume_from_model {
        let model_doc = state.aegis_get_doc("models", model_id_resume).await?;
        model_doc.get("file_path").and_then(|v| v.as_str()).map(String::from)
    } else {
        None
    };

    let run_id = format!("tr_{}", &Uuid::new_v4().to_string()[..8]);
    let model_id = format!("mdl_{}", &Uuid::new_v4().to_string()[..8]);
    let architecture = req.architecture.unwrap_or_else(|| "lstm_autoencoder".into());

    let hyperparameters = req.hyperparameters.unwrap_or_else(|| {
        json!({
            "learning_rate": 0.001,
            "batch_size": 64,
            "epochs": 100,
            "hidden_dim": 64,
            "bottleneck_dim": 32,
            "num_layers": 2,
            "sequence_length": 60,
            "dropout": 0.1,
            "optimizer": "adam",
            "loss": "mse",
        })
    });

    // Check server-wide capacity — queue if full
    let is_queued = {
        let active = state.active_trainings.read().await;
        active.len() as u32 >= state.config.max_concurrent_trainings
    };

    let status = if is_queued { "queued" } else { "running" };

    let run = json!({
        "id": run_id,
        "user_id": auth.user_id,
        "model_id": model_id,
        "dataset_id": req.dataset_id,
        "architecture": architecture,
        "hyperparameters": hyperparameters,
        "status": status,
        "current_epoch": 0,
        "total_epochs": hyperparameters.get("epochs").and_then(|v| v.as_u64()).unwrap_or(100),
        "best_val_loss": null,
        "training_time_seconds": 0,
        "started_at": Utc::now().to_rfc3339(),
        "completed_at": null,
        "epoch_metrics": [],
        "resume_from_model": req.resume_from_model,
        "resume_from_path": resume_from_path,
    });

    state.aegis_create_doc("training_plans", run.clone()).await?;

    if is_queued {
        // Add to queue — will be started when a slot opens
        let queued = crate::state::QueuedTraining {
            run_id: run_id.clone(),
            user_id: auth.user_id.clone(),
            dataset_id: req.dataset_id.clone(),
            architecture: architecture.clone(),
            hyperparameters: hyperparameters.clone(),
            queued_at: Utc::now().to_rfc3339(),
        };
        {
            let mut queue = state.training_queue.write().await;
            let pos = queue.len() + 1;
            queue.push_back(queued);
            tracing::info!("Training {run_id} queued at position {pos}");
        }

        // Send push notification that training is queued
        let _ = crate::api::push::notify_user_typed(
            &state,
            &auth.user_id,
            "Training Queued",
            &format!("Your training run {run_id} has been queued. You'll be notified when it starts."),
            &crate::api::push::NotificationType::TrainingQueued,
            Some(json!({ "training_id": run_id })),
        ).await;

        // Send email notification if enabled
        notify_training_queued_email(&state, &auth.user_id, &run_id).await;
    } else {
        // Start immediately
        spawn_training_job_async(&state, &run_id).await;
    }

    Ok(Json(run))
}

/// Spawn the actual training thread for a given run_id (already in Aegis-DB).
/// Must be called from an async context.
async fn spawn_training_job_async(state: &AppState, run_id: &str) {
    let state_clone = state.clone();
    let run_id_clone = run_id.to_string();
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    {
        let mut trainings = state.active_trainings.write().await;
        trainings.insert(
            run_id.to_string(),
            crate::state::TrainingHandle {
                id: run_id.to_string(),
                cancel_token: cancel_tx,
            },
        );
    }

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to create training runtime");
        rt.block_on(run_training_job(state_clone, run_id_clone, cancel_rx));
    });
}

/// Try to start the next queued training if there's capacity.
async fn drain_training_queue(state: &AppState) {
    let can_start = {
        let active = state.active_trainings.read().await;
        (active.len() as u32) < state.config.max_concurrent_trainings
    };

    if !can_start {
        return;
    }

    let next = {
        let mut queue = state.training_queue.write().await;
        queue.pop_front()
    };

    if let Some(queued) = next {
        tracing::info!("Dequeuing training {} for user {}", queued.run_id, queued.user_id);

        // Update status from "queued" to "running" in Aegis-DB
        let _ = state
            .aegis_update_doc("training_plans", &queued.run_id, json!({
                "status": "running",
                "started_at": Utc::now().to_rfc3339(),
            }))
            .await;

        // Notify user their training has started
        let _ = crate::api::push::notify_user_typed(
            state,
            &queued.user_id,
            "Training Started",
            &format!("Your queued training run {} is now running!", queued.run_id),
            &crate::api::push::NotificationType::TrainingStarted,
            Some(json!({ "training_id": queued.run_id })),
        ).await;

        // Send email notification if enabled
        notify_training_started_email(state, &queued.user_id, &queued.run_id).await;

        spawn_training_job_async(state, &queued.run_id).await;
    }
}

/// Send email notification if user has email_notifications enabled.
async fn send_training_email(state: &AppState, user_id: &str, subject: &str, html: &str) {
    let prefs = match state.aegis_get_doc("user_preferences", user_id).await {
        Ok(p) => p,
        Err(_) => return,
    };
    let email_enabled = prefs.get("email_notifications").and_then(|v| v.as_bool()).unwrap_or(true);
    let notifs_enabled = prefs.get("notifications_enabled").and_then(|v| v.as_bool()).unwrap_or(true);
    if !email_enabled || !notifs_enabled {
        return;
    }
    let email_svc = match prometheus_email::EmailService::from_env() {
        Ok(svc) => svc,
        Err(_) => return,
    };
    let email_addr = match get_user_email(state, user_id).await {
        Some(e) => e,
        None => return,
    };
    let _ = email_svc.send_notification(&email_addr, subject, html).await;
}

async fn notify_training_queued_email(state: &AppState, user_id: &str, run_id: &str) {
    let subject = format!("Training Queued: {}", run_id);
    let body = format!(
        "<h2>Training Run Queued</h2>\
        <p>Your training run <strong>{}</strong> has been added to the queue.</p>\
        <p>The server is currently at maximum training capacity. \
        You will be notified when your training starts.</p>",
        run_id
    );
    send_training_email(state, user_id, &subject, &body).await;
}

async fn notify_training_started_email(state: &AppState, user_id: &str, run_id: &str) {
    let subject = format!("Training Started: {}", run_id);
    let body = format!(
        "<h2>Training Run Started</h2>\
        <p>Your queued training run <strong>{}</strong> is now running!</p>\
        <p>You will receive another notification when training completes.</p>",
        run_id
    );
    send_training_email(state, user_id, &subject, &body).await;
}

/// Look up user email from Aegis-DB users collection.
async fn get_user_email(state: &AppState, user_id: &str) -> Option<String> {
    state.aegis_get_doc("users", user_id).await.ok()
        .and_then(|doc| doc.get("email").and_then(|v| v.as_str()).map(String::from))
}

pub async fn get_training_run(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("training_plans", &id).await?;
    if !auth.is_admin() && doc.get("user_id").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    Ok(Json(doc))
}

pub async fn stop_training(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("training_plans", &id).await?;
    if !auth.is_admin() && doc.get("user_id").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    // Send cancel signal to the training thread
    {
        let trainings = state.active_trainings.read().await;
        if let Some(handle) = trainings.get(&id) {
            let _ = handle.cancel_token.send(true);
        }
    }
    // Also update status immediately so the UI reflects it
    let _ = state.aegis_update_doc("training_plans", &id, json!({ "status": "cancelled" })).await;
    // Remove from active trainings
    {
        let mut trainings = state.active_trainings.write().await;
        trainings.remove(&id);
    }
    Ok(Json(json!({ "status": "cancelled", "id": id })))
}

/// Clear all completed/failed/cancelled training runs for the current user.
pub async fn clear_completed_training(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<serde_json::Value>> {
    let docs = state.aegis_list_docs("training_plans").await?;
    let mut cleared = 0u32;
    for doc in &docs {
        let owner = doc.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
        if !auth.is_admin() && owner != auth.user_id {
            continue;
        }
        let status = doc.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if matches!(status, "completed" | "failed" | "cancelled" | "stopped") {
            if let Some(id) = doc.get("id").and_then(|v| v.as_str()) {
                let _ = state.aegis_delete_doc("training_plans", id).await;
                cleared += 1;
            }
        }
    }
    Ok(Json(json!({ "cleared": cleared })))
}

async fn run_training_job(
    state: AppState,
    run_id: String,
    mut cancel_rx: tokio::sync::watch::Receiver<bool>,
) {
    // Get training run doc for model_id, dataset_id, architecture, hyperparameters
    let run_doc = state
        .aegis_get_doc("training_plans", &run_id)
        .await
        .ok();

    let model_id = run_doc
        .as_ref()
        .and_then(|d| d.get("model_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("mdl_unknown")
        .to_string();
    let user_id = run_doc
        .as_ref()
        .and_then(|d| d.get("user_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let dataset_id = run_doc
        .as_ref()
        .and_then(|d| d.get("dataset_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("ds_unknown")
        .to_string();
    let architecture_str = run_doc
        .as_ref()
        .and_then(|d| d.get("architecture"))
        .and_then(|v| v.as_str())
        .unwrap_or("lstm_autoencoder")
        .to_string();
    let hp_json = run_doc
        .as_ref()
        .and_then(|d| d.get("hyperparameters"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    // Resolve dataset file path from Aegis-DB
    let dataset_doc = state.aegis_get_doc("datasets", &dataset_id).await.ok();
    let dataset_path = dataset_doc
        .as_ref()
        .and_then(|ds| ds.get("file_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Decompress dataset if it's compressed
    let dataset_path = if dataset_path.ends_with(".zst") {
        crate::api::data_lifecycle::decompress_dataset(&state, &dataset_id)
            .await
            .unwrap_or(dataset_path)
    } else {
        dataset_path
    };

    if dataset_path.is_empty() {
        let _ = state
            .aegis_update_doc("training_plans", &run_id, json!({ "status": "failed", "error": "Dataset file not found" }))
            .await;
        {
            let mut trainings = state.active_trainings.write().await;
            trainings.remove(&run_id);
        }
        drain_training_queue(&state).await;
        return;
    }

    // Map architecture string to enum
    let architecture = match architecture_str.as_str() {
        "gru_predictor" => prometheus_training::Architecture::GruPredictor,
        "sentinel" => prometheus_training::Architecture::Sentinel,
        "rnn" => prometheus_training::Architecture::Rnn,
        "resnet" => prometheus_training::Architecture::ResNet,
        "vgg" => prometheus_training::Architecture::Vgg,
        "vit" | "vision_transformer" => prometheus_training::Architecture::ViT,
        "bert" => prometheus_training::Architecture::Bert,
        "gpt2" => prometheus_training::Architecture::Gpt2,
        "nexus" => prometheus_training::Architecture::Nexus,
        "phantom" => prometheus_training::Architecture::Phantom,
        "conv1d" => prometheus_training::Architecture::Conv1d,
        "conv2d" => prometheus_training::Architecture::Conv2d,
        _ => prometheus_training::Architecture::LstmAutoencoder,
    };

    // Build hyperparameters from JSON
    let hyperparameters = prometheus_training::Hyperparameters {
        learning_rate: hp_json.get("learning_rate").and_then(|v| v.as_f64()).unwrap_or(0.001),
        epochs: hp_json.get("epochs").and_then(|v| v.as_u64()).unwrap_or(100) as usize,
        batch_size: hp_json.get("batch_size").and_then(|v| v.as_u64()).unwrap_or(64) as usize,
        sequence_length: hp_json.get("sequence_length").and_then(|v| v.as_u64()).unwrap_or(60) as usize,
        hidden_dim: hp_json.get("hidden_dim").and_then(|v| v.as_u64()).unwrap_or(64) as usize,
        num_layers: hp_json.get("num_layers").and_then(|v| v.as_u64()).unwrap_or(2) as usize,
        dropout: hp_json.get("dropout").and_then(|v| v.as_f64()).unwrap_or(0.1),
        weight_decay: hp_json.get("weight_decay").and_then(|v| v.as_f64()).unwrap_or(0.01),
        early_stopping_patience: hp_json.get("early_stopping_patience").and_then(|v| v.as_u64()).unwrap_or(10) as usize,
        val_check_interval: 1,
    };

    // Count input features from dataset columns
    let input_features = dataset_doc
        .as_ref()
        .and_then(|ds| ds.get("columns"))
        .and_then(|v| v.as_array())
        .map(|a| a.len().saturating_sub(1)) // exclude timestamp column
        .unwrap_or(10);

    let output_path = format!("{}/models", state.config.data_dir);
    let total_epochs = hyperparameters.epochs as u64;
    let config = prometheus_training::TrainingConfig {
        run_id: run_id.clone(),
        architecture,
        hyperparameters,
        dataset_path,
        output_path,
        input_features,
        train_split: 0.7,
        val_split: 0.15,
        test_split: 0.15,
        quantize: false,
        cross_compile_target: None,
        resume_from: run_doc.as_ref()
            .and_then(|d| d.get("resume_from_path"))
            .and_then(|v| v.as_str())
            .map(String::from),
    };

    // Capture model metadata before config is moved into pipeline
    let cfg_input_features = config.input_features;
    let cfg_hidden_dim = config.hyperparameters.hidden_dim;
    let cfg_num_layers = config.hyperparameters.num_layers;
    let cfg_sequence_length = config.hyperparameters.sequence_length;
    let cfg_batch_size = config.hyperparameters.batch_size;
    let cfg_bottleneck = config.hyperparameters.hidden_dim / 2; // bottleneck is typically half hidden

    // Create channel for receiving training events
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<prometheus_training::TrainingEvent>(100);

    // Forward training events to Aegis-DB for WebSocket progress updates
    let state_for_events = state.clone();
    let run_id_for_events = run_id.clone();
    tokio::spawn(async move {
        let mut epoch_metrics = Vec::new();
        let mut best_val_loss = f64::MAX;

        while let Some(event) = event_rx.recv().await {
            match event {
                prometheus_training::TrainingEvent::EpochComplete { epoch, train_loss, val_loss, .. } => {
                    let vl = val_loss.unwrap_or(train_loss) as f64;
                    if vl < best_val_loss { best_val_loss = vl; }
                    epoch_metrics.push(json!({
                        "epoch": epoch,
                        "train_loss": train_loss as f64,
                        "val_loss": vl,
                    }));
                    {
                        let _ = state_for_events
                            .aegis_update_doc("training_plans", &run_id_for_events, json!({
                                "current_epoch": epoch + 1,
                                "best_val_loss": best_val_loss,
                                "epoch_metrics": epoch_metrics,
                            }))
                            .await;
                    }
                }
                prometheus_training::TrainingEvent::Error { message } => {
                    let _ = state_for_events
                        .aegis_update_doc("training_plans", &run_id_for_events, json!({ "status": "failed", "error": message }))
                        .await;
                }
                _ => {}
            }
        }
    });

    // Run the actual training pipeline with cancellation support
    let pipeline_result = tokio::select! {
        result = prometheus_training::run_pipeline(config, Some(event_tx)) => result,
        _ = async {
            loop {
                cancel_rx.changed().await.ok();
                if *cancel_rx.borrow() { break; }
            }
        } => {
            let _ = state
                .aegis_update_doc("training_plans", &run_id, json!({ "status": "cancelled" }))
                .await;
            {
                let mut trainings = state.active_trainings.write().await;
                trainings.remove(&run_id);
            }
            drain_training_queue(&state).await;
            return;
        }
    };

    match pipeline_result {
        Ok(result) => {
            let training_secs = result.training_duration.as_secs();

            // Mark training run completed
            let _ = state
                .aegis_update_doc("training_plans", &run_id, json!({
                    "status": "completed",
                    "current_epoch": result.epochs_trained,
                    "completed_at": Utc::now().to_rfc3339(),
                    "training_time_seconds": training_secs,
                }))
                .await;

            // Compute file size
            let file_size = tokio::fs::metadata(&result.artifact_path)
                .await
                .map(|m| m.len())
                .unwrap_or(0);

            // Create model document in Aegis-DB with real pipeline metrics
            let model_doc = json!({
                "id": model_id,
                "name": format!("Model {} \u{2014} {}", &model_id[4..], architecture_str),
                "architecture": architecture_str,
                "dataset_id": dataset_id,
                "training_run_id": run_id,
                "input_features": cfg_input_features,
                "hidden_dim": cfg_hidden_dim,
                "bottleneck_dim": cfg_bottleneck,
                "num_layers": cfg_num_layers,
                "sequence_length": cfg_sequence_length,
                "batch_size": cfg_batch_size,
                "epochs_trained": result.epochs_trained,
                "training_time_seconds": training_secs,
                "metrics": {
                    "val_loss": result.final_val_loss as f64,
                    "train_loss": result.final_train_loss as f64,
                    "precision": result.metrics.precision as f64,
                    "recall": result.metrics.recall as f64,
                    "f1": result.metrics.f1 as f64,
                    "accuracy": result.metrics.accuracy as f64,
                    "mse": result.metrics.mse as f64,
                    "mae": result.metrics.mae as f64,
                },
                "file_path": result.artifact_path,
                "file_size_bytes": file_size,
                "quantized": result.quantized_artifact_path.is_some(),
                "status": "ready",
                "created_by": user_id,
                "created_at": Utc::now().to_rfc3339(),
            });

            let _ = state.aegis_create_doc("models", model_doc).await;

            // Notify user via push notification
            if !user_id.is_empty() {
                let _ = crate::api::push::notify_user_typed(
                    &state,
                    &user_id,
                    "Training Complete",
                    &format!("Training run {run_id} finished successfully"),
                    &crate::api::push::NotificationType::TrainingComplete,
                    Some(json!({
                        "training_id": run_id,
                        "model_id": model_id,
                    })),
                )
                .await;
            }
        }
        Err(e) => {
            let _ = state
                .aegis_update_doc("training_plans", &run_id, json!({
                    "status": "failed",
                    "error": e.to_string(),
                }))
                .await;

            // Notify user of failure via push notification
            if !user_id.is_empty() {
                let _ = crate::api::push::notify_user_typed(
                    &state,
                    &user_id,
                    "Training Failed",
                    &format!("Training run {run_id} failed: {e}"),
                    &crate::api::push::NotificationType::TrainingFailed,
                    Some(json!({
                        "training_id": run_id,
                        "error": e.to_string(),
                    })),
                )
                .await;
            }
        }
    }

    // Remove from active trainings and drain queue
    {
        let mut trainings = state.active_trainings.write().await;
        trainings.remove(&run_id);
    }
    drain_training_queue(&state).await;
}
