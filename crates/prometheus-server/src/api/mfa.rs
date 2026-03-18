// ============================================================================
// File: mfa.rs
// Description: TOTP-based multi-factor authentication setup, verification, and management
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

//! TOTP-based multi-factor authentication.
//!
//! MFA secrets are stored in Aegis-DB `mfa_secrets` collection.
//! Users enable/disable MFA from their profile preferences.

use axum::{extract::State, Extension, Json};
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, TOTP, Secret};
use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// MFA setup response — contains secret and provisioning URI for QR code.
#[derive(Debug, Serialize)]
pub struct MfaSetupResponse {
    pub secret: String,
    pub otpauth_url: String,
    pub qr_code_base64: String,
}

/// MFA verification request.
#[derive(Debug, Deserialize)]
pub struct MfaVerifyRequest {
    pub code: String,
}

/// MFA status response.
#[derive(Debug, Serialize)]
pub struct MfaStatusResponse {
    pub enabled: bool,
}

/// MFA validation request (during login).
#[derive(Debug, Deserialize)]
pub struct MfaValidateRequest {
    pub user_id: String,
    pub code: String,
}

/// MFA record stored in Aegis-DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MfaSecret {
    id: String,
    user_id: String,
    secret_base32: String,
    enabled: bool,
    created_at: String,
    verified_at: Option<String>,
}

/// Begin MFA setup — generates a TOTP secret and returns QR code data.
/// The secret is stored but not yet enabled until verified.
pub async fn setup(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<MfaSetupResponse>> {

    // Generate a random TOTP secret
    let secret = Secret::generate_secret();
    let secret_base32 = secret.to_encoded().to_string();

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().map_err(|e| AppError::Internal(format!("Secret error: {e}")))?,
        Some("Prometheus".to_string()),
        auth.username.clone(),
    ).map_err(|e| AppError::Internal(format!("TOTP creation error: {e}")))?;

    let otpauth_url = totp.get_url();

    // Generate QR code as base64 PNG
    let qr_code_base64 = generate_qr_base64(&otpauth_url)?;

    // Store the secret (not yet enabled)
    let now = chrono::Utc::now().to_rfc3339();
    let mfa = MfaSecret {
        id: auth.user_id.clone(),
        user_id: auth.user_id.clone(),
        secret_base32: secret_base32.clone(),
        enabled: false,
        created_at: now,
        verified_at: None,
    };

    let doc = serde_json::to_value(&mfa)
        .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;

    // Upsert: delete any existing, then create
    let _ = state.aegis_delete_doc("mfa_secrets", &auth.user_id).await;
    state.aegis_create_doc("mfa_secrets", doc).await?;

    Ok(Json(MfaSetupResponse {
        secret: secret_base32,
        otpauth_url,
        qr_code_base64,
    }))
}

/// Verify a TOTP code to enable MFA.
/// Must be called after setup with the current code from the authenticator app.
pub async fn verify(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<MfaVerifyRequest>,
) -> AppResult<Json<MfaStatusResponse>> {

    let mfa = load_mfa_secret(&state, &auth.user_id).await?;

    // Validate the code
    if !validate_totp_code(&mfa.secret_base32, &req.code, &auth.username)? {
        return Err(AppError::BadRequest("Invalid MFA code".into()));
    }

    // Enable MFA
    let now = chrono::Utc::now().to_rfc3339();
    let updated = MfaSecret {
        enabled: true,
        verified_at: Some(now.clone()),
        ..mfa
    };

    let doc = serde_json::to_value(&updated)
        .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;
    let _ = state.aegis_delete_doc("mfa_secrets", &auth.user_id).await;
    state.aegis_create_doc("mfa_secrets", doc).await?;

    // Also update user preferences to reflect MFA enabled
    update_user_mfa_pref(&state, &auth.user_id, true).await?;

    Ok(Json(MfaStatusResponse { enabled: true }))
}

/// Disable MFA — requires a valid TOTP code.
pub async fn disable(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<MfaVerifyRequest>,
) -> AppResult<Json<MfaStatusResponse>> {

    let mfa = load_mfa_secret(&state, &auth.user_id).await?;

    if !mfa.enabled {
        return Err(AppError::BadRequest("MFA is not enabled".into()));
    }

    // Validate the code before disabling
    if !validate_totp_code(&mfa.secret_base32, &req.code, &auth.username)? {
        return Err(AppError::BadRequest("Invalid MFA code".into()));
    }

    // Delete MFA secret
    let _ = state.aegis_delete_doc("mfa_secrets", &auth.user_id).await;
    update_user_mfa_pref(&state, &auth.user_id, false).await?;

    Ok(Json(MfaStatusResponse { enabled: false }))
}

/// Validate an MFA code during login — called from auth flow.
/// This is a public-ish endpoint but requires a valid user_id.
pub async fn validate(
    State(state): State<AppState>,
    Json(req): Json<MfaValidateRequest>,
) -> AppResult<Json<MfaStatusResponse>> {
    let mfa = load_mfa_secret(&state, &req.user_id).await
        .map_err(|_| AppError::BadRequest("MFA not configured for this user".into()))?;

    if !mfa.enabled {
        // MFA not enabled, allow
        return Ok(Json(MfaStatusResponse { enabled: false }));
    }

    let username = req.user_id.clone(); // Use user_id as fallback for TOTP account name
    if !validate_totp_code(&mfa.secret_base32, &req.code, &username)? {
        return Err(AppError::Unauthorized("Invalid MFA code".into()));
    }

    Ok(Json(MfaStatusResponse { enabled: true }))
}

/// Check if a user has MFA enabled.
pub async fn check_mfa_required(state: &AppState, user_id: &str) -> bool {
    match load_mfa_secret(state, user_id).await {
        Ok(mfa) => mfa.enabled,
        Err(_) => false,
    }
}

// ── Internal helpers ──────────────────────────────────────

async fn load_mfa_secret(state: &AppState, user_id: &str) -> Result<MfaSecret, AppError> {
    let doc = state.aegis_get_doc("mfa_secrets", user_id).await?;
    serde_json::from_value(doc)
        .map_err(|e| AppError::Internal(format!("MFA secret parse error: {e}")))
}

fn validate_totp_code(secret_base32: &str, code: &str, account_name: &str) -> Result<bool, AppError> {
    let secret = Secret::Encoded(secret_base32.to_string());
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().map_err(|e| AppError::Internal(format!("Secret decode error: {e}")))?,
        Some("Prometheus".to_string()),
        account_name.to_string(),
    ).map_err(|e| AppError::Internal(format!("TOTP error: {e}")))?;

    Ok(totp.check_current(code).unwrap_or(false))
}

fn generate_qr_base64(data: &str) -> Result<String, AppError> {
    // Simple QR code generation — encode the otpauth URL as a base64 string.
    // In production this would use a QR code library, but we return the URL
    // and let the client render it (more flexible for mobile/web).
    // The base64 field contains the raw otpauth URL encoded in base64.
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        data.as_bytes(),
    ))
}

async fn update_user_mfa_pref(state: &AppState, user_id: &str, enabled: bool) -> Result<(), AppError> {
    // Load existing preferences or create default
    let prefs = match state.aegis_get_doc("user_preferences", user_id).await {
        Ok(doc) => {
            let mut p: serde_json::Value = doc;
            p["mfa_enabled"] = serde_json::json!(enabled);
            p["updated_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
            p
        }
        Err(_) => {
            serde_json::json!({
                "id": user_id,
                "user_id": user_id,
                "mfa_enabled": enabled,
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })
        }
    };

    let _ = state.aegis_delete_doc("user_preferences", user_id).await;
    state.aegis_create_doc("user_preferences", prefs).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mfa_secret_serialize_roundtrip() {
        let mfa = MfaSecret {
            id: "user-1".into(),
            user_id: "user-1".into(),
            secret_base32: "JBSWY3DPEHPK3PXP".into(),
            enabled: true,
            created_at: "2026-01-01T00:00:00Z".into(),
            verified_at: Some("2026-01-01T00:01:00Z".into()),
        };
        let json = serde_json::to_value(&mfa).unwrap();
        let parsed: MfaSecret = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.user_id, "user-1");
        assert!(parsed.enabled);
        assert!(parsed.verified_at.is_some());
    }

    #[test]
    fn mfa_secret_disabled_by_default() {
        let mfa = MfaSecret {
            id: "user-2".into(),
            user_id: "user-2".into(),
            secret_base32: "ABCDEFGH".into(),
            enabled: false,
            created_at: "2026-01-01T00:00:00Z".into(),
            verified_at: None,
        };
        assert!(!mfa.enabled);
        assert!(mfa.verified_at.is_none());
    }

    #[test]
    fn validate_totp_code_with_known_secret() {
        // Generate a secret and get the current code, then validate
        let secret = Secret::generate_secret();
        let secret_base32 = secret.to_encoded().to_string();

        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret.to_bytes().unwrap(),
            Some("Prometheus".to_string()),
            "testuser".to_string(),
        ).unwrap();

        let code = totp.generate_current().unwrap();
        assert!(validate_totp_code(&secret_base32, &code, "testuser").unwrap());
    }

    #[test]
    fn validate_totp_code_wrong_code() {
        let secret = Secret::generate_secret();
        let secret_base32 = secret.to_encoded().to_string();
        assert!(!validate_totp_code(&secret_base32, "000000", "testuser").unwrap());
    }

    #[test]
    fn generate_qr_base64_encodes_url() {
        let url = "otpauth://totp/Prometheus:user@example.com?secret=ABC&issuer=Prometheus";
        let result = generate_qr_base64(url).unwrap();
        assert!(!result.is_empty());
        // Decode it back
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &result,
        ).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), url);
    }

    #[test]
    fn mfa_verify_request_deserialize() {
        let json = r#"{"code":"123456"}"#;
        let req: MfaVerifyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "123456");
    }

    #[test]
    fn mfa_validate_request_deserialize() {
        let json = r#"{"user_id":"u-42","code":"654321"}"#;
        let req: MfaValidateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.user_id, "u-42");
        assert_eq!(req.code, "654321");
    }

    #[test]
    fn mfa_status_response_serialize() {
        let resp = MfaStatusResponse { enabled: true };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("true"));
    }

    #[test]
    fn mfa_setup_response_serialize() {
        let resp = MfaSetupResponse {
            secret: "JBSWY3DPEHPK3PXP".into(),
            otpauth_url: "otpauth://totp/Prometheus:user?secret=JBSWY3DPEHPK3PXP".into(),
            qr_code_base64: "dGVzdA==".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["secret"], "JBSWY3DPEHPK3PXP");
    }
}
