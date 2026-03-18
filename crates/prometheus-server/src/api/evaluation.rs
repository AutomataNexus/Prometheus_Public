// ============================================================================
// File: evaluation.rs
// Description: Model evaluation CRUD — create, list, and retrieve evaluation results
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

pub async fn list_evaluations(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let docs = state.aegis_list_docs("evaluations").await?;
    if auth.is_admin() {
        return Ok(Json(docs));
    }
    let filtered = docs.into_iter().filter(|d| {
        d.get("created_by").and_then(|v| v.as_str()) == Some(&auth.user_id)
    }).collect();
    Ok(Json(filtered))
}

pub async fn get_evaluation(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("evaluations", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    Ok(Json(doc))
}

pub async fn run_gradient_eval(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    // Load the model to get its metrics
    let model = state.aegis_get_doc("models", &id).await
        .map_err(|_| AppError::NotFound(format!("Model {} not found", id)))?;

    let model_metrics = model.get("metrics").cloned().unwrap_or_else(|| json!({}));
    let architecture = model.get("architecture").and_then(|v| v.as_str()).unwrap_or("unknown");

    // If Gradient AI is configured, run ADK evaluation
    if let Some(ref api_key) = state.config.gradient_api_key {
        if let Some(ref agent_id) = state.config.gradient_agent_id {
            let resp = state
                .http_client
                .post(format!("https://cluster.digitalocean.com/v1/agents/{agent_id}/evaluate"))
                .header("Authorization", format!("Bearer {api_key}"))
                .json(&json!({
                    "model_id": id,
                    "model_metrics": model_metrics,
                    "architecture": architecture,
                }))
                .send()
                .await;

            if let Ok(resp) = resp {
                if resp.status().is_success() {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        // Store evaluation result
                        let eval_id = format!("eval_{}", &Uuid::new_v4().to_string()[..8]);
                        let eval_doc = json!({
                            "id": eval_id,
                            "model_id": id,
                            "source": "gradient_ai",
                            "gradient_metrics": body,
                            "model_metrics": model_metrics,
                            "created_by": auth.user_id,
                            "created_at": Utc::now().to_rfc3339(),
                        });
                        let _ = state.aegis_create_doc("evaluations", eval_doc).await;
                        return Ok(Json(body));
                    }
                }
            }
        }
    }

    // Local evaluation using stored model metrics
    let precision = model_metrics.get("precision").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let recall = model_metrics.get("recall").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let f1 = model_metrics.get("f1").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let val_loss = model_metrics.get("val_loss").and_then(|v| v.as_f64()).unwrap_or(1.0);

    // Derive quality assessment from model metrics
    let deploy_ready = f1 > 0.85 && val_loss < 0.05;
    let quality_tier = if f1 > 0.95 { "excellent" }
        else if f1 > 0.90 { "good" }
        else if f1 > 0.80 { "acceptable" }
        else { "needs_improvement" };

    let eval_id = format!("eval_{}", &Uuid::new_v4().to_string()[..8]);
    let evaluation = json!({
        "evaluation_id": eval_id,
        "model_id": id,
        "source": "local",
        "created_by": auth.user_id,
        "model_metrics": {
            "precision": precision,
            "recall": recall,
            "f1": f1,
            "val_loss": val_loss,
        },
        "assessment": {
            "quality_tier": quality_tier,
            "deploy_ready": deploy_ready,
            "recommendations": generate_recommendations(architecture, precision, recall, f1, val_loss),
        },
        "created_at": Utc::now().to_rfc3339(),
    });

    // Store evaluation
    let _ = state.aegis_create_doc("evaluations", evaluation.clone()).await;

    Ok(Json(evaluation))
}

fn generate_recommendations(
    architecture: &str,
    precision: f64,
    recall: f64,
    f1: f64,
    val_loss: f64,
) -> Vec<String> {
    let mut recs = Vec::new();

    if f1 < 0.80 {
        recs.push("F1 score is below 0.80. Consider collecting more training data or adjusting the learning rate.".into());
    }
    if precision < recall - 0.1 {
        recs.push("Precision is significantly lower than recall. The model may be generating too many false positives. Consider increasing the anomaly threshold.".into());
    }
    if recall < precision - 0.1 {
        recs.push("Recall is significantly lower than precision. The model may be missing anomalies. Consider lowering the anomaly threshold or using a lower learning rate.".into());
    }
    if val_loss > 0.1 {
        recs.push("Validation loss is high. The model may be underfitting. Try increasing the hidden dimension or number of layers.".into());
    }
    if val_loss < 0.001 {
        recs.push("Validation loss is very low. Check for potential overfitting by comparing train and validation loss curves.".into());
    }
    if architecture == "lstm_autoencoder" && f1 < 0.85 {
        recs.push("For LSTM Autoencoder, try adjusting the bottleneck dimension — too small may lose important features, too large may not compress enough.".into());
    }
    if recs.is_empty() {
        recs.push("Model metrics look good. The model is ready for edge deployment.".into());
    }

    recs
}
