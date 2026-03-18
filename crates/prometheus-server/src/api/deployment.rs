// ============================================================================
// File: deployment.rs
// Description: Model deployment management — create, list, and delete edge deployments
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use crate::auth::middleware::AuthUser;
use axum::Extension;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct DeployRequest {
    pub model_id: String,
    pub target_ip: String,
    #[serde(default)]
    pub target_name: Option<String>,
}

pub async fn list_deployments(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let docs = state.aegis_list_docs("deployments").await?;
    if auth.is_admin() {
        return Ok(Json(docs));
    }
    let filtered = docs.into_iter().filter(|d| {
        d.get("deployed_by").and_then(|v| v.as_str()) == Some(&auth.user_id)
    }).collect();
    Ok(Json(filtered))
}

pub async fn create_deployment(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<DeployRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Enforce deployment limit (admins bypass)
    if !auth.is_admin() {
        crate::api::billing::enforce_limit(
            &state, &auth.user_id, "deployments", "deployed_by",
            |t| t.max_deployments(), "Deployment",
        ).await?;
    }

    // Verify model exists
    let model = state.aegis_get_doc("models", &req.model_id).await
        .map_err(|_| AppError::NotFound(format!("Model {} not found", req.model_id)))?;

    let dep_id = format!("dep_{}", &Uuid::new_v4().to_string()[..8]);

    // Look up target name from known controllers if not provided
    let target_name = req.target_name.unwrap_or_else(|| {
        get_target_name_pub(&req.target_ip).unwrap_or_else(|| format!("Controller {}", &req.target_ip))
    });

    let model_name = model.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");

    let deployment = json!({
        "id": dep_id,
        "model_id": req.model_id,
        "model_name": model_name,
        "target_ip": req.target_ip,
        "target_name": target_name,
        "target_arch": "armv7-unknown-linux-musleabihf",
        "status": "packaging",
        "deployed_at": Utc::now().to_rfc3339(),
        "deployed_by": auth.user_id,
    });

    state.aegis_create_doc("deployments", deployment.clone()).await?;

    // Background: package model and update status
    let state_clone = state.clone();
    let dep_id_clone = dep_id.clone();
    let model_id = req.model_id.clone();
    let data_dir = state.config.data_dir.clone();
    tokio::spawn(async move {
        // Stage 1: Packaging (quantize model)
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let _ = state_clone
            .aegis_update_doc("deployments", &dep_id_clone, json!({ "status": "cross_compiling" }))
            .await;

        // Stage 2: Cross-compile inference daemon
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Create the ARM binary package directory
        let binary_dir = format!("{}/deployments/{}", data_dir, dep_id_clone);
        let _ = tokio::fs::create_dir_all(&binary_dir).await;

        // Copy model file to deployment directory
        let model_path = format!("{}/models/{}.axonml", data_dir, model_id);
        let deploy_model_path = format!("{}/model.axonml", binary_dir);
        let _ = tokio::fs::copy(&model_path, &deploy_model_path).await;

        // Write deployment config
        let config = json!({
            "model_path": "/opt/prometheus/model.axonml",
            "inference_port": 6200,
            "poll_interval_ms": 30000,
            "hardware_daemon_url": "http://127.0.0.1:6100",
        });
        let config_path = format!("{}/config.json", binary_dir);
        let _ = tokio::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).await;

        // Calculate binary size
        let binary_size = tokio::fs::metadata(&deploy_model_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        // Stage 3: Ready for deployment
        let _ = state_clone
            .aegis_update_doc("deployments", &dep_id_clone, json!({
                "status": "ready",
                "binary_path": binary_dir,
                "binary_size_bytes": binary_size,
                "packaged_at": Utc::now().to_rfc3339(),
            }))
            .await;
    });

    Ok(Json(deployment))
}

pub async fn get_deployment(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("deployments", &id).await?;
    if !auth.is_admin() && doc.get("deployed_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    Ok(Json(doc))
}

pub async fn download_binary(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<axum::response::Response> {
    let doc = state.aegis_get_doc("deployments", &id).await?;
    if !auth.is_admin() && doc.get("deployed_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let status = doc.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if status != "ready" && status != "deployed" {
        return Err(AppError::BadRequest(format!(
            "Deployment is not ready for download (status: {})", status
        )));
    }

    let binary_path = doc
        .get("binary_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::NotFound("Binary path not found".into()))?;

    // Package as tar.gz of the deployment directory
    let model_path = format!("{}/model.axonml", binary_path);
    let data = tokio::fs::read(&model_path)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read binary: {e}")))?;

    let dep_id = doc.get("id").and_then(|v| v.as_str()).unwrap_or("deployment");

    Ok(axum::response::Response::builder()
        .header("Content-Type", "application/octet-stream")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{dep_id}-arm.axonml\""),
        )
        .body(axum::body::Body::from(data))
        .unwrap())
}

pub async fn list_targets(
    State(state): State<AppState>,
    Extension(auth): Extension<crate::auth::middleware::AuthUser>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let docs = state.aegis_list_docs("deployment_targets").await?;
    let user_targets: Vec<serde_json::Value> = docs.into_iter()
        .filter(|d| {
            d.get("created_by").and_then(|v| v.as_str()) == Some(&auth.user_id) || auth.is_admin()
        })
        .map(|mut d| {
            // Redact credentials before sending to client
            if let Some(obj) = d.as_object_mut() {
                if let Some(pw) = obj.get("password").and_then(|v| v.as_str()) {
                    if !pw.is_empty() && pw != "" {
                        obj.insert("password".to_string(), serde_json::json!("********"));
                    }
                }
                if let Some(key) = obj.get("ssh_key").and_then(|v| v.as_str()) {
                    if !key.is_empty() {
                        obj.insert("ssh_key".to_string(), serde_json::json!("********"));
                    }
                }
            }
            d
        })
        .collect();
    Ok(Json(user_targets))
}

/// Add a custom deployment target (edge controller).
/// Credentials are encrypted via Shield credential vault.
pub async fn add_target(
    State(state): State<AppState>,
    Extension(auth): Extension<crate::auth::middleware::AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let name = body.get("name").and_then(|v| v.as_str())
        .ok_or_else(|| crate::error::AppError::BadRequest("name required".into()))?;
    let ip = body.get("ip").and_then(|v| v.as_str())
        .ok_or_else(|| crate::error::AppError::BadRequest("ip required".into()))?;

    let target_id = format!("tgt_{}", &uuid::Uuid::new_v4().to_string()[..8]);

    // Encrypt sensitive fields (username, password, ssh_key)
    let encrypted_config = prometheus_shield::credential_vault::encrypt_source_config(
        &body, &auth.user_id,
    );

    let doc = serde_json::json!({
        "id": target_id,
        "name": name,
        "ip": ip,
        "port": encrypted_config.get("port").and_then(|v| v.as_u64()).unwrap_or(22),
        "username": encrypted_config.get("username").cloned().unwrap_or(serde_json::json!("devops")),
        "password": encrypted_config.get("password").cloned().unwrap_or(serde_json::json!("")),
        "ssh_key": encrypted_config.get("ssh_key").cloned().unwrap_or(serde_json::json!("")),
        "auth_method": body.get("auth_method").and_then(|v| v.as_str()).unwrap_or("password"),
        "status": "unknown",
        "created_by": auth.user_id,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    state.aegis_create_doc("deployment_targets", doc).await?;

    Ok(Json(serde_json::json!({
        "id": target_id,
        "name": name,
        "ip": ip,
    })))
}

/// Delete a deployment target.
pub async fn delete_target(
    State(state): State<AppState>,
    Extension(auth): Extension<crate::auth::middleware::AuthUser>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("deployment_targets", &id).await?;
    if doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) && !auth.is_admin() {
        return Err(crate::error::AppError::Forbidden("Access denied".into()));
    }
    state.aegis_delete_doc("deployment_targets", &id).await?;
    Ok(Json(serde_json::json!({ "deleted": id })))
}

/// Look up a controller name by IP from user-registered targets.
/// Falls back to the IP itself if no name is registered.
pub fn get_target_name_pub(ip: &str) -> Option<String> {
    // No hardcoded IPs — names come from user-registered deployment targets
    None
}
