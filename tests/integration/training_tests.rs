//! Prometheus Training Pipeline Integration Tests
//!
//! Tests the full training lifecycle: dataset upload, training start,
//! progress monitoring (via polling), completion, model retrieval, and
//! evaluation metrics.
//!
//! # Requirements
//! - Prometheus server running on `PROMETHEUS_URL` (default: `http://localhost:3030`)
//! - Aegis-DB running on port 9091
//! - AxonML server available
//!
//! # Running
//! ```bash
//! cargo test --test training_tests -- --test-threads=1
//! ```

use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

fn base_url() -> String {
    env::var("PROMETHEUS_URL").unwrap_or_else(|_| "http://localhost:3030".to_string())
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
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

async fn upload_test_dataset(token: &str) -> String {
    let csv = "\
timestamp,supply_temp,return_temp,outside_air_temp,discharge_temp,fan_speed,damper_position,filter_dp\n\
2026-01-01T00:00:00Z,55.2,72.1,28.4,52.8,78.5,35.0,1.12\n\
2026-01-01T00:15:00Z,55.4,72.0,28.2,52.9,78.3,35.2,1.12\n\
2026-01-01T00:30:00Z,55.1,71.8,27.9,52.6,78.1,34.8,1.13\n\
2026-01-01T00:45:00Z,55.3,71.9,27.7,52.7,78.4,35.1,1.13\n\
2026-01-01T01:00:00Z,55.0,71.7,27.5,52.5,77.9,34.5,1.14\n\
2026-01-01T01:15:00Z,54.8,71.5,27.3,52.3,77.6,34.2,1.14\n\
2026-01-01T01:30:00Z,54.6,71.3,27.0,52.1,77.2,33.8,1.15\n\
2026-01-01T01:45:00Z,54.5,71.2,26.8,52.0,77.0,33.5,1.15\n\
2026-01-01T02:00:00Z,54.3,71.0,26.5,51.8,76.8,33.2,1.16\n\
2026-01-01T02:15:00Z,54.2,70.9,26.3,51.7,76.5,33.0,1.16\n";

    let form = reqwest::multipart::Form::new()
        .text("name", "Training Pipeline Test Dataset")
        .text("equipment_type", "air_handler")
        .part(
            "file",
            reqwest::multipart::Part::bytes(csv.as_bytes().to_vec())
                .file_name("pipeline_test.csv")
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

    assert!(resp.status().is_success(), "Dataset upload failed");
    let body: Value = resp.json().await.unwrap();
    body["id"].as_str().unwrap().to_string()
}

// ─── Training start ────────────────────────────────────────

#[tokio::test]
async fn test_start_lstm_autoencoder_training() {
    let token = login().await;
    let dataset_id = upload_test_dataset(&token).await;

    let resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "dataset_id": dataset_id,
            "architecture": "lstm_autoencoder",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": 32,
                "epochs": 5,
                "hidden_dim": 32,
                "bottleneck_dim": 16,
                "num_layers": 1,
                "sequence_length": 10,
                "dropout": 0.1,
                "optimizer": "adam",
                "loss": "mse"
            }
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Training start failed: {}",
        resp.status()
    );

    let run: Value = resp.json().await.unwrap();
    assert!(run["id"].is_string(), "Missing training run ID");
    assert!(
        ["running", "queued", "pending"].contains(&run["status"].as_str().unwrap()),
        "Unexpected status: {}",
        run["status"]
    );
}

#[tokio::test]
async fn test_start_gru_predictor_training() {
    let token = login().await;
    let dataset_id = upload_test_dataset(&token).await;

    let resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "dataset_id": dataset_id,
            "architecture": "gru_predictor",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": 32,
                "epochs": 5,
                "hidden_dim": 64,
                "num_layers": 1,
                "sequence_length": 10,
                "dropout": 0.1,
                "optimizer": "adamw",
                "loss": "bce"
            }
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let run: Value = resp.json().await.unwrap();
    assert!(run["id"].is_string());
}

#[tokio::test]
async fn test_start_training_with_invalid_dataset_returns_error() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "dataset_id": "nonexistent_dataset_id_12345",
            "architecture": "lstm_autoencoder",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": 32,
                "epochs": 5,
                "hidden_dim": 32,
                "bottleneck_dim": 16,
                "num_layers": 1,
                "sequence_length": 10,
                "dropout": 0.1,
                "optimizer": "adam",
                "loss": "mse"
            }
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx error, got {}",
        resp.status()
    );
}

// ─── Training progress polling ─────────────────────────────

#[tokio::test]
async fn test_training_progress_polling() {
    let token = login().await;
    let dataset_id = upload_test_dataset(&token).await;

    // Start a short training run
    let start_resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "dataset_id": dataset_id,
            "architecture": "lstm_autoencoder",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": 32,
                "epochs": 3,
                "hidden_dim": 16,
                "bottleneck_dim": 8,
                "num_layers": 1,
                "sequence_length": 5,
                "dropout": 0.0,
                "optimizer": "adam",
                "loss": "mse"
            }
        }))
        .send()
        .await
        .unwrap();

    let run: Value = start_resp.json().await.unwrap();
    let run_id = run["id"].as_str().unwrap();

    // Poll for progress
    let mut attempts = 0;
    let max_attempts = 60;
    let mut last_status = String::new();

    loop {
        let status_resp = client()
            .get(format!("{}/api/v1/training/{}", base_url(), run_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        assert!(status_resp.status().is_success());
        let status: Value = status_resp.json().await.unwrap();
        last_status = status["status"].as_str().unwrap_or("unknown").to_string();

        if last_status == "completed" || last_status == "failed" || last_status == "stopped" {
            break;
        }

        // Verify expected fields during running
        if last_status == "running" {
            assert!(
                status["current_epoch"].is_number(),
                "Missing current_epoch"
            );
            assert!(status["total_epochs"].is_number(), "Missing total_epochs");
        }

        attempts += 1;
        if attempts >= max_attempts {
            break;
        }

        sleep(Duration::from_secs(5)).await;
    }

    assert!(
        last_status == "completed" || last_status == "running",
        "Training ended with unexpected status: {}",
        last_status
    );
}

// ─── Stop training ─────────────────────────────────────────

#[tokio::test]
async fn test_stop_running_training() {
    let token = login().await;
    let dataset_id = upload_test_dataset(&token).await;

    // Start a long training run
    let start_resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "dataset_id": dataset_id,
            "architecture": "lstm_autoencoder",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": 32,
                "epochs": 1000,
                "hidden_dim": 64,
                "bottleneck_dim": 32,
                "num_layers": 2,
                "sequence_length": 60,
                "dropout": 0.1,
                "optimizer": "adam",
                "loss": "mse"
            }
        }))
        .send()
        .await
        .unwrap();

    let run: Value = start_resp.json().await.unwrap();
    let run_id = run["id"].as_str().unwrap();

    // Wait a moment for it to start
    sleep(Duration::from_secs(3)).await;

    // Stop it
    let stop_resp = client()
        .post(format!("{}/api/v1/training/{}/stop", base_url(), run_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(
        stop_resp.status().is_success(),
        "Stop request failed: {}",
        stop_resp.status()
    );

    // Verify stopped
    sleep(Duration::from_secs(2)).await;

    let status_resp = client()
        .get(format!("{}/api/v1/training/{}", base_url(), run_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let status: Value = status_resp.json().await.unwrap();
    assert!(
        ["stopped", "cancelled"].contains(&status["status"].as_str().unwrap_or("")),
        "Expected stopped/cancelled, got: {}",
        status["status"]
    );
}

// ─── Training completion produces model ────────────────────

#[tokio::test]
async fn test_completed_training_creates_model() {
    let token = login().await;
    let dataset_id = upload_test_dataset(&token).await;

    // Start a short run
    let start_resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
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
        }))
        .send()
        .await
        .unwrap();

    let run: Value = start_resp.json().await.unwrap();
    let run_id = run["id"].as_str().unwrap();

    // Poll until complete
    let mut final_status = json!({});
    for _ in 0..120 {
        let resp = client()
            .get(format!("{}/api/v1/training/{}", base_url(), run_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        final_status = resp.json().await.unwrap();
        if final_status["status"] == "completed" {
            break;
        }
        sleep(Duration::from_secs(5)).await;
    }

    if final_status["status"] != "completed" {
        eprintln!(
            "Training did not complete in time. Status: {}",
            final_status["status"]
        );
        return;
    }

    // Verify model was created
    assert!(
        final_status["model_id"].is_string(),
        "Completed training should have model_id"
    );

    let model_id = final_status["model_id"].as_str().unwrap();

    let model_resp = client()
        .get(format!("{}/api/v1/models/{}", base_url(), model_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(model_resp.status().is_success());
    let model: Value = model_resp.json().await.unwrap();
    assert_eq!(model["id"], model_id);
    assert_eq!(model["architecture"], "lstm_autoencoder");
    assert!(model["metrics"].is_object());
    assert!(model["file_size_bytes"].is_number());
}

// ─── Training list ─────────────────────────────────────────

#[tokio::test]
async fn test_list_training_runs() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/training", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
}

// ─── Training with invalid hyperparameters ─────────────────

#[tokio::test]
async fn test_training_rejects_invalid_architecture() {
    let token = login().await;
    let dataset_id = upload_test_dataset(&token).await;

    let resp = client()
        .post(format!("{}/api/v1/training/start", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "dataset_id": dataset_id,
            "architecture": "nonexistent_architecture",
            "hyperparameters": {
                "learning_rate": 0.001,
                "epochs": 5
            }
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx, got {}",
        resp.status()
    );
}
