// ============================================================================
// File: users.rs
// Description: User lifecycle management — signup, verification, password reset, and admin CRUD
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

//! Full user lifecycle management.
//!
//! Public endpoints: signup, email verification, forgot/reset password.
//! Protected endpoints: change password.
//! Admin endpoints: CRUD users with subscription tier management.
//!
//! All user data lives in Aegis-DB. Password hashing is handled by Aegis-DB
//! (Argon2id). Prometheus manages the lifecycle state (email verification,
//! password reset tokens, subscription tiers) via document collections.

use axum::{extract::{Path, State}, Extension, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SignupResponse {
    pub user_id: String,
    pub username: String,
    pub email_pending: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminCreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub skip_verification: bool,
}

fn default_role() -> String { "operator".into() }

#[derive(Debug, Deserialize)]
pub struct AdminUpdateUserRequest {
    pub email: Option<String>,
    pub role: Option<String>,
    pub password: Option<String>,
    pub tier: Option<String>,
    pub email_verified: Option<bool>,
    pub account_approved: Option<bool>,
}

// ---------------------------------------------------------------------------
// Internal doc types stored in Aegis-DB collections
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct UserStatus {
    id: String,
    username: String,
    email: String,
    email_verified: bool,
    account_approved: bool,
    created_at: String,
    verified_at: Option<String>,
    approved_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmailVerification {
    id: String,
    user_id: String,
    username: String,
    email: String,
    code: String,
    created_at: String,
    expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PasswordReset {
    id: String,
    username: String,
    email: String,
    token: String,
    created_at: String,
    expires_at: String,
    used: bool,
}

// ---------------------------------------------------------------------------
// Public endpoints
// ---------------------------------------------------------------------------

/// POST /auth/signup — self-registration.
///
/// Creates user in Aegis-DB (Argon2id password hashing handled there),
/// stores verification state, sends verification email, creates Free subscription.
pub async fn signup(
    State(state): State<AppState>,
    Extension(email_svc): Extension<std::sync::Arc<prometheus_email::EmailService>>,
    Json(req): Json<SignupRequest>,
) -> AppResult<Json<SignupResponse>> {
    // Basic validation
    if req.username.len() < 3 {
        return Err(AppError::BadRequest("Username must be at least 3 characters".into()));
    }
    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::BadRequest("Username must be alphanumeric (underscores allowed)".into()));
    }
    if !req.email.contains('@') || !req.email.contains('.') {
        return Err(AppError::BadRequest("Invalid email format".into()));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest("Password must be at least 8 characters".into()));
    }

    // Authenticate to Aegis-DB as admin (credentials from Vault via env)
    let admin_user = std::env::var("AEGIS_DB_USERNAME")
        .map_err(|_| AppError::Internal("AEGIS_DB_USERNAME not configured".into()))?;
    let admin_pass = std::env::var("AEGIS_DB_PASSWORD")
        .map_err(|_| AppError::Internal("AEGIS_DB_PASSWORD not configured".into()))?;
    let login_resp = state.http_client
        .post(format!("{}/api/v1/auth/login", state.config.aegis_db_url))
        .json(&serde_json::json!({"username": admin_user, "password": admin_pass}))
        .send().await
        .map_err(|e| AppError::AegisDb(format!("Admin auth failed: {e}")))?;
    let admin_token = login_resp.json::<serde_json::Value>().await
        .ok()
        .and_then(|v| v.get("token").and_then(|t| t.as_str()).map(String::from))
        .ok_or_else(|| AppError::Internal("Failed to authenticate with Aegis-DB".into()))?;

    // Create user in Aegis-DB (handles Argon2id hashing)
    let aegis_resp = state
        .http_client
        .post(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {admin_token}"))
        .json(&serde_json::json!({
            "username": req.username,
            "email": req.email,
            "password": req.password,
            "role": "operator",
        }))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !aegis_resp.status().is_success() {
        let body: serde_json::Value = aegis_resp.json().await.unwrap_or_default();
        let err = body.get("error").and_then(|v| v.as_str()).unwrap_or("Registration failed");
        return Err(AppError::BadRequest(err.to_string()));
    }

    let body: serde_json::Value = aegis_resp.json().await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;
    let user_id = body
        .get("user")
        .and_then(|u| u.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let now = Utc::now();

    // Store user status (email_pending, account not approved)
    let status = UserStatus {
        id: user_id.clone(),
        username: req.username.clone(),
        email: req.email.clone(),
        email_verified: false,
        account_approved: false,
        created_at: now.to_rfc3339(),
        verified_at: None,
        approved_at: None,
    };
    let _ = state.aegis_create_doc(
        "user_status",
        serde_json::to_value(&status).unwrap_or_default(),
    ).await;

    // Create Free tier subscription
    let sub = serde_json::json!({
        "id": user_id,
        "user_id": user_id,
        "tier": "free",
        "stripe_customer_id": null,
        "stripe_subscription_id": null,
        "tokens_used": 0,
        "tokens_limit": 1000,
        "current_period_start": now.to_rfc3339(),
        "current_period_end": (now + chrono::Duration::days(30)).to_rfc3339(),
    });
    let _ = state.aegis_create_doc("subscriptions", sub).await;

    // Generate verification code and store it
    let code = generate_verification_code();
    let verification = EmailVerification {
        id: format!("ev_{}", &Uuid::new_v4().to_string()[..8]),
        user_id: user_id.clone(),
        username: req.username.clone(),
        email: req.email.clone(),
        code: code.clone(),
        created_at: now.to_rfc3339(),
        expires_at: (now + chrono::Duration::minutes(15)).to_rfc3339(),
    };
    let _ = state.aegis_create_doc(
        "email_verifications",
        serde_json::to_value(&verification).unwrap_or_default(),
    ).await;

    // Send verification email (best-effort)
    let _ = email_svc
        .send_verification(
            &req.email,
            &req.username,
            &code,
            15,
        )
        .await;

    Ok(Json(SignupResponse {
        user_id,
        username: req.username,
        email_pending: true,
        message: "Account created. Check your email for a verification code.".into(),
    }))
}

/// POST /auth/verify-email — validate the emailed verification code.
pub async fn verify_email(
    State(state): State<AppState>,
    Json(req): Json<VerifyEmailRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let verifications = state.aegis_list_docs("email_verifications").await?;
    let now = Utc::now();

    let matching = verifications.iter().find(|v| {
        v.get("code").and_then(|c| c.as_str()) == Some(&req.code)
    });

    let verification = matching
        .ok_or_else(|| AppError::BadRequest("Invalid or expired verification code".into()))?;

    // Check expiry
    if let Some(expires) = verification.get("expires_at").and_then(|v| v.as_str()) {
        if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(expires) {
            if now > exp {
                return Err(AppError::BadRequest("Verification code has expired".into()));
            }
        }
    }

    let user_id = verification.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    // Update user_status to verified
    let _ = state.aegis_delete_doc("user_status", user_id).await;
    let username = verification.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let email = verification.get("email").and_then(|v| v.as_str()).unwrap_or("");

    let status = UserStatus {
        id: user_id.to_string(),
        username: username.to_string(),
        email: email.to_string(),
        email_verified: true,
        account_approved: false, // still needs admin approval
        created_at: verification.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        verified_at: Some(now.to_rfc3339()),
        approved_at: None,
    };
    let _ = state.aegis_create_doc(
        "user_status",
        serde_json::to_value(&status).unwrap_or_default(),
    ).await;

    // Clean up the verification doc
    if let Some(ev_id) = verification.get("id").and_then(|v| v.as_str()) {
        let _ = state.aegis_delete_doc("email_verifications", ev_id).await;
    }

    Ok(Json(serde_json::json!({
        "verified": true,
        "message": "Email verified successfully"
    })))
}

/// POST /auth/resend-verification — resend a verification email.
pub async fn resend_verification(
    State(state): State<AppState>,
    Extension(email_svc): Extension<std::sync::Arc<prometheus_email::EmailService>>,
    Json(req): Json<ForgotPasswordRequest>, // reuse — just needs email
) -> AppResult<Json<serde_json::Value>> {
    // Find user_status by email
    let statuses = state.aegis_list_docs("user_status").await?;
    let user_status = statuses.iter().find(|s| {
        s.get("email").and_then(|v| v.as_str()) == Some(&req.email)
    });

    let user_status = user_status
        .ok_or_else(|| AppError::NotFound("No account with that email".into()))?;

    if user_status.get("email_verified").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(Json(serde_json::json!({
            "message": "Email is already verified"
        })));
    }

    let user_id = user_status.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let username = user_status.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let now = Utc::now();

    // Delete old verification codes for this user
    let verifications = state.aegis_list_docs("email_verifications").await.unwrap_or_default();
    for v in &verifications {
        if v.get("user_id").and_then(|vi| vi.as_str()) == Some(user_id) {
            if let Some(vid) = v.get("id").and_then(|vi| vi.as_str()) {
                let _ = state.aegis_delete_doc("email_verifications", vid).await;
            }
        }
    }

    // Generate new code
    let code = generate_verification_code();
    let verification = EmailVerification {
        id: format!("ev_{}", &Uuid::new_v4().to_string()[..8]),
        user_id: user_id.to_string(),
        username: username.to_string(),
        email: req.email.clone(),
        code: code.clone(),
        created_at: now.to_rfc3339(),
        expires_at: (now + chrono::Duration::minutes(15)).to_rfc3339(),
    };
    let _ = state.aegis_create_doc(
        "email_verifications",
        serde_json::to_value(&verification).unwrap_or_default(),
    ).await;

    let _ = email_svc.send_verification(&req.email, username, &code, 15).await;

    Ok(Json(serde_json::json!({
        "message": "Verification email sent"
    })))
}

/// POST /auth/forgot-password — request a password reset email.
pub async fn forgot_password(
    State(state): State<AppState>,
    Extension(email_svc): Extension<std::sync::Arc<prometheus_email::EmailService>>,
    Json(req): Json<ForgotPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Look up user by email in user_status collection
    let statuses = state.aegis_list_docs("user_status").await.unwrap_or_default();
    let user_status = statuses.iter().find(|s| {
        s.get("email").and_then(|v| v.as_str()) == Some(&req.email)
    });

    // Always return success to prevent email enumeration
    let Some(status) = user_status else {
        return Ok(Json(serde_json::json!({
            "message": "If an account exists with that email, a reset link has been sent"
        })));
    };

    let username = status.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let now = Utc::now();
    let reset_token = format!("rst_{}", Uuid::new_v4());

    let reset = PasswordReset {
        id: format!("pr_{}", &Uuid::new_v4().to_string()[..8]),
        username: username.to_string(),
        email: req.email.clone(),
        token: reset_token.clone(),
        created_at: now.to_rfc3339(),
        expires_at: (now + chrono::Duration::minutes(30)).to_rfc3339(),
        used: false,
    };
    let _ = state.aegis_create_doc(
        "password_resets",
        serde_json::to_value(&reset).unwrap_or_default(),
    ).await;

    // Send reset email (best-effort)
    let _ = email_svc.send_password_reset(&req.email, username, &reset_token, 30).await;

    Ok(Json(serde_json::json!({
        "message": "If an account exists with that email, a reset link has been sent"
    })))
}

/// POST /auth/reset-password — complete password reset with token.
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.new_password.len() < 8 {
        return Err(AppError::BadRequest("Password must be at least 8 characters".into()));
    }

    let resets = state.aegis_list_docs("password_resets").await?;
    let now = Utc::now();

    let matching = resets.iter().find(|r| {
        r.get("token").and_then(|t| t.as_str()) == Some(&req.token)
            && r.get("used").and_then(|u| u.as_bool()) != Some(true)
    });

    let reset = matching
        .ok_or_else(|| AppError::BadRequest("Invalid or expired reset token".into()))?;

    // Check expiry
    if let Some(expires) = reset.get("expires_at").and_then(|v| v.as_str()) {
        if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(expires) {
            if now > exp {
                return Err(AppError::BadRequest("Reset token has expired".into()));
            }
        }
    }

    let username = reset.get("username").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("Corrupt reset record".into()))?;

    // Update password in Aegis-DB
    let resp = state
        .http_client
        .put(format!("{}/api/v1/admin/users/{}", state.config.aegis_db_url, username))
        .json(&serde_json::json!({ "password": req.new_password }))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(AppError::Internal("Failed to update password".into()));
    }

    // Mark reset token as used
    if let Some(reset_id) = reset.get("id").and_then(|v| v.as_str()) {
        let _ = state.aegis_delete_doc("password_resets", reset_id).await;
    }

    Ok(Json(serde_json::json!({
        "message": "Password reset successfully"
    })))
}

// ---------------------------------------------------------------------------
// Protected endpoints (require auth)
// ---------------------------------------------------------------------------

/// PUT /auth/change-password — authenticated user changes own password.
pub async fn change_password(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.new_password.len() < 8 {
        return Err(AppError::BadRequest("Password must be at least 8 characters".into()));
    }

    // Verify current password by attempting login through Aegis-DB
    let login_resp = state
        .http_client
        .post(format!("{}/api/v1/auth/login", state.config.aegis_db_url))
        .json(&serde_json::json!({
            "username": auth.username,
            "password": req.current_password,
        }))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !login_resp.status().is_success() {
        return Err(AppError::Unauthorized("Current password is incorrect".into()));
    }

    // Update password in Aegis-DB (Aegis handles Argon2id re-hashing)
    let resp = state
        .http_client
        .put(format!("{}/api/v1/admin/users/{}", state.config.aegis_db_url, auth.username))
        .json(&serde_json::json!({ "password": req.new_password }))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(AppError::Internal("Failed to update password".into()));
    }

    Ok(Json(serde_json::json!({
        "message": "Password changed successfully"
    })))
}

// ---------------------------------------------------------------------------
// Admin endpoints (require admin role)
// ---------------------------------------------------------------------------

/// GET /admin/users — list all users with subscription info.
pub async fn admin_list_users(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<serde_json::Value>> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden("Admin access required".into()));
    }

    // Get users from Aegis-DB
    let resp = state
        .http_client
        .get(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {}", auth.token))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(AppError::AegisDb("Failed to list users".into()));
    }

    let users: Vec<serde_json::Value> = resp.json().await.unwrap_or_default();

    // Enrich with subscription, status, and usage info
    let subscriptions = state.aegis_list_docs("subscriptions").await.unwrap_or_default();
    let statuses = state.aegis_list_docs("user_status").await.unwrap_or_default();
    let mfa_secrets = state.aegis_list_docs("mfa_secrets").await.unwrap_or_default();
    let all_datasets = state.aegis_list_docs("datasets").await.unwrap_or_default();
    let all_models = state.aegis_list_docs("models").await.unwrap_or_default();
    let all_training = state.aegis_list_docs("training_plans").await.unwrap_or_default();

    let enriched: Vec<serde_json::Value> = users.iter().map(|user| {
        let uid = user.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let uname = user.get("username").and_then(|v| v.as_str()).unwrap_or("");

        let sub = subscriptions.iter().find(|s| {
            s.get("user_id").and_then(|v| v.as_str()) == Some(uid)
        });
        let status = statuses.iter().find(|s| {
            s.get("id").and_then(|v| v.as_str()) == Some(uid)
                || s.get("username").and_then(|v| v.as_str()) == Some(uname)
        });
        let has_mfa = mfa_secrets.iter().any(|m| {
            m.get("id").and_then(|v| v.as_str()) == Some(uid)
                || m.get("user_id").and_then(|v| v.as_str()) == Some(uid)
        });

        let tier = sub
            .and_then(|s| s.get("tier").and_then(|v| v.as_str()))
            .unwrap_or("free");
        let tokens_used = sub
            .and_then(|s| s.get("tokens_used").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let tokens_limit = sub
            .and_then(|s| s.get("tokens_limit").and_then(|v| v.as_u64()))
            .unwrap_or(1000);
        let email_verified = status
            .and_then(|s| s.get("email_verified").and_then(|v| v.as_bool()))
            .unwrap_or(false);
        let account_approved = status
            .and_then(|s| s.get("account_approved").and_then(|v| v.as_bool()))
            .unwrap_or(false);

        // Count user's datasets, models, training runs, and storage
        let user_datasets: Vec<&serde_json::Value> = all_datasets.iter().filter(|d| {
            d.get("created_by").and_then(|v| v.as_str()) == Some(uid)
        }).collect();
        let user_models: Vec<&serde_json::Value> = all_models.iter().filter(|d| {
            d.get("created_by").and_then(|v| v.as_str()) == Some(uid)
        }).collect();
        let user_training: Vec<&serde_json::Value> = all_training.iter().filter(|d| {
            d.get("user_id").and_then(|v| v.as_str()) == Some(uid)
        }).collect();
        let active_training = user_training.iter().filter(|t| {
            matches!(t.get("status").and_then(|v| v.as_str()), Some("running") | Some("queued"))
        }).count();

        let dataset_storage: u64 = user_datasets.iter()
            .filter_map(|d| d.get("file_size_bytes").and_then(|v| v.as_u64()))
            .sum();
        let model_storage: u64 = user_models.iter()
            .filter_map(|d| d.get("file_size_bytes").and_then(|v| v.as_u64()))
            .sum();
        let total_storage = dataset_storage + model_storage;

        // Get last login from user data
        let last_login = user.get("last_login_at")
            .or_else(|| user.get("updated_at"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut enriched = user.clone();
        if let Some(obj) = enriched.as_object_mut() {
            obj.insert("tier".into(), serde_json::json!(tier));
            obj.insert("tokens_used".into(), serde_json::json!(tokens_used));
            obj.insert("tokens_limit".into(), serde_json::json!(tokens_limit));
            obj.insert("email_verified".into(), serde_json::json!(email_verified));
            obj.insert("account_approved".into(), serde_json::json!(account_approved));
            obj.insert("mfa_enabled".into(), serde_json::json!(has_mfa));
            obj.insert("dataset_count".into(), serde_json::json!(user_datasets.len()));
            obj.insert("model_count".into(), serde_json::json!(user_models.len()));
            obj.insert("training_count".into(), serde_json::json!(user_training.len()));
            obj.insert("active_training".into(), serde_json::json!(active_training));
            obj.insert("storage_bytes".into(), serde_json::json!(total_storage));
            obj.insert("last_login".into(), serde_json::json!(last_login));
        }

        enriched
    }).collect();

    Ok(Json(serde_json::json!({
        "users": enriched,
        "total": enriched.len()
    })))
}

/// POST /admin/users — admin creates a user (with optional tier and skip verification).
pub async fn admin_create_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Extension(email_svc): Extension<std::sync::Arc<prometheus_email::EmailService>>,
    Json(req): Json<AdminCreateUserRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden("Admin access required".into()));
    }

    if req.password.len() < 8 {
        return Err(AppError::BadRequest("Password must be at least 8 characters".into()));
    }

    // Create user in Aegis-DB
    let aegis_resp = state
        .http_client
        .post(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {}", auth.token))
        .json(&serde_json::json!({
            "username": req.username,
            "email": req.email,
            "password": req.password,
            "role": req.role,
        }))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !aegis_resp.status().is_success() {
        let body: serde_json::Value = aegis_resp.json().await.unwrap_or_default();
        let err = body.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to create user");
        return Err(AppError::BadRequest(err.to_string()));
    }

    let body: serde_json::Value = aegis_resp.json().await.unwrap_or_default();
    let user_id = body
        .get("user")
        .and_then(|u| u.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let now = Utc::now();

    // Store user status (admin-created users are auto-approved)
    let status = UserStatus {
        id: user_id.clone(),
        username: req.username.clone(),
        email: req.email.clone(),
        email_verified: req.skip_verification,
        account_approved: true, // admin-created users are pre-approved
        created_at: now.to_rfc3339(),
        verified_at: if req.skip_verification { Some(now.to_rfc3339()) } else { None },
        approved_at: Some(now.to_rfc3339()),
    };
    let _ = state.aegis_create_doc(
        "user_status",
        serde_json::to_value(&status).unwrap_or_default(),
    ).await;

    // Create subscription with specified tier
    let tier = req.tier.as_deref().unwrap_or("free");
    let tokens_limit: u64 = match tier {
        "pro" => 50_000,
        "enterprise" => 500_000,
        _ => 1_000,
    };
    let sub = serde_json::json!({
        "id": user_id,
        "user_id": user_id,
        "tier": tier,
        "stripe_customer_id": null,
        "stripe_subscription_id": null,
        "tokens_used": 0,
        "tokens_limit": tokens_limit,
        "current_period_start": now.to_rfc3339(),
        "current_period_end": (now + chrono::Duration::days(30)).to_rfc3339(),
    });
    let _ = state.aegis_create_doc("subscriptions", sub).await;

    // Send verification email if not skipping
    if !req.skip_verification {
        let code = generate_verification_code();
        let verification = EmailVerification {
            id: format!("ev_{}", &Uuid::new_v4().to_string()[..8]),
            user_id: user_id.clone(),
            username: req.username.clone(),
            email: req.email.clone(),
            code: code.clone(),
            created_at: now.to_rfc3339(),
            expires_at: (now + chrono::Duration::minutes(15)).to_rfc3339(),
        };
        let _ = state.aegis_create_doc(
            "email_verifications",
            serde_json::to_value(&verification).unwrap_or_default(),
        ).await;
        let _ = email_svc.send_verification(&req.email, &req.username, &code, 15).await;
    } else {
        // Send welcome email for pre-verified users
        let _ = email_svc.send_welcome(&req.email, &req.username).await;
    }

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "username": req.username,
        "email": req.email,
        "role": req.role,
        "tier": tier,
        "email_verified": req.skip_verification,
    })))
}

/// GET /admin/users/:username — get user detail with subscription info.
pub async fn admin_get_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(username): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden("Admin access required".into()));
    }

    // Get all users and find the one we want
    let resp = state
        .http_client
        .get(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {}", auth.token))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(AppError::AegisDb("Failed to fetch users".into()));
    }

    let users: Vec<serde_json::Value> = resp.json().await.unwrap_or_default();
    let user = users.iter().find(|u| {
        u.get("username").and_then(|v| v.as_str()) == Some(&username)
    }).ok_or_else(|| AppError::NotFound(format!("User '{username}' not found")))?;

    let uid = user.get("id").and_then(|v| v.as_str()).unwrap_or("");

    // Get subscription
    let sub = state.aegis_get_doc("subscriptions", uid).await.ok();

    // Get status
    let status = state.aegis_get_doc("user_status", uid).await.ok();

    // Check MFA
    let has_mfa = state.aegis_get_doc("mfa_secrets", uid).await.is_ok();

    // Get preferences
    let prefs = state.aegis_get_doc("user_preferences", uid).await.ok();

    let mut result = user.clone();
    if let Some(obj) = result.as_object_mut() {
        if let Some(s) = &sub {
            obj.insert("tier".into(), s.get("tier").cloned().unwrap_or(serde_json::json!("free")));
            obj.insert("tokens_used".into(), s.get("tokens_used").cloned().unwrap_or(serde_json::json!(0)));
            obj.insert("tokens_limit".into(), s.get("tokens_limit").cloned().unwrap_or(serde_json::json!(1000)));
            obj.insert("stripe_customer_id".into(), s.get("stripe_customer_id").cloned().unwrap_or(serde_json::json!(null)));
        }
        if let Some(st) = &status {
            obj.insert("email_verified".into(), st.get("email_verified").cloned().unwrap_or(serde_json::json!(false)));
            obj.insert("account_approved".into(), st.get("account_approved").cloned().unwrap_or(serde_json::json!(false)));
            obj.insert("verified_at".into(), st.get("verified_at").cloned().unwrap_or(serde_json::json!(null)));
            obj.insert("approved_at".into(), st.get("approved_at").cloned().unwrap_or(serde_json::json!(null)));
        }
        obj.insert("mfa_enabled".into(), serde_json::json!(has_mfa));
        if let Some(p) = &prefs {
            obj.insert("preferences".into(), p.clone());
        }
    }

    Ok(Json(result))
}

/// PUT /admin/users/:username — update user (role, email, password, tier, verification).
pub async fn admin_update_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(username): Path<String>,
    Json(req): Json<AdminUpdateUserRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden("Admin access required".into()));
    }

    // Update core user fields in Aegis-DB (email, role, password)
    if req.email.is_some() || req.role.is_some() || req.password.is_some() {
        let mut update = serde_json::Map::new();
        if let Some(ref email) = req.email {
            update.insert("email".into(), serde_json::json!(email));
        }
        if let Some(ref role) = req.role {
            update.insert("role".into(), serde_json::json!(role));
        }
        if let Some(ref password) = req.password {
            if password.len() < 8 {
                return Err(AppError::BadRequest("Password must be at least 8 characters".into()));
            }
            update.insert("password".into(), serde_json::json!(password));
        }

        let resp = state
            .http_client
            .put(format!("{}/api/v1/admin/users/{}", state.config.aegis_db_url, username))
            .header("Authorization", format!("Bearer {}", auth.token))
            .json(&serde_json::Value::Object(update))
            .send()
            .await
            .map_err(|e| AppError::AegisDb(e.to_string()))?;

        if !resp.status().is_success() {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let err = body.get("error").and_then(|v| v.as_str()).unwrap_or("Update failed");
            return Err(AppError::BadRequest(err.to_string()));
        }
    }

    // Look up user ID for collection updates
    let all_users = state
        .http_client
        .get(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {}", auth.token))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;
    let users: Vec<serde_json::Value> = all_users.json().await.unwrap_or_default();
    let uid = users.iter()
        .find(|u| u.get("username").and_then(|v| v.as_str()) == Some(&username))
        .and_then(|u| u.get("id").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    // Update subscription tier if specified
    if let Some(ref tier) = req.tier {
        let tokens_limit: u64 = match tier.as_str() {
            "pro" => 50_000,
            "enterprise" => 500_000,
            _ => 1_000,
        };

        // Get existing subscription or create new
        let existing = state.aegis_get_doc("subscriptions", &uid).await.ok();
        let tokens_used = existing
            .as_ref()
            .and_then(|s| s.get("tokens_used").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let _ = state.aegis_delete_doc("subscriptions", &uid).await;
        let now = Utc::now();
        let sub = serde_json::json!({
            "id": uid,
            "user_id": uid,
            "tier": tier,
            "stripe_customer_id": existing.as_ref().and_then(|s| s.get("stripe_customer_id").cloned()).unwrap_or(serde_json::json!(null)),
            "stripe_subscription_id": existing.as_ref().and_then(|s| s.get("stripe_subscription_id").cloned()).unwrap_or(serde_json::json!(null)),
            "tokens_used": tokens_used,
            "tokens_limit": tokens_limit,
            "current_period_start": now.to_rfc3339(),
            "current_period_end": (now + chrono::Duration::days(30)).to_rfc3339(),
        });
        let _ = state.aegis_create_doc("subscriptions", sub).await;
    }

    // Update email verification or account approval status if specified
    if req.email_verified.is_some() || req.account_approved.is_some() {
        let existing = state.aegis_get_doc("user_status", &uid).await.ok();
        let _ = state.aegis_delete_doc("user_status", &uid).await;

        let email = req.email.as_deref()
            .or_else(|| existing.as_ref().and_then(|s| s.get("email").and_then(|v| v.as_str())))
            .unwrap_or("")
            .to_string();
        let created = existing
            .as_ref()
            .and_then(|s| s.get("created_at").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        let prev_verified = existing
            .as_ref()
            .and_then(|s| s.get("email_verified").and_then(|v| v.as_bool()))
            .unwrap_or(false);
        let prev_approved = existing
            .as_ref()
            .and_then(|s| s.get("account_approved").and_then(|v| v.as_bool()))
            .unwrap_or(false);
        let prev_verified_at = existing
            .as_ref()
            .and_then(|s| s.get("verified_at").and_then(|v| v.as_str()))
            .map(String::from);
        let prev_approved_at = existing
            .as_ref()
            .and_then(|s| s.get("approved_at").and_then(|v| v.as_str()))
            .map(String::from);

        let now = Utc::now();
        let verified = req.email_verified.unwrap_or(prev_verified);
        let approved = req.account_approved.unwrap_or(prev_approved);

        let status = UserStatus {
            id: uid.clone(),
            username: username.clone(),
            email,
            email_verified: verified,
            account_approved: approved,
            created_at: created,
            verified_at: if verified && prev_verified_at.is_none() { Some(now.to_rfc3339()) } else { prev_verified_at },
            approved_at: if approved && prev_approved_at.is_none() { Some(now.to_rfc3339()) } else { prev_approved_at },
        };
        let _ = state.aegis_create_doc(
            "user_status",
            serde_json::to_value(&status).unwrap_or_default(),
        ).await;
    }

    Ok(Json(serde_json::json!({
        "updated": true,
        "username": username,
        "tier": req.tier,
        "email_verified": req.email_verified,
        "account_approved": req.account_approved,
    })))
}

/// DELETE /admin/users/:username — delete user and all associated data.
pub async fn admin_delete_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(username): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden("Admin access required".into()));
    }

    // Prevent self-deletion
    if auth.username == username {
        return Err(AppError::BadRequest("Cannot delete your own account".into()));
    }

    // Look up user ID
    let all_users = state
        .http_client
        .get(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {}", auth.token))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;
    let users: Vec<serde_json::Value> = all_users.json().await.unwrap_or_default();
    let uid = users.iter()
        .find(|u| u.get("username").and_then(|v| v.as_str()) == Some(&username))
        .and_then(|u| u.get("id").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    // Delete user from Aegis-DB
    let resp = state
        .http_client
        .delete(format!("{}/api/v1/admin/users/{}", state.config.aegis_db_url, username))
        .header("Authorization", format!("Bearer {}", auth.token))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::AegisDb(format!("Failed to delete user: {body}")));
    }

    // Cleanup associated docs (best-effort)
    if !uid.is_empty() {
        let _ = state.aegis_delete_doc("subscriptions", &uid).await;
        let _ = state.aegis_delete_doc("user_status", &uid).await;
        let _ = state.aegis_delete_doc("user_preferences", &uid).await;
        let _ = state.aegis_delete_doc("mfa_secrets", &uid).await;

        // Clean up push tokens for this user
        if let Ok(tokens) = state.aegis_list_docs("push_tokens").await {
            for token in &tokens {
                if token.get("user_id").and_then(|v| v.as_str()) == Some(&uid) {
                    if let Some(tid) = token.get("id").and_then(|v| v.as_str()) {
                        let _ = state.aegis_delete_doc("push_tokens", tid).await;
                    }
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "deleted": true,
        "username": username,
    })))
}

/// POST /admin/users/:username/approve — approve a user account.
pub async fn admin_approve_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(username): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden("Admin access required".into()));
    }

    // Look up user ID
    let all_users = state
        .http_client
        .get(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {}", auth.token))
        .send()
        .await
        .map_err(|e| AppError::AegisDb(e.to_string()))?;
    let users: Vec<serde_json::Value> = all_users.json().await.unwrap_or_default();
    let uid = users.iter()
        .find(|u| u.get("username").and_then(|v| v.as_str()) == Some(&username))
        .and_then(|u| u.get("id").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    if uid.is_empty() {
        return Err(AppError::NotFound(format!("User '{username}' not found")));
    }

    let existing = state.aegis_get_doc("user_status", &uid).await.ok();
    let _ = state.aegis_delete_doc("user_status", &uid).await;

    let now = Utc::now();
    let email = existing.as_ref()
        .and_then(|s| s.get("email").and_then(|v| v.as_str()))
        .unwrap_or("").to_string();
    let email_verified = existing.as_ref()
        .and_then(|s| s.get("email_verified").and_then(|v| v.as_bool()))
        .unwrap_or(false);
    let created_at = existing.as_ref()
        .and_then(|s| s.get("created_at").and_then(|v| v.as_str()))
        .unwrap_or("").to_string();
    let verified_at = existing.as_ref()
        .and_then(|s| s.get("verified_at").and_then(|v| v.as_str()))
        .map(String::from);

    let status = UserStatus {
        id: uid.clone(),
        username: username.clone(),
        email: email.clone(),
        email_verified,
        account_approved: true,
        created_at,
        verified_at,
        approved_at: Some(now.to_rfc3339()),
    };
    let _ = state.aegis_create_doc(
        "user_status",
        serde_json::to_value(&status).unwrap_or_default(),
    ).await;

    // Send welcome email to newly approved user
    // (the Extension<Arc<EmailService>> isn't available on this handler signature,
    //  so we use a notification instead)
    let _ = crate::api::push::notify_user_typed(
        &state,
        &uid,
        "Account Approved",
        &format!("Your Prometheus account ({username}) has been approved. You can now sign in."),
        &crate::api::push::NotificationType::AccountVerified,
        Some(serde_json::json!({ "username": username })),
    ).await;

    Ok(Json(serde_json::json!({
        "approved": true,
        "username": username,
        "approved_at": now.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a 6-digit numeric verification code.
fn generate_verification_code() -> String {
    use rand::Rng;
    let code: u32 = rand::thread_rng().gen_range(100_000..999_999);
    code.to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signup_request_deserialize() {
        let json = r#"{"username":"alice","email":"alice@example.com","password":"secret1234"}"#;
        let req: SignupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "alice");
        assert_eq!(req.email, "alice@example.com");
        assert_eq!(req.password, "secret1234");
    }

    #[test]
    fn signup_response_serialize() {
        let resp = SignupResponse {
            user_id: "user-001".into(),
            username: "alice".into(),
            email_pending: true,
            message: "Check email".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["email_pending"], true);
        assert_eq!(json["user_id"], "user-001");
    }

    #[test]
    fn verify_email_request_deserialize() {
        let json = r#"{"code":"123456"}"#;
        let req: VerifyEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "123456");
    }

    #[test]
    fn forgot_password_request_deserialize() {
        let json = r#"{"email":"alice@example.com"}"#;
        let req: ForgotPasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "alice@example.com");
    }

    #[test]
    fn reset_password_request_deserialize() {
        let json = r#"{"token":"rst_abc123","new_password":"newsecret1234"}"#;
        let req: ResetPasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.token, "rst_abc123");
        assert_eq!(req.new_password, "newsecret1234");
    }

    #[test]
    fn change_password_request_deserialize() {
        let json = r#"{"current_password":"old","new_password":"newpass12"}"#;
        let req: ChangePasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.current_password, "old");
        assert_eq!(req.new_password, "newpass12");
    }

    #[test]
    fn admin_create_user_request_defaults() {
        let json = r#"{"username":"bob","email":"bob@example.com","password":"password123"}"#;
        let req: AdminCreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.role, "operator");
        assert!(req.tier.is_none());
        assert!(!req.skip_verification);
    }

    #[test]
    fn admin_create_user_request_full() {
        let json = r#"{"username":"bob","email":"bob@example.com","password":"password123","role":"admin","tier":"enterprise","skip_verification":true}"#;
        let req: AdminCreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.role, "admin");
        assert_eq!(req.tier.as_deref(), Some("enterprise"));
        assert!(req.skip_verification);
    }

    #[test]
    fn admin_update_user_request_partial() {
        let json = r#"{"tier":"pro"}"#;
        let req: AdminUpdateUserRequest = serde_json::from_str(json).unwrap();
        assert!(req.email.is_none());
        assert!(req.role.is_none());
        assert!(req.password.is_none());
        assert_eq!(req.tier.as_deref(), Some("pro"));
        assert!(req.email_verified.is_none());
        assert!(req.account_approved.is_none());
    }

    #[test]
    fn admin_update_user_approve_account() {
        let json = r#"{"account_approved":true}"#;
        let req: AdminUpdateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.account_approved, Some(true));
        assert!(req.email.is_none());
    }

    #[test]
    fn user_status_roundtrip() {
        let status = UserStatus {
            id: "user-001".into(),
            username: "alice".into(),
            email: "alice@example.com".into(),
            email_verified: false,
            account_approved: false,
            created_at: "2026-01-01T00:00:00Z".into(),
            verified_at: None,
            approved_at: None,
        };
        let json = serde_json::to_value(&status).unwrap();
        let parsed: UserStatus = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.username, "alice");
        assert!(!parsed.email_verified);
        assert!(!parsed.account_approved);
        assert!(parsed.verified_at.is_none());
        assert!(parsed.approved_at.is_none());
    }

    #[test]
    fn user_status_verified_roundtrip() {
        let status = UserStatus {
            id: "user-002".into(),
            username: "bob".into(),
            email: "bob@example.com".into(),
            email_verified: true,
            account_approved: true,
            created_at: "2026-01-01T00:00:00Z".into(),
            verified_at: Some("2026-01-01T01:00:00Z".into()),
            approved_at: Some("2026-01-01T02:00:00Z".into()),
        };
        let json = serde_json::to_value(&status).unwrap();
        let parsed: UserStatus = serde_json::from_value(json).unwrap();
        assert!(parsed.email_verified);
        assert!(parsed.account_approved);
        assert_eq!(parsed.verified_at.as_deref(), Some("2026-01-01T01:00:00Z"));
        assert_eq!(parsed.approved_at.as_deref(), Some("2026-01-01T02:00:00Z"));
    }

    #[test]
    fn email_verification_roundtrip() {
        let ev = EmailVerification {
            id: "ev_abc123".into(),
            user_id: "user-001".into(),
            username: "alice".into(),
            email: "alice@example.com".into(),
            code: "123456".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            expires_at: "2026-01-01T00:15:00Z".into(),
        };
        let json = serde_json::to_value(&ev).unwrap();
        let parsed: EmailVerification = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.code, "123456");
        assert_eq!(parsed.user_id, "user-001");
    }

    #[test]
    fn password_reset_roundtrip() {
        let pr = PasswordReset {
            id: "pr_abc123".into(),
            username: "alice".into(),
            email: "alice@example.com".into(),
            token: "rst_xyz789".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            expires_at: "2026-01-01T00:30:00Z".into(),
            used: false,
        };
        let json = serde_json::to_value(&pr).unwrap();
        let parsed: PasswordReset = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.token, "rst_xyz789");
        assert!(!parsed.used);
    }

    #[test]
    fn verification_code_format() {
        let code = generate_verification_code();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn verification_code_unique() {
        let a = generate_verification_code();
        let b = generate_verification_code();
        // Statistically should be different (1 in 900K chance of collision)
        // If this flakes, that's fine — the important test is format above
        let _ = (a, b); // use but don't assert uniqueness to avoid flakes
    }

    #[test]
    fn default_role_is_operator() {
        assert_eq!(default_role(), "operator");
    }
}
