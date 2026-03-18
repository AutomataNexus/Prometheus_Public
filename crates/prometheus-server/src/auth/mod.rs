// ============================================================================
// File: mod.rs
// Description: Authentication module — login proxy to Aegis-DB and submodule declarations
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

pub mod middleware;
pub mod models;

use axum::{extract::State, Json};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use models::*;

/// Login — proxy to Aegis-DB and return opaque bearer token.
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let resp = state
        .http_client
        .post(format!("{}/api/v1/auth/login", state.config.aegis_db_url))
        .json(&serde_json::json!({
            "username": req.username,
            "password": req.password,
        }))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(AppError::RateLimited);
    }

    if !resp.status().is_success() {
        return Err(AppError::Unauthorized("Invalid credentials".into()));
    }

    let aegis_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    // Pass through the Aegis-DB opaque bearer token
    let token = aegis_resp
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::AegisDb("No token in response".into()))?
        .to_string();

    let user_id = aegis_resp
        .get("user")
        .and_then(|u| u.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("1")
        .to_string();

    let user = User {
        id: user_id.clone(),
        username: req.username.clone(),
        email: aegis_resp
            .get("user")
            .and_then(|u| u.get("email"))
            .and_then(|v| v.as_str())
            .map(String::from),
        role: aegis_resp
            .get("user")
            .and_then(|u| u.get("role"))
            .and_then(|v| v.as_str())
            .map(|r| match r {
                "admin" => Role::Admin,
                "operator" => Role::Operator,
                _ => Role::Viewer,
            })
            .unwrap_or(Role::Operator),
    };

    // Check email verification and account approval status
    let user_status = state
        .aegis_get_doc("user_status", &user_id)
        .await
        .ok();

    let email_verified = user_status
        .as_ref()
        .and_then(|s| s.get("email_verified").and_then(|v| v.as_bool()));

    let account_approved = user_status
        .as_ref()
        .and_then(|s| s.get("account_approved").and_then(|v| v.as_bool()));

    // Block login if email not verified
    if email_verified == Some(false) {
        return Err(AppError::Forbidden(
            "Email not verified. Please check your email for a verification code.".into(),
        ));
    }

    // Block login if account not approved (but email IS verified)
    if email_verified == Some(true) && account_approved == Some(false) {
        return Err(AppError::Forbidden(
            "Account pending admin approval. You will be notified when approved.".into(),
        ));
    }

    // Check if MFA is enabled
    let mfa_required = state
        .aegis_get_doc("mfa_secrets", &user_id)
        .await
        .ok()
        .and_then(|s| s.get("verified").and_then(|v| v.as_bool()));

    Ok(Json(LoginResponse {
        token,
        user,
        mfa_required,
        email_verified,
        account_approved,
    }))
}

/// Logout — proxy to Aegis-DB to invalidate session.
pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(auth) = headers.get("Authorization").and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            let _ = state
                .http_client
                .post(format!("{}/api/v1/auth/logout", state.config.aegis_db_url))
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await;
        }
    }
    Ok(Json(serde_json::json!({ "message": "Logged out" })))
}

/// Get session info — validate token against Aegis-DB.
pub async fn get_session(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> AppResult<Json<SessionInfo>> {
    let auth = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !auth.starts_with("Bearer ") {
        return Ok(Json(SessionInfo { valid: false, user: None }));
    }

    let token = &auth[7..];
    let resp = state
        .http_client
        .get(format!("{}/api/v1/auth/me", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !resp.status().is_success() {
        return Ok(Json(SessionInfo { valid: false, user: None }));
    }

    let u: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if u.is_null() {
        return Ok(Json(SessionInfo { valid: false, user: None }));
    }

    let user = Some(User {
        id: u.get("id").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
        username: u.get("username").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
        email: u.get("email").and_then(|v| v.as_str()).map(String::from),
        role: match u.get("role").and_then(|v| v.as_str()).unwrap_or("viewer") {
            "admin" => Role::Admin,
            "operator" => Role::Operator,
            _ => Role::Viewer,
        },
    });

    Ok(Json(SessionInfo { valid: true, user }))
}

/// Get current user info.
pub async fn get_me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> AppResult<Json<User>> {
    let session = get_session(State(state), headers).await?.0;
    session
        .user
        .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))
        .map(Json)
}
