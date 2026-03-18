// ============================================================================
// File: service_accounts.rs
// Description: Service account CRUD proxied to Aegis-DB with admin-only access control
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::extract::State;
use axum::Json;
use serde_json::json;
use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// List service accounts (proxies to Aegis-DB).
pub async fn list_service_accounts(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
) -> AppResult<Json<serde_json::Value>> {
    if !auth_user.is_admin() {
        return Err(AppError::Forbidden("Admin access required".into()));
    }

    let token = &auth_user.token;
    let resp = state
        .http_client
        .get(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {token}"))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::AegisDb(format!("Failed to list users: {body}")));
    }

    let users: serde_json::Value = resp.json().await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    Ok(Json(users))
}

/// Create a service account (proxies to Aegis-DB user creation).
pub async fn create_service_account(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    Json(req): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    if !auth_user.is_admin() {
        return Err(AppError::Forbidden("Admin access required".into()));
    }

    let username = req.get("username").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("username required".into()))?;
    let password = req.get("password").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("password required".into()))?;
    let role = req.get("role").and_then(|v| v.as_str()).unwrap_or("operator");
    let email = req.get("email").and_then(|v| v.as_str()).unwrap_or("");

    let token = &auth_user.token;
    let resp = state
        .http_client
        .post(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "username": username,
            "email": if email.is_empty() { format!("{username}@prometheus.local") } else { email.to_string() },
            "password": password,
            "role": role,
        }))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !status.is_success() {
        let err_msg = body.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        return Err(AppError::BadRequest(err_msg.to_string()));
    }

    Ok(Json(body))
}

/// Delete a service account (proxies to Aegis-DB).
pub async fn delete_service_account(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    axum::extract::Path(username): axum::extract::Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    if !auth_user.is_admin() {
        return Err(AppError::Forbidden("Admin access required".into()));
    }

    let token = &auth_user.token;
    let resp = state
        .http_client
        .delete(format!("{}/api/v1/admin/users/{username}", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {token}"))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::AegisDb(format!("Failed to delete user: {body}")));
    }

    Ok(Json(json!({ "deleted": username })))
}
