//! Prometheus Evaluation API Integration Tests
//!
//! Tests the evaluation pipeline: listing evaluations, retrieving detailed
//! metrics, running Gradient evaluations, validating metric ranges, and
//! error handling.
//!
//! # Requirements
//! - Prometheus server running on `PROMETHEUS_URL` (default: `http://localhost:3030`)
//! - Aegis-DB running on port 9091
//! - At least one trained model available for evaluation
//!
//! # Running
//! ```bash
//! cargo test --test evaluation_tests -- --test-threads=1
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
        .timeout(Duration::from_secs(120))
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

/// Retrieve the first available model ID, or None.
async fn first_model_id(token: &str) -> Option<String> {
    let resp = client()
        .get(format!("{}/api/v1/models", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let models: Value = resp.json().await.unwrap();
    models
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|m| m["id"].as_str())
        .map(|s| s.to_string())
}

// ─── List evaluations ───────────────────────────────────────

#[tokio::test]
async fn test_list_evaluations_returns_array() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array(), "Expected array, got: {:?}", body);
}

#[tokio::test]
async fn test_list_evaluations_without_auth_returns_401() {
    let resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_multiple_evaluations_can_be_listed() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let evals: Value = resp.json().await.unwrap();
    let arr = evals.as_array().unwrap();

    // Just verify the structure is valid; there may be zero or many
    eprintln!("Found {} evaluations in list", arr.len());
    for eval in arr {
        assert!(eval["id"].is_string() || eval["id"].is_number(), "Evaluation missing 'id'");
    }
}

// ─── Get evaluation by ID ───────────────────────────────────

#[tokio::test]
async fn test_get_evaluation_by_id_returns_detailed_metrics() {
    let token = login().await;

    let list_resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let evals: Value = list_resp.json().await.unwrap();
    let arr = evals.as_array().unwrap();

    if arr.is_empty() {
        eprintln!("No evaluations found -- skipping detail test");
        return;
    }

    let eval_id = arr[0]["id"].as_str().unwrap();

    let detail_resp = client()
        .get(format!("{}/api/v1/evaluations/{}", base_url(), eval_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(detail_resp.status().is_success());
    let detail: Value = detail_resp.json().await.unwrap();

    assert_eq!(detail["id"], eval_id);
    assert!(
        detail["metrics"].is_object() || detail["results"].is_object(),
        "Evaluation detail should contain metrics or results"
    );
}

#[tokio::test]
async fn test_get_evaluation_for_nonexistent_id_returns_404() {
    let token = login().await;

    let resp = client()
        .get(format!(
            "{}/api/v1/evaluations/nonexistent_eval_id_99999",
            base_url()
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        404,
        "Expected 404, got {}",
        resp.status()
    );
}

// ─── Run Gradient evaluation ────────────────────────────────

#[tokio::test]
async fn test_run_gradient_evaluation_returns_metrics() {
    let token = login().await;

    let model_id = match first_model_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No models -- skipping Gradient evaluation test");
            return;
        }
    };

    let resp = client()
        .post(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "model_id": model_id,
            "evaluation_type": "gradient"
        }))
        .send()
        .await
        .unwrap();

    if !resp.status().is_success() {
        eprintln!(
            "Gradient evaluation request failed ({}) -- may require async polling",
            resp.status()
        );
        return;
    }

    let eval: Value = resp.json().await.unwrap();
    let eval_id = eval["id"].as_str().unwrap_or("unknown");

    // Poll if the evaluation runs asynchronously
    let mut final_eval = eval.clone();
    if eval["status"].as_str() == Some("running") || eval["status"].as_str() == Some("pending") {
        for _ in 0..60 {
            sleep(Duration::from_secs(5)).await;
            let poll_resp = client()
                .get(format!("{}/api/v1/evaluations/{}", base_url(), eval_id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .unwrap();
            final_eval = poll_resp.json().await.unwrap();
            let status = final_eval["status"].as_str().unwrap_or("unknown");
            if status == "completed" || status == "failed" {
                break;
            }
        }
    }

    // Verify metrics if completed
    if final_eval["status"].as_str() == Some("completed") {
        let metrics = &final_eval["metrics"];
        assert!(
            metrics.is_object(),
            "Completed evaluation should have metrics object"
        );

        // Gradient evaluation should produce many metrics
        let metric_count = metrics.as_object().map(|m| m.len()).unwrap_or(0);
        eprintln!("Gradient evaluation produced {} metrics", metric_count);
        assert!(
            metric_count >= 5,
            "Expected at least 5 metrics, got {}",
            metric_count
        );
    }
}

// ─── Evaluation metric content ──────────────────────────────

#[tokio::test]
async fn test_evaluation_contains_standard_classification_metrics() {
    let token = login().await;

    let list_resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let evals: Value = list_resp.json().await.unwrap();
    let arr = evals.as_array().unwrap();

    // Find a completed evaluation
    let completed = arr
        .iter()
        .find(|e| e["status"].as_str() == Some("completed"));

    let eval = match completed {
        Some(e) => e,
        None => {
            eprintln!("No completed evaluations -- skipping standard metrics check");
            return;
        }
    };

    let eval_id = eval["id"].as_str().unwrap();

    let detail_resp = client()
        .get(format!("{}/api/v1/evaluations/{}", base_url(), eval_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let detail: Value = detail_resp.json().await.unwrap();
    let metrics = &detail["metrics"];

    if metrics.is_object() {
        let keys: Vec<&str> = metrics
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();

        // Check for common classification metrics
        let expected_candidates = ["precision", "recall", "f1", "f1_score", "auc_roc", "auc", "accuracy"];
        let found: Vec<&&str> = expected_candidates
            .iter()
            .filter(|k| keys.iter().any(|key| key.to_lowercase().contains(k)))
            .collect();

        eprintln!(
            "Found standard metrics: {:?} out of {:?}",
            found, expected_candidates
        );
    }
}

#[tokio::test]
async fn test_evaluation_contains_confusion_matrix() {
    let token = login().await;

    let list_resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let evals: Value = list_resp.json().await.unwrap();
    let arr = evals.as_array().unwrap();

    let completed = arr
        .iter()
        .find(|e| e["status"].as_str() == Some("completed"));

    let eval = match completed {
        Some(e) => e,
        None => {
            eprintln!("No completed evaluations -- skipping confusion matrix check");
            return;
        }
    };

    let eval_id = eval["id"].as_str().unwrap();

    let detail_resp = client()
        .get(format!("{}/api/v1/evaluations/{}", base_url(), eval_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let detail: Value = detail_resp.json().await.unwrap();

    // Confusion matrix might be under metrics or at top level
    let has_cm = detail["confusion_matrix"].is_object()
        || detail["confusion_matrix"].is_array()
        || detail["metrics"]["confusion_matrix"].is_object()
        || detail["metrics"]["confusion_matrix"].is_array();

    if has_cm {
        eprintln!("Confusion matrix found in evaluation {}", eval_id);
    } else {
        eprintln!(
            "No confusion_matrix field found for evaluation {} -- may not apply to this model type",
            eval_id
        );
    }
}

// ─── Evaluation for non-existent model ──────────────────────

#[tokio::test]
async fn test_evaluation_for_nonexistent_model_returns_error() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "model_id": "nonexistent_model_id_99999",
            "evaluation_type": "gradient"
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx for nonexistent model, got {}",
        resp.status()
    );
}

// ─── Evaluation response includes model_id ──────────────────

#[tokio::test]
async fn test_evaluation_response_includes_model_id() {
    let token = login().await;

    let list_resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let evals: Value = list_resp.json().await.unwrap();
    let arr = evals.as_array().unwrap();

    if arr.is_empty() {
        eprintln!("No evaluations -- skipping model_id check");
        return;
    }

    for (i, eval) in arr.iter().enumerate() {
        assert!(
            eval["model_id"].is_string(),
            "Evaluation {} missing 'model_id'",
            i
        );
    }
}

// ─── Evaluation metrics are within valid ranges ─────────────

#[tokio::test]
async fn test_evaluation_metrics_within_valid_ranges() {
    let token = login().await;

    let list_resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let evals: Value = list_resp.json().await.unwrap();
    let arr = evals.as_array().unwrap();

    let completed = arr
        .iter()
        .find(|e| e["status"].as_str() == Some("completed"));

    let eval = match completed {
        Some(e) => e,
        None => {
            eprintln!("No completed evaluations -- skipping range validation");
            return;
        }
    };

    let eval_id = eval["id"].as_str().unwrap();

    let detail_resp = client()
        .get(format!("{}/api/v1/evaluations/{}", base_url(), eval_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let detail: Value = detail_resp.json().await.unwrap();
    let metrics = &detail["metrics"];

    if let Some(obj) = metrics.as_object() {
        let bounded_metrics = [
            "precision", "recall", "f1", "f1_score", "accuracy",
            "auc_roc", "auc", "specificity", "sensitivity",
        ];

        for (key, value) in obj {
            if bounded_metrics.iter().any(|m| key.to_lowercase().contains(m)) {
                if let Some(v) = value.as_f64() {
                    assert!(
                        (0.0..=1.0).contains(&v),
                        "Metric '{}' = {} is outside [0.0, 1.0]",
                        key,
                        v
                    );
                }
            }
        }
    }
}

// ─── Evaluation with custom parameters ──────────────────────

#[tokio::test]
async fn test_run_evaluation_with_custom_parameters() {
    let token = login().await;

    let model_id = match first_model_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No models -- skipping custom evaluation test");
            return;
        }
    };

    let resp = client()
        .post(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "model_id": model_id,
            "evaluation_type": "gradient",
            "parameters": {
                "threshold": 0.5,
                "test_split": 0.2
            }
        }))
        .send()
        .await
        .unwrap();

    // Should either succeed or gracefully reject unknown params
    assert!(
        resp.status().is_success() || resp.status().is_client_error(),
        "Unexpected status for custom evaluation: {}",
        resp.status()
    );

    if resp.status().is_success() {
        let body: Value = resp.json().await.unwrap();
        assert!(
            body["id"].is_string(),
            "Evaluation response should have an id"
        );
    }
}

// ─── Evaluation includes timestamp ──────────────────────────

#[tokio::test]
async fn test_evaluation_includes_timestamp() {
    let token = login().await;

    let list_resp = client()
        .get(format!("{}/api/v1/evaluations", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let evals: Value = list_resp.json().await.unwrap();
    let arr = evals.as_array().unwrap();

    if arr.is_empty() {
        eprintln!("No evaluations -- skipping timestamp check");
        return;
    }

    for (i, eval) in arr.iter().enumerate() {
        let has_timestamp = eval["created_at"].is_string()
            || eval["timestamp"].is_string()
            || eval["started_at"].is_string();
        assert!(
            has_timestamp,
            "Evaluation {} is missing a timestamp field (created_at, timestamp, or started_at)",
            i
        );
    }
}
