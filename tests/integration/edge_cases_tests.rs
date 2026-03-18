//! Prometheus Edge Cases and Error Handling Integration Tests
//!
//! Tests boundary conditions, malformed input, concurrent operations,
//! special characters, and general error handling across the API surface.
//!
//! # Requirements
//! - Prometheus server running on `PROMETHEUS_URL` (default: `http://localhost:3030`)
//! - Aegis-DB running on port 9091
//!
//! # Running
//! ```bash
//! cargo test --test edge_cases_tests
//! ```

use serde_json::{json, Value};
use std::env;
use std::time::Duration;

fn base_url() -> String {
    env::var("PROMETHEUS_URL").unwrap_or_else(|_| "http://localhost:3030".to_string())
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap()
}

async fn login() -> String {
    let resp = client()
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({
            "username": env::var("TEST_ADMIN_USER").unwrap_or_else(|_| "admin".into()),
            "password": env::var("TEST_ADMIN_PASS").unwrap_or_else(|_| "admin_password".into()),
        }))
        .send()
        .await
        .expect("Login request failed");

    let body: Value = resp.json().await.unwrap();
    body["token"].as_str().unwrap().to_string()
}

// ─── Empty CSV upload ───────────────────────────────────────

#[tokio::test]
async fn test_empty_csv_upload_returns_error() {
    let token = login().await;

    let form = reqwest::multipart::Form::new()
        .text("name", "Empty CSV Test")
        .text("equipment_type", "air_handler")
        .part(
            "file",
            reqwest::multipart::Part::bytes(Vec::new())
                .file_name("empty.csv")
                .mime_str("text/csv")
                .unwrap(),
        );

    let resp = client()
        .post(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx for empty CSV, got {}",
        resp.status()
    );
}

// ─── Malformed CSV upload ───────────────────────────────────

#[tokio::test]
async fn test_malformed_csv_upload_returns_error() {
    let token = login().await;

    let bad_csv = "this is not,,,a proper\ncsv \"file\" with \x00 null bytes\n\x01\x02\x03";

    let form = reqwest::multipart::Form::new()
        .text("name", "Malformed CSV Test")
        .text("equipment_type", "air_handler")
        .part(
            "file",
            reqwest::multipart::Part::bytes(bad_csv.as_bytes().to_vec())
                .file_name("malformed.csv")
                .mime_str("text/csv")
                .unwrap(),
        );

    let resp = client()
        .post(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    // Should either reject the malformed data or handle it gracefully
    assert!(
        resp.status().is_client_error() || resp.status().is_success(),
        "Expected 4xx or graceful handling, got {}",
        resp.status()
    );

    // Definitely should NOT be a 500 server error
    assert_ne!(
        resp.status().as_u16(),
        500,
        "Server should not return 500 for malformed CSV"
    );
}

// ─── Very large dataset name ────────────────────────────────

#[tokio::test]
async fn test_very_large_dataset_name_is_handled() {
    let token = login().await;

    let long_name = "A".repeat(1000);
    let csv = "timestamp,value\n2026-01-01T00:00:00Z,42.0\n";

    let form = reqwest::multipart::Form::new()
        .text("name", long_name)
        .text("equipment_type", "air_handler")
        .part(
            "file",
            reqwest::multipart::Part::bytes(csv.as_bytes().to_vec())
                .file_name("long_name_test.csv")
                .mime_str("text/csv")
                .unwrap(),
        );

    let resp = client()
        .post(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    // Should be accepted or cleanly rejected
    let status = resp.status().as_u16();
    assert!(
        status < 500,
        "Server should not return 5xx for long name, got {}",
        status
    );
}

// ─── Special characters in dataset name ─────────────────────

#[tokio::test]
async fn test_special_characters_in_dataset_name() {
    let token = login().await;

    let special_name = "Test <Dataset> & \"Quotes\" 'Single' /slash\\ @#$%";
    let csv = "timestamp,value\n2026-01-01T00:00:00Z,42.0\n";

    let form = reqwest::multipart::Form::new()
        .text("name", special_name)
        .text("equipment_type", "air_handler")
        .part(
            "file",
            reqwest::multipart::Part::bytes(csv.as_bytes().to_vec())
                .file_name("special_chars_test.csv")
                .mime_str("text/csv")
                .unwrap(),
        );

    let resp = client()
        .post(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert!(
        status < 500,
        "Server should not crash on special characters, got {}",
        status
    );

    if resp.status().is_success() {
        let body: Value = resp.json().await.unwrap();
        assert!(body["id"].is_string(), "Should return dataset ID");
    }
}

// ─── Unicode in dataset names and agent messages ────────────

#[tokio::test]
async fn test_unicode_in_dataset_name() {
    let token = login().await;

    let unicode_name = "Datensatz-Test \u{00E4}\u{00F6}\u{00FC} \u{4F60}\u{597D} \u{0410}\u{0411}\u{0412} \u{1F680}";
    let csv = "timestamp,value\n2026-01-01T00:00:00Z,42.0\n";

    let form = reqwest::multipart::Form::new()
        .text("name", unicode_name)
        .text("equipment_type", "air_handler")
        .part(
            "file",
            reqwest::multipart::Part::bytes(csv.as_bytes().to_vec())
                .file_name("unicode_test.csv")
                .mime_str("text/csv")
                .unwrap(),
        );

    let resp = client()
        .post(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert!(
        status < 500,
        "Server should handle Unicode names, got {}",
        status
    );
}

#[tokio::test]
async fn test_unicode_in_agent_message() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "Analysiere bitte meine Daten \u{00E4}\u{00F6}\u{00FC}. \u{4F60}\u{597D}\u{4E16}\u{754C}! \u{041F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}"
        }))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert!(
        status < 500,
        "Agent should handle Unicode messages, got {}",
        status
    );

    if resp.status().is_success() {
        let body: Value = resp.json().await.unwrap();
        assert!(body["response"].is_string());
    }
}

// ─── Concurrent training starts ─────────────────────────────

#[tokio::test]
async fn test_concurrent_training_starts() {
    let token = login().await;

    // Upload a dataset first
    let csv = "\
timestamp,supply_temp,return_temp,fan_speed\n\
2026-01-01T00:00:00Z,55.2,72.1,78.5\n\
2026-01-01T00:15:00Z,55.4,72.0,78.3\n\
2026-01-01T00:30:00Z,55.1,71.8,78.1\n\
2026-01-01T00:45:00Z,55.3,71.9,78.4\n\
2026-01-01T01:00:00Z,55.0,71.7,77.9\n";

    let form = reqwest::multipart::Form::new()
        .text("name", "Concurrent Training Test")
        .text("equipment_type", "air_handler")
        .part(
            "file",
            reqwest::multipart::Part::bytes(csv.as_bytes().to_vec())
                .file_name("concurrent_test.csv")
                .mime_str("text/csv")
                .unwrap(),
        );

    let upload_resp = client()
        .post(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let dataset: Value = upload_resp.json().await.unwrap();
    let dataset_id = dataset["id"].as_str().unwrap();

    // Fire off multiple training starts concurrently
    let training_body = json!({
        "dataset_id": dataset_id,
        "architecture": "lstm_autoencoder",
        "hyperparameters": {
            "learning_rate": 0.01,
            "batch_size": 32,
            "epochs": 2,
            "hidden_dim": 16,
            "bottleneck_dim": 8,
            "num_layers": 1,
            "sequence_length": 5,
            "dropout": 0.0,
            "optimizer": "adam",
            "loss": "mse"
        }
    });

    let (resp1, resp2) = tokio::join!(
        client()
            .post(format!("{}/api/v1/training/start", base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&training_body)
            .send(),
        client()
            .post(format!("{}/api/v1/training/start", base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&training_body)
            .send()
    );

    let status1 = resp1.unwrap().status().as_u16();
    let status2 = resp2.unwrap().status().as_u16();

    eprintln!(
        "Concurrent training start results: {} and {}",
        status1, status2
    );

    // At least one should succeed; neither should be 500
    assert!(
        status1 < 500 && status2 < 500,
        "Server should not return 5xx for concurrent starts ({}, {})",
        status1,
        status2
    );
}

// ─── Request with malformed JSON ────────────────────────────

#[tokio::test]
async fn test_request_with_malformed_json_returns_400() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body("{this is not valid json!!!")
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx for malformed JSON, got {}",
        resp.status()
    );
}

// ─── Request with wrong content-type ────────────────────────

#[tokio::test]
async fn test_request_with_wrong_content_type_returns_error() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "text/plain")
        .body("dataset_id=abc&architecture=lstm")
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx for wrong content-type, got {}",
        resp.status()
    );
}

// ─── Very long agent chat message ───────────────────────────

#[tokio::test]
async fn test_very_long_agent_chat_message() {
    let token = login().await;

    let long_message = "Analyze the following sensor data point: ".to_string()
        + &"42.5, ".repeat(1500)
        + "and recommend an architecture.";

    // Approximately 10,000 characters
    assert!(
        long_message.len() > 9000,
        "Message should be over 9000 chars, got {}",
        long_message.len()
    );

    let resp = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "message": long_message }))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    // Should succeed or return 413/400, but never 500
    assert!(
        status < 500,
        "Server should handle very long messages gracefully, got {}",
        status
    );

    if resp.status().is_success() {
        let body: Value = resp.json().await.unwrap();
        assert!(body["response"].is_string());
    }
}

// ─── Rapid sequential API calls ─────────────────────────────

#[tokio::test]
async fn test_rapid_sequential_api_calls_authenticated() {
    let token = login().await;

    let mut success_count = 0u32;
    let total_calls = 20u32;

    for _ in 0..total_calls {
        let resp = client()
            .get(format!("{}/api/v1/datasets", base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        if resp.status().is_success() {
            success_count += 1;
        }
    }

    // Authenticated requests should not be rate-limited under normal load
    assert!(
        success_count >= total_calls / 2,
        "Expected at least half of {} rapid calls to succeed, got {}",
        total_calls,
        success_count
    );

    eprintln!(
        "Rapid sequential calls: {}/{} succeeded",
        success_count, total_calls
    );
}

// ─── Empty request body where body expected ─────────────────

#[tokio::test]
async fn test_empty_body_where_expected_returns_400() {
    let token = login().await;

    // POST to training/start with empty body
    let resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body("")
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx for empty body on training start, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_empty_json_object_where_fields_expected_returns_400() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx for empty JSON body on training start, got {}",
        resp.status()
    );
}

// ─── Nonexistent endpoint returns 404 ───────────────────────

#[tokio::test]
async fn test_nonexistent_endpoint_returns_404() {
    let token = login().await;

    let resp = client()
        .get(format!(
            "{}/api/v1/this_endpoint_does_not_exist",
            base_url()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        404,
        "Nonexistent endpoint should return 404, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_nonexistent_nested_endpoint_returns_404() {
    let token = login().await;

    let resp = client()
        .get(format!(
            "{}/api/v1/datasets/abc123/nonexistent_action",
            base_url()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status() == 404 || resp.status() == 405,
        "Expected 404 or 405 for nonexistent nested endpoint, got {}",
        resp.status()
    );
}

// ─── Dataset ID that looks like path traversal ──────────────

#[tokio::test]
async fn test_path_traversal_in_dataset_id_is_rejected() {
    let token = login().await;

    let malicious_ids = vec![
        "../../../etc/passwd",
        "..%2F..%2F..%2Fetc%2Fpasswd",
        "....//....//etc/passwd",
        "%00null_byte",
    ];

    for mal_id in malicious_ids {
        let resp = client()
            .get(format!("{}/api/v1/datasets/{}", base_url(), mal_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        let status = resp.status().as_u16();
        assert!(
            status == 400 || status == 404 || status == 403,
            "Path traversal attempt '{}' should be rejected, got {}",
            mal_id,
            status
        );
    }
}

// ─── Very large JSON payload ────────────────────────────────

#[tokio::test]
async fn test_very_large_json_payload_is_handled() {
    let token = login().await;

    // Build a large JSON payload with many hyperparameters
    let mut large_params = serde_json::Map::new();
    for i in 0..500 {
        large_params.insert(
            format!("param_{}", i),
            json!(i as f64 * 0.001),
        );
    }

    let resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "dataset_id": "some_dataset_id",
            "architecture": "lstm_autoencoder",
            "hyperparameters": Value::Object(large_params)
        }))
        .send()
        .await
        .unwrap();

    // Should either reject unknown params or ignore them
    let status = resp.status().as_u16();
    assert!(
        status < 500,
        "Server should handle large JSON payloads without crashing, got {}",
        status
    );
}
