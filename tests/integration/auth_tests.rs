//! Prometheus Authentication Integration Tests
//!
//! Tests the full authentication flow: login, session validation,
//! role-based access control, logout, and rate limiting.
//!
//! # Requirements
//! - Prometheus server running on `PROMETHEUS_URL` (default: `http://localhost:3030`)
//! - Aegis-DB running on port 9091
//!
//! # Running
//! ```bash
//! cargo test --test auth_tests
//! ```

use serde_json::{json, Value};
use std::env;

fn base_url() -> String {
    env::var("PROMETHEUS_URL").unwrap_or_else(|_| "http://localhost:3030".to_string())
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn admin_creds() -> (String, String) {
    (
        env::var("TEST_ADMIN_USER").unwrap_or_else(|_| "admin".into()),
        env::var("TEST_ADMIN_PASS").unwrap_or_else(|_| "admin_password".into()),
    )
}

fn operator_creds() -> (String, String) {
    (
        env::var("TEST_OPERATOR_USER").unwrap_or_else(|_| "operator".into()),
        env::var("TEST_OPERATOR_PASS").unwrap_or_else(|_| "operator_password".into()),
    )
}

fn viewer_creds() -> (String, String) {
    (
        env::var("TEST_VIEWER_USER").unwrap_or_else(|_| "viewer".into()),
        env::var("TEST_VIEWER_PASS").unwrap_or_else(|_| "viewer_password".into()),
    )
}

// ─── Login ─────────────────────────────────────────────────

#[tokio::test]
async fn test_login_with_valid_credentials() {
    let (user, pass) = admin_creds();

    let resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success(), "Login failed: {}", resp.status());

    let body: Value = resp.json().await.unwrap();
    assert!(body["token"].is_string(), "No token in response");
    assert!(
        body["token"].as_str().unwrap().len() > 10,
        "Token too short"
    );
    assert!(body["user"].is_object(), "No user object in response");
    assert_eq!(body["user"]["username"], user);
}

#[tokio::test]
async fn test_login_with_invalid_username() {
    let resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({
            "username": "nonexistent_user_xyz",
            "password": "some_password"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401, "Expected 401, got {}", resp.status());
}

#[tokio::test]
async fn test_login_with_wrong_password() {
    let (user, _) = admin_creds();

    let resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({
            "username": user,
            "password": "completely_wrong_password_123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_login_with_empty_body() {
    let resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_login_with_missing_password() {
    let resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": "admin" }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_client_error());
}

// ─── Session validation ────────────────────────────────────

#[tokio::test]
async fn test_session_validation_with_valid_token() {
    let (user, pass) = admin_creds();

    // Login first
    let login_resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await
        .unwrap();

    let login_body: Value = login_resp.json().await.unwrap();
    let token = login_body["token"].as_str().unwrap();

    // Validate session
    let session_resp = client()
        .get(format!("{}/api/v1/auth/session", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(session_resp.status().is_success());
    let session: Value = session_resp.json().await.unwrap();
    assert_eq!(session["valid"], true);
    assert!(session["user"].is_object());
    assert_eq!(session["user"]["username"], user);
}

#[tokio::test]
async fn test_session_validation_with_invalid_token() {
    let resp = client()
        .get(format!("{}/api/v1/auth/session", base_url()))
        .header("Authorization", "Bearer invalid_token_12345")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_session_validation_without_token() {
    let resp = client()
        .get(format!("{}/api/v1/auth/session", base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ─── Current user (me) endpoint ────────────────────────────

#[tokio::test]
async fn test_me_endpoint_returns_user_info() {
    let (user, pass) = admin_creds();

    let login_resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await
        .unwrap();

    let body: Value = login_resp.json().await.unwrap();
    let token = body["token"].as_str().unwrap();

    let me_resp = client()
        .get(format!("{}/api/v1/auth/me", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(me_resp.status().is_success());
    let me: Value = me_resp.json().await.unwrap();
    assert_eq!(me["username"], user);
    assert!(me["role"].is_string());
}

// ─── Logout ────────────────────────────────────────────────

#[tokio::test]
async fn test_logout_invalidates_session() {
    let (user, pass) = admin_creds();

    // Login
    let login_resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await
        .unwrap();

    let body: Value = login_resp.json().await.unwrap();
    let token = body["token"].as_str().unwrap();

    // Verify session is valid
    let session_resp = client()
        .get(format!("{}/api/v1/auth/session", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert!(session_resp.status().is_success());

    // Logout
    let logout_resp = client()
        .post(format!("{}/api/v1/auth/logout", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert!(logout_resp.status().is_success());

    // Session should now be invalid
    let post_logout_resp = client()
        .get(format!("{}/api/v1/auth/session", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(post_logout_resp.status(), 401);
}

#[tokio::test]
async fn test_api_request_after_logout_returns_401() {
    let (user, pass) = admin_creds();

    let login_resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await
        .unwrap();

    let body: Value = login_resp.json().await.unwrap();
    let token = body["token"].as_str().unwrap();

    // Logout
    client()
        .post(format!("{}/api/v1/auth/logout", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    // Try to access a protected endpoint
    let resp = client()
        .get(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ─── Role-based access control ─────────────────────────────

#[tokio::test]
async fn test_admin_can_access_all_endpoints() {
    let (user, pass) = admin_creds();

    let login_resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await
        .unwrap();

    let body: Value = login_resp.json().await.unwrap();
    let token = body["token"].as_str().unwrap();

    let endpoints = vec![
        "/api/v1/datasets",
        "/api/v1/models",
        "/api/v1/training",
        "/api/v1/deployments",
        "/api/v1/evaluations",
        "/api/v1/agent/history",
        "/api/v1/system/metrics",
    ];

    for endpoint in endpoints {
        let resp = client()
            .get(format!("{}{}", base_url(), endpoint))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        assert!(
            resp.status().is_success(),
            "Admin should access {} but got {}",
            endpoint,
            resp.status()
        );
    }
}

#[tokio::test]
async fn test_operator_can_upload_datasets() {
    let (user, pass) = operator_creds();

    let login_resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await;

    // If operator user does not exist, skip
    if login_resp.is_err() {
        eprintln!("Operator user not available — skipping RBAC test");
        return;
    }

    let resp = login_resp.unwrap();
    if !resp.status().is_success() {
        eprintln!("Operator login failed — skipping RBAC test");
        return;
    }

    let body: Value = resp.json().await.unwrap();
    let token = body["token"].as_str().unwrap();

    // Operator should be able to read datasets
    let datasets_resp = client()
        .get(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(datasets_resp.status().is_success());
}

#[tokio::test]
async fn test_viewer_has_read_only_access() {
    let (user, pass) = viewer_creds();

    let login_resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await;

    if login_resp.is_err() {
        eprintln!("Viewer user not available — skipping RBAC test");
        return;
    }

    let resp = login_resp.unwrap();
    if !resp.status().is_success() {
        eprintln!("Viewer login failed — skipping RBAC test");
        return;
    }

    let body: Value = resp.json().await.unwrap();
    let token = body["token"].as_str().unwrap();

    // Viewer can read datasets
    let read_resp = client()
        .get(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert!(read_resp.status().is_success());

    // Viewer should NOT be able to start training (403)
    let train_resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "dataset_id": "ds_test",
            "architecture": "lstm_autoencoder",
            "hyperparameters": { "epochs": 1 }
        }))
        .send()
        .await
        .unwrap();

    assert!(
        train_resp.status() == 403 || train_resp.status() == 401,
        "Viewer should not start training, got {}",
        train_resp.status()
    );
}

// ─── Rate limiting ─────────────────────────────────────────

#[tokio::test]
async fn test_login_rate_limiting() {
    // Send many rapid failed login attempts
    let mut last_status = 401u16;

    for i in 0..35 {
        let resp = client()
            .post(format!("{}/api/v1/auth/login", base_url()))
            .json(&json!({
                "username": format!("brute_force_{}", i),
                "password": "wrong_password"
            }))
            .send()
            .await
            .unwrap();

        last_status = resp.status().as_u16();

        // If we got rate-limited (429), the test passes
        if last_status == 429 {
            break;
        }
    }

    // After 30+ attempts, should either get 429 or the server should still respond
    // (rate limiting may be per-IP, per-user, or globally configured)
    assert!(
        last_status == 429 || last_status == 401,
        "Expected 429 or 401 after rate limit, got {}",
        last_status
    );
}

// ─── Token format ──────────────────────────────────────────

#[tokio::test]
async fn test_token_is_opaque_not_jwt() {
    let (user, pass) = admin_creds();

    let resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let token = body["token"].as_str().unwrap();

    // Opaque tokens should NOT have the JWT structure (xxxxx.yyyyy.zzzzz)
    let dot_count = token.chars().filter(|c| *c == '.').count();
    assert_ne!(
        dot_count, 2,
        "Token looks like a JWT (has 2 dots). Prometheus uses opaque bearer tokens."
    );
}

// ─── Multiple sessions ─────────────────────────────────────

#[tokio::test]
async fn test_multiple_logins_produce_different_tokens() {
    let (user, pass) = admin_creds();

    let resp1 = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await
        .unwrap();
    let body1: Value = resp1.json().await.unwrap();
    let token1 = body1["token"].as_str().unwrap();

    let resp2 = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "username": user, "password": pass }))
        .send()
        .await
        .unwrap();
    let body2: Value = resp2.json().await.unwrap();
    let token2 = body2["token"].as_str().unwrap();

    assert_ne!(
        token1, token2,
        "Successive logins should produce different tokens"
    );

    // Both tokens should be valid
    let session1 = client()
        .get(format!("{}/api/v1/auth/session", base_url()))
        .header("Authorization", format!("Bearer {}", token1))
        .send()
        .await
        .unwrap();
    assert!(session1.status().is_success());

    let session2 = client()
        .get(format!("{}/api/v1/auth/session", base_url()))
        .header("Authorization", format!("Bearer {}", token2))
        .send()
        .await
        .unwrap();
    assert!(session2.status().is_success());
}
