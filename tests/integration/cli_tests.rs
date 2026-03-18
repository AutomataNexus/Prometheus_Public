//! Prometheus CLI Authentication Integration Tests
//!
//! Tests the device-authorization-like CLI auth flow:
//!   1. POST /api/v1/auth/cli/init — create a pending session
//!   2. GET  /api/v1/auth/cli/poll?code=<session_code> — poll for status
//!   3. POST /api/v1/auth/cli/verify — link a token to a session
//!
//! # Requirements
//! - Prometheus server running on `PROMETHEUS_URL` (default: `http://localhost:3030`)
//! - Aegis-DB running on port 9091
//!
//! # Running
//! ```bash
//! cargo test --test cli_tests
//! ```

use serde_json::{json, Value};
use std::env;

fn base_url() -> String {
    env::var("PROMETHEUS_URL").unwrap_or_else(|_| "http://localhost:3030".to_string())
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn login() -> String {
    let url = format!("{}/api/v1/auth/login", base_url());
    let resp = client()
        .post(&url)
        .json(&json!({
            "username": env::var("TEST_ADMIN_USER").unwrap_or_else(|_| "admin".into()),
            "password": env::var("TEST_ADMIN_PASS").unwrap_or_else(|_| "admin_password".into()),
        }))
        .send()
        .await
        .expect("Failed to send login request");

    assert!(resp.status().is_success(), "Login failed: {}", resp.status());

    let body: Value = resp.json().await.expect("Failed to parse login response");
    body["token"]
        .as_str()
        .expect("No token in login response")
        .to_string()
}

// ─── CLI auth init ──────────────────────────────────────────

#[tokio::test]
async fn test_cli_init_creates_pending_session() {
    let session_code = format!("test_cli_{}", uuid_v4_stub());

    let resp = client()
        .post(format!("{}/api/v1/auth/cli/init", base_url()))
        .json(&json!({ "session_code": session_code }))
        .send()
        .await
        .expect("CLI init request failed");

    assert!(
        resp.status().is_success(),
        "CLI init failed: {}",
        resp.status()
    );

    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body["session_code"].as_str().unwrap(),
        session_code,
        "Returned session_code should match the requested one"
    );
    assert!(
        body["verify_url"].is_string(),
        "Response should include a verify_url"
    );
    assert!(
        body["verify_url"]
            .as_str()
            .unwrap()
            .contains(&session_code),
        "verify_url should contain the session code"
    );
    assert!(
        body["expires_in"].is_number(),
        "Response should include expires_in"
    );
}

#[tokio::test]
async fn test_cli_init_returns_auth_url_with_code() {
    let session_code = format!("test_url_{}", uuid_v4_stub());

    let resp = client()
        .post(format!("{}/api/v1/auth/cli/init", base_url()))
        .json(&json!({ "session_code": session_code }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());

    let body: Value = resp.json().await.unwrap();
    let verify_url = body["verify_url"].as_str().unwrap();

    // The auth URL should be a valid URL containing the code parameter
    assert!(
        verify_url.starts_with("http"),
        "verify_url should be a full URL: {}",
        verify_url
    );
    assert!(
        verify_url.contains("/auth/verify"),
        "verify_url should point to the /auth/verify page: {}",
        verify_url
    );
    assert!(
        verify_url.contains(&format!("code={}", session_code)),
        "verify_url should include code=<session_code>: {}",
        verify_url
    );
}

// ─── CLI auth poll (pending) ───────────────────────────────

#[tokio::test]
async fn test_cli_poll_returns_pending_initially() {
    let session_code = format!("test_poll_{}", uuid_v4_stub());

    // Create session first
    let init_resp = client()
        .post(format!("{}/api/v1/auth/cli/init", base_url()))
        .json(&json!({ "session_code": session_code }))
        .send()
        .await
        .unwrap();
    assert!(init_resp.status().is_success());

    // Poll for the session — should be pending
    let poll_resp = client()
        .get(format!(
            "{}/api/v1/auth/cli/poll?code={}",
            base_url(),
            session_code
        ))
        .send()
        .await
        .unwrap();

    assert!(
        poll_resp.status().is_success(),
        "Poll request failed: {}",
        poll_resp.status()
    );

    let body: Value = poll_resp.json().await.unwrap();
    assert_eq!(
        body["status"].as_str().unwrap(),
        "pending",
        "Session should be pending before verification"
    );
    // Token should not be present in pending state
    assert!(
        body.get("token").is_none() || body["token"].is_null(),
        "Token should not be returned while pending"
    );
}

// ─── CLI auth verify ───────────────────────────────────────

#[tokio::test]
async fn test_cli_verify_links_token_to_session() {
    let session_code = format!("test_verify_{}", uuid_v4_stub());

    // Step 1: Create pending session
    let init_resp = client()
        .post(format!("{}/api/v1/auth/cli/init", base_url()))
        .json(&json!({ "session_code": session_code }))
        .send()
        .await
        .unwrap();
    assert!(init_resp.status().is_success());

    // Step 2: Get a real auth token by logging in
    let token = login().await;

    // Step 3: Verify the CLI session (simulates user completing browser auth)
    let verify_resp = client()
        .post(format!("{}/api/v1/auth/cli/verify", base_url()))
        .json(&json!({
            "code": session_code,
            "token": token,
            "username": "admin",
            "role": "admin",
        }))
        .send()
        .await
        .unwrap();

    assert!(
        verify_resp.status().is_success(),
        "Verify request failed: {}",
        verify_resp.status()
    );

    let body: Value = verify_resp.json().await.unwrap();
    assert_eq!(
        body["status"].as_str().unwrap(),
        "verified",
        "Verify response should have status=verified"
    );
    assert!(
        body["message"].is_string(),
        "Verify response should include a user-facing message"
    );
}

// ─── CLI auth poll (after verification) ─────────────────────

#[tokio::test]
async fn test_cli_poll_returns_authenticated_after_verify() {
    let session_code = format!("test_flow_{}", uuid_v4_stub());

    // Step 1: Init
    let init_resp = client()
        .post(format!("{}/api/v1/auth/cli/init", base_url()))
        .json(&json!({ "session_code": session_code }))
        .send()
        .await
        .unwrap();
    assert!(init_resp.status().is_success());

    // Step 2: Poll — should be pending
    let poll_pending = client()
        .get(format!(
            "{}/api/v1/auth/cli/poll?code={}",
            base_url(),
            session_code
        ))
        .send()
        .await
        .unwrap();
    assert!(poll_pending.status().is_success());
    let pending_body: Value = poll_pending.json().await.unwrap();
    assert_eq!(pending_body["status"].as_str().unwrap(), "pending");

    // Step 3: Verify with a real token
    let token = login().await;
    let verify_resp = client()
        .post(format!("{}/api/v1/auth/cli/verify", base_url()))
        .json(&json!({
            "code": session_code,
            "token": token,
            "username": "admin",
            "role": "admin",
        }))
        .send()
        .await
        .unwrap();
    assert!(verify_resp.status().is_success());

    // Step 4: Poll again — should now be verified with token
    let poll_verified = client()
        .get(format!(
            "{}/api/v1/auth/cli/poll?code={}",
            base_url(),
            session_code
        ))
        .send()
        .await
        .unwrap();
    assert!(
        poll_verified.status().is_success(),
        "Poll after verify failed: {}",
        poll_verified.status()
    );

    let verified_body: Value = poll_verified.json().await.unwrap();
    assert_eq!(
        verified_body["status"].as_str().unwrap(),
        "verified",
        "Status should be 'verified' after user verifies in browser"
    );
    assert!(
        verified_body["token"].is_string(),
        "Verified response should include the auth token"
    );
    assert!(
        verified_body["token"].as_str().unwrap().len() > 10,
        "Token should be non-trivial"
    );
    assert_eq!(
        verified_body["username"].as_str().unwrap(),
        "admin",
        "Username should match the verifying user"
    );
    assert_eq!(
        verified_body["role"].as_str().unwrap(),
        "admin",
        "Role should match the verifying user"
    );
}

// ─── Invalid session_code ──────────────────────────────────

#[tokio::test]
async fn test_cli_poll_invalid_session_code_returns_error() {
    let resp = client()
        .get(format!(
            "{}/api/v1/auth/cli/poll?code=nonexistent_session_xyz_99999",
            base_url()
        ))
        .send()
        .await
        .unwrap();

    // Should return 404 or a 4xx error for unknown session codes
    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "Polling for nonexistent session should return an error, got {}",
        resp.status()
    );

    let status = resp.status().as_u16();
    assert!(
        status == 404 || status == 400 || status == 500,
        "Expected 404/400/500 for invalid session code, got {}",
        status
    );
}

#[tokio::test]
async fn test_cli_verify_invalid_session_code_returns_error() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/auth/cli/verify", base_url()))
        .json(&json!({
            "code": "nonexistent_session_xyz_12345",
            "token": token,
            "username": "admin",
            "role": "admin",
        }))
        .send()
        .await
        .unwrap();

    // Verifying a non-existent session should fail
    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "Verifying nonexistent session should fail, got {}",
        resp.status()
    );
}

// ─── Session expiry behavior ───────────────────────────────

#[tokio::test]
async fn test_cli_init_returns_expires_in_field() {
    let session_code = format!("test_expiry_{}", uuid_v4_stub());

    let resp = client()
        .post(format!("{}/api/v1/auth/cli/init", base_url()))
        .json(&json!({ "session_code": session_code }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());

    let body: Value = resp.json().await.unwrap();
    let expires_in = body["expires_in"].as_u64().expect("expires_in should be a number");
    assert!(
        expires_in > 0,
        "expires_in should be positive (got {})",
        expires_in
    );
    assert!(
        expires_in <= 600,
        "expires_in should be reasonable (<=600s), got {}",
        expires_in
    );
}

#[tokio::test]
async fn test_cli_session_consumed_after_poll_verified() {
    let session_code = format!("test_consume_{}", uuid_v4_stub());

    // Init
    let init_resp = client()
        .post(format!("{}/api/v1/auth/cli/init", base_url()))
        .json(&json!({ "session_code": session_code }))
        .send()
        .await
        .unwrap();
    assert!(init_resp.status().is_success());

    // Verify
    let token = login().await;
    let verify_resp = client()
        .post(format!("{}/api/v1/auth/cli/verify", base_url()))
        .json(&json!({
            "code": session_code,
            "token": token,
            "username": "admin",
            "role": "admin",
        }))
        .send()
        .await
        .unwrap();
    assert!(verify_resp.status().is_success());

    // First poll — consumes the session
    let poll1 = client()
        .get(format!(
            "{}/api/v1/auth/cli/poll?code={}",
            base_url(),
            session_code
        ))
        .send()
        .await
        .unwrap();
    assert!(poll1.status().is_success());
    let body1: Value = poll1.json().await.unwrap();
    assert_eq!(body1["status"].as_str().unwrap(), "verified");

    // Second poll — session was deleted after first successful poll,
    // so this should return an error (404 or similar)
    let poll2 = client()
        .get(format!(
            "{}/api/v1/auth/cli/poll?code={}",
            base_url(),
            session_code
        ))
        .send()
        .await
        .unwrap();

    let status2 = poll2.status().as_u16();
    assert!(
        status2 == 404 || status2 == 400 || status2 == 500,
        "Session should be consumed after verified poll, but second poll returned {}",
        status2
    );
}

// ─── Full end-to-end flow ──────────────────────────────────

#[tokio::test]
async fn test_cli_full_auth_flow_init_poll_verify_poll() {
    let session_code = format!("test_e2e_{}", uuid_v4_stub());

    // 1. Init — creates pending session
    let init_resp = client()
        .post(format!("{}/api/v1/auth/cli/init", base_url()))
        .json(&json!({ "session_code": session_code }))
        .send()
        .await
        .unwrap();
    assert!(init_resp.status().is_success());
    let init_body: Value = init_resp.json().await.unwrap();
    assert_eq!(init_body["session_code"].as_str().unwrap(), session_code);
    assert!(init_body["verify_url"].as_str().unwrap().len() > 10);

    // 2. Poll — pending
    let poll1 = client()
        .get(format!(
            "{}/api/v1/auth/cli/poll?code={}",
            base_url(),
            session_code
        ))
        .send()
        .await
        .unwrap();
    assert!(poll1.status().is_success());
    let poll1_body: Value = poll1.json().await.unwrap();
    assert_eq!(poll1_body["status"].as_str().unwrap(), "pending");

    // 3. User authenticates in browser and verifies the CLI session
    let token = login().await;
    let verify_resp = client()
        .post(format!("{}/api/v1/auth/cli/verify", base_url()))
        .json(&json!({
            "code": session_code,
            "token": token,
            "username": "admin",
            "role": "admin",
        }))
        .send()
        .await
        .unwrap();
    assert!(verify_resp.status().is_success());

    // 4. Poll — verified, returns token
    let poll2 = client()
        .get(format!(
            "{}/api/v1/auth/cli/poll?code={}",
            base_url(),
            session_code
        ))
        .send()
        .await
        .unwrap();
    assert!(poll2.status().is_success());
    let poll2_body: Value = poll2.json().await.unwrap();
    assert_eq!(poll2_body["status"].as_str().unwrap(), "verified");
    let returned_token = poll2_body["token"].as_str().unwrap();
    assert!(returned_token.len() > 10);

    // 5. The returned token should be usable for API requests
    let api_resp = client()
        .get(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", returned_token))
        .send()
        .await
        .unwrap();
    assert!(
        api_resp.status().is_success(),
        "Token from CLI auth should be valid for API requests, got {}",
        api_resp.status()
    );
}

// ─── Helpers ────────────────────────────────────────────────

/// Simple pseudo-unique ID for test isolation (not a real UUID).
fn uuid_v4_stub() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{:x}{:x}", secs, nanos)
}
