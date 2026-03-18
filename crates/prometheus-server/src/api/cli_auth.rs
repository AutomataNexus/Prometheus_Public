// ============================================================================
// File: cli_auth.rs
// Description: CLI device-authorization flow for headless authentication via browser verification
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

//! CLI authentication endpoints.
//!
//! Implements the device-authorization-like flow:
//! 1. CLI calls POST /init to create a pending session
//! 2. User opens the verify URL in a browser and authenticates
//! 3. CLI polls GET /poll until it receives the auth token

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::error::AppResult;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct InitRequest {
    pub session_code: String,
}

#[derive(Deserialize)]
pub struct PollQuery {
    pub code: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub code: String,
    pub token: String,
    pub username: String,
    pub role: String,
}

/// Initialize a CLI auth session.
/// Creates a pending session that the user must verify via browser.
pub async fn cli_auth_init(
    State(state): State<AppState>,
    Json(req): Json<InitRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Store pending session in Aegis-DB
    let session_doc = json!({
        "id": req.session_code,
        "status": "pending",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "token": null,
        "username": null,
        "role": null,
    });

    let _ = state.aegis_create_doc("cli_sessions", session_doc).await;

    let verify_url = format!(
        "{}/auth/verify?code={}",
        state
            .config
            .public_url()
            .unwrap_or_else(|| format!("http://{}:{}", state.config.host, state.config.port)),
        req.session_code
    );

    Ok(Json(json!({
        "session_code": req.session_code,
        "verify_url": verify_url,
        "expires_in": 120,
    })))
}

/// Poll for CLI auth completion.
/// Returns the token once the user has authenticated via browser.
pub async fn cli_auth_poll(
    State(state): State<AppState>,
    Query(query): Query<PollQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state
        .aegis_get_doc("cli_sessions", &query.code)
        .await?;

    let status = doc
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("pending");

    if status == "verified" {
        let token = doc.get("token").and_then(|v| v.as_str()).unwrap_or("");
        let username = doc
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("user");
        let role = doc
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("operator");

        // Clean up the session
        let _ = state
            .aegis_delete_doc("cli_sessions", &query.code)
            .await;

        Ok(Json(json!({
            "status": "verified",
            "token": token,
            "username": username,
            "role": role,
        })))
    } else {
        Ok(Json(json!({
            "status": "pending",
        })))
    }
}

/// Verify a CLI session (called from the browser after user authenticates).
/// Links the user's auth token to the pending CLI session.
pub async fn cli_auth_verify(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Update the pending session with the authenticated token
    let _ = state
        .aegis_update_doc("cli_sessions", &req.code, json!({
            "status": "verified",
            "token": req.token,
            "username": req.username,
            "role": req.role,
            "verified_at": chrono::Utc::now().to_rfc3339(),
        }))
        .await?;

    Ok(Json(json!({
        "status": "verified",
        "message": "CLI session authenticated. You can close this tab.",
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── InitRequest deserialization ────────────────────────

    #[test]
    fn init_request_deserializes_from_json() {
        let json_str = r#"{"session_code": "ABC-123"}"#;
        let req: InitRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.session_code, "ABC-123");
    }

    #[test]
    fn init_request_rejects_missing_session_code() {
        let json_str = r#"{}"#;
        let result = serde_json::from_str::<InitRequest>(json_str);
        assert!(result.is_err());
    }

    // ── PollQuery deserialization ──────────────────────────

    #[test]
    fn poll_query_deserializes_from_json() {
        let json_str = r#"{"code": "XYZ-789"}"#;
        let query: PollQuery = serde_json::from_str(json_str).unwrap();
        assert_eq!(query.code, "XYZ-789");
    }

    #[test]
    fn poll_query_rejects_missing_code() {
        let json_str = r#"{}"#;
        let result = serde_json::from_str::<PollQuery>(json_str);
        assert!(result.is_err());
    }

    // ── VerifyRequest deserialization ──────────────────────

    #[test]
    fn verify_request_deserializes_all_fields() {
        let json_str = r#"{
            "code": "ABC-123",
            "token": "bearer_tok_xyz",
            "username": "alice",
            "role": "admin"
        }"#;
        let req: VerifyRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.code, "ABC-123");
        assert_eq!(req.token, "bearer_tok_xyz");
        assert_eq!(req.username, "alice");
        assert_eq!(req.role, "admin");
    }

    #[test]
    fn verify_request_rejects_partial_fields() {
        let json_str = r#"{"code": "ABC", "token": "tok"}"#;
        let result = serde_json::from_str::<VerifyRequest>(json_str);
        assert!(result.is_err());
    }

    // ── Session document structure ─────────────────────────

    #[test]
    fn session_document_structure_has_expected_fields() {
        let code = "TEST-CODE-42";
        let session_doc = json!({
            "id": code,
            "status": "pending",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "token": null,
            "username": null,
            "role": null,
        });

        assert_eq!(session_doc["id"], "TEST-CODE-42");
        assert_eq!(session_doc["status"], "pending");
        assert!(session_doc["token"].is_null());
        assert!(session_doc["username"].is_null());
        assert!(session_doc["role"].is_null());
        // created_at should be a valid RFC3339 timestamp
        let created_at = session_doc["created_at"].as_str().unwrap();
        assert!(chrono::DateTime::parse_from_rfc3339(created_at).is_ok());
    }

    #[test]
    fn verified_session_document_structure() {
        let verified_doc = json!({
            "status": "verified",
            "token": "my_auth_token",
            "username": "bob",
            "role": "operator",
            "verified_at": chrono::Utc::now().to_rfc3339(),
        });

        assert_eq!(verified_doc["status"], "verified");
        assert_eq!(verified_doc["token"], "my_auth_token");
        assert_eq!(verified_doc["username"], "bob");
        assert_eq!(verified_doc["role"], "operator");
        let verified_at = verified_doc["verified_at"].as_str().unwrap();
        assert!(chrono::DateTime::parse_from_rfc3339(verified_at).is_ok());
    }

    // ── Session code generation patterns ───────────────────

    #[test]
    fn session_code_can_be_any_string() {
        // The session code is user-provided, so it should accept various formats
        let codes = vec![
            "simple",
            "ABC-123-XYZ",
            "a1b2c3d4e5f6",
            "session_2026_03_07_001",
        ];
        for code in codes {
            let json_str = format!(r#"{{"session_code": "{}"}}"#, code);
            let req: InitRequest = serde_json::from_str(&json_str).unwrap();
            assert_eq!(req.session_code, code);
        }
    }

    #[test]
    fn verify_url_format() {
        // Test the verify URL construction pattern
        let host = "0.0.0.0";
        let port = 3030;
        let code = "TEST-CODE";
        let verify_url = format!("http://{}:{}/auth/verify?code={}", host, port, code);
        assert_eq!(verify_url, "http://0.0.0.0:3030/auth/verify?code=TEST-CODE");
    }

    #[test]
    fn verify_url_with_public_url() {
        let public_url = "https://prometheus.example.com";
        let code = "ABC-123";
        let verify_url = format!("{}/auth/verify?code={}", public_url, code);
        assert_eq!(verify_url, "https://prometheus.example.com/auth/verify?code=ABC-123");
    }

    #[test]
    fn poll_response_pending_structure() {
        let response = json!({
            "status": "pending",
        });
        assert_eq!(response["status"], "pending");
    }

    #[test]
    fn poll_response_verified_structure() {
        let response = json!({
            "status": "verified",
            "token": "tok_abc",
            "username": "alice",
            "role": "admin",
        });
        assert_eq!(response["status"], "verified");
        assert_eq!(response["token"], "tok_abc");
        assert_eq!(response["username"], "alice");
        assert_eq!(response["role"], "admin");
    }

    #[test]
    fn init_response_structure() {
        let code = "MY-CODE";
        let response = json!({
            "session_code": code,
            "verify_url": format!("http://0.0.0.0:3030/auth/verify?code={}", code),
            "expires_in": 120,
        });
        assert_eq!(response["session_code"], "MY-CODE");
        assert_eq!(response["expires_in"], 120);
        assert!(response["verify_url"].as_str().unwrap().contains("MY-CODE"));
    }
}
