//! Prometheus API Integration Tests
//!
//! Full lifecycle tests against a running Prometheus server backed by Aegis-DB.
//! These tests exercise the REST API endpoints for datasets, models, training,
//! deployments, and evaluations.
//!
//! # Requirements
//! - Prometheus server running on `PROMETHEUS_URL` (default: `http://localhost:3030`)
//! - Aegis-DB running on port 9091
//!
//! # Running
//! ```bash
//! cargo test --test api_tests
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

// ─── Health check ──────────────────────────────────────────

#[tokio::test]
async fn test_health_endpoint_is_accessible() {
    let resp = client()
        .get(format!("{}/health", base_url()))
        .send()
        .await
        .expect("Health endpoint unreachable");

    assert!(resp.status().is_success());
}

#[tokio::test]
async fn test_health_returns_ok_status() {
    let resp = client()
        .get(format!("{}/health", base_url()))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

// ─── Authentication required ───────────────────────────────

#[tokio::test]
async fn test_unauthenticated_datasets_returns_401() {
    let resp = client()
        .get(format!("{}/api/v1/datasets", base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_invalid_token_returns_401() {
    let resp = client()
        .get(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", "Bearer invalid_token_abc123")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ─── Dataset lifecycle ─────────────────────────────────────

#[tokio::test]
async fn test_dataset_upload_and_retrieval() {
    let token = login().await;

    // Upload CSV
    let csv_content = "\
timestamp,supply_temp,return_temp,outside_air_temp,fan_speed\n\
2026-01-01T00:00:00Z,55.2,72.1,28.4,78.5\n\
2026-01-01T00:15:00Z,55.4,72.0,28.2,78.3\n\
2026-01-01T00:30:00Z,55.1,71.8,27.9,78.1\n";

    let form = reqwest::multipart::Form::new()
        .text("name", "Integration Test AHU")
        .text("equipment_type", "air_handler")
        .part(
            "file",
            reqwest::multipart::Part::bytes(csv_content.as_bytes().to_vec())
                .file_name("test_ahu.csv")
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

    assert!(
        upload_resp.status().is_success(),
        "Upload failed: {}",
        upload_resp.status()
    );

    let dataset: Value = upload_resp.json().await.unwrap();
    let dataset_id = dataset["id"].as_str().expect("No dataset ID");

    // Retrieve
    let get_resp = client()
        .get(format!("{}/api/v1/datasets/{}", base_url(), dataset_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(get_resp.status().is_success());
    let detail: Value = get_resp.json().await.unwrap();
    assert_eq!(detail["id"], dataset_id);
    assert!(detail["columns"].is_array());
    assert!(detail["row_count"].is_number());

    // List
    let list_resp = client()
        .get(format!("{}/api/v1/datasets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(list_resp.status().is_success());
    let datasets: Value = list_resp.json().await.unwrap();
    assert!(datasets.is_array());

    // Preview
    let preview_resp = client()
        .get(format!(
            "{}/api/v1/datasets/{}/preview",
            base_url(),
            dataset_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(preview_resp.status().is_success());

    // Delete
    let del_resp = client()
        .delete(format!("{}/api/v1/datasets/{}", base_url(), dataset_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(del_resp.status().is_success());

    // Confirm deleted
    let get_deleted = client()
        .get(format!("{}/api/v1/datasets/{}", base_url(), dataset_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(get_deleted.status(), 404);
}

// ─── Model lifecycle ───────────────────────────────────────

#[tokio::test]
async fn test_list_models() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/models", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn test_model_detail_and_download() {
    let token = login().await;

    // List models
    let resp = client()
        .get(format!("{}/api/v1/models", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let models: Value = resp.json().await.unwrap();
    let models_arr = models.as_array().unwrap();

    if models_arr.is_empty() {
        eprintln!("No models available — skipping model detail test");
        return;
    }

    let model_id = models_arr[0]["id"].as_str().unwrap();

    // Detail
    let detail_resp = client()
        .get(format!("{}/api/v1/models/{}", base_url(), model_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(detail_resp.status().is_success());
    let detail: Value = detail_resp.json().await.unwrap();
    assert_eq!(detail["id"], model_id);
    assert!(detail["architecture"].is_string());
    assert!(detail["metrics"].is_object());

    // Download
    let dl_resp = client()
        .get(format!(
            "{}/api/v1/models/{}/download",
            base_url(),
            model_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(dl_resp.status().is_success());
    let content_type = dl_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("octet-stream") || content_type.contains("binary"),
        "Expected binary content type, got: {}",
        content_type
    );
}

#[tokio::test]
async fn test_model_comparison() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/models", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let models: Value = resp.json().await.unwrap();
    let arr = models.as_array().unwrap();

    if arr.len() < 2 {
        eprintln!("Need at least 2 models for comparison test — skipping");
        return;
    }

    let id_a = arr[0]["id"].as_str().unwrap();
    let id_b = arr[1]["id"].as_str().unwrap();

    let compare_resp = client()
        .post(format!("{}/api/v1/models/{}/compare", base_url(), id_a))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "compare_with": id_b }))
        .send()
        .await
        .unwrap();

    assert!(compare_resp.status().is_success());
    let comparison: Value = compare_resp.json().await.unwrap();
    assert!(comparison["models"].is_array());
}

// ─── Deployment lifecycle ──────────────────────────────────

#[tokio::test]
async fn test_list_deployments() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn test_list_edge_targets() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/deployments/targets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
}

// ─── Evaluation lifecycle ──────────────────────────────────

#[tokio::test]
async fn test_list_evaluations() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
}

// ─── Agent endpoints ───────────────────────────────────────

#[tokio::test]
async fn test_agent_chat() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "What model architectures are available?"
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body["response"].is_string());
    assert!(
        body["response"].as_str().unwrap().len() > 10,
        "Agent response too short"
    );
}

#[tokio::test]
async fn test_agent_history() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/agent/history", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
}

// ─── System metrics ────────────────────────────────────────

#[tokio::test]
async fn test_system_metrics() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/system/metrics", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
}
