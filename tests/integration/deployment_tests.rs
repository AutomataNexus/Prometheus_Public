//! Prometheus Deployment API Integration Tests
//!
//! Tests the deployment lifecycle: listing deployments, listing edge targets,
//! creating deployments, downloading binaries, status transitions, and error
//! handling for the deployment pipeline.
//!
//! # Requirements
//! - Prometheus server running on `PROMETHEUS_URL` (default: `http://localhost:3030`)
//! - Aegis-DB running on port 9091
//!
//! # Running
//! ```bash
//! cargo test --test deployment_tests -- --test-threads=1
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

/// Retrieve the first available model ID, or None if no models exist.
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

/// Retrieve the first available edge target ID, or None if none exist.
async fn first_target_id(token: &str) -> Option<String> {
    let resp = client()
        .get(format!("{}/api/v1/deployments/targets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let targets: Value = resp.json().await.unwrap();
    targets
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|t| t["id"].as_str())
        .map(|s| s.to_string())
}

// ─── List deployments ────────────────────────────────────────

#[tokio::test]
async fn test_list_deployments_returns_array() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array(), "Expected array, got: {:?}", body);
}

#[tokio::test]
async fn test_list_deployments_without_auth_returns_401() {
    let resp = client()
        .get(format!("{}/api/v1/deployments", base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ─── List edge targets ──────────────────────────────────────

#[tokio::test]
async fn test_list_edge_targets_returns_array() {
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

#[tokio::test]
async fn test_edge_targets_have_expected_fields() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/deployments/targets", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let targets: Value = resp.json().await.unwrap();
    let arr = targets.as_array().unwrap();

    if arr.is_empty() {
        eprintln!("No edge targets available -- skipping field validation");
        return;
    }

    let target = &arr[0];
    assert!(target["id"].is_string(), "Target missing 'id'");
    assert!(target["name"].is_string(), "Target missing 'name'");
    assert!(target["ip"].is_string(), "Target missing 'ip'");
    assert!(target["arch"].is_string(), "Target missing 'arch'");
    assert!(target["status"].is_string(), "Target missing 'status'");
}

// ─── Create deployment ──────────────────────────────────────

#[tokio::test]
async fn test_create_deployment_with_valid_model_and_target() {
    let token = login().await;

    let model_id = match first_model_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No models available -- skipping deployment creation test");
            return;
        }
    };

    let target_id = match first_target_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No edge targets available -- skipping deployment creation test");
            return;
        }
    };

    let resp = client()
        .post(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "model_id": model_id,
            "target_id": target_id,
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Create deployment failed: {}",
        resp.status()
    );

    let deployment: Value = resp.json().await.unwrap();
    assert!(deployment["id"].is_string(), "Deployment missing 'id'");
    assert_eq!(deployment["model_id"], model_id);
    assert_eq!(deployment["target_id"], target_id);
    assert!(deployment["status"].is_string(), "Deployment missing 'status'");
}

#[tokio::test]
async fn test_create_deployment_with_invalid_model_returns_error() {
    let token = login().await;

    let target_id = match first_target_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No edge targets -- skipping");
            return;
        }
    };

    let resp = client()
        .post(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "model_id": "nonexistent_model_id_99999",
            "target_id": target_id,
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx for invalid model_id, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_create_deployment_with_missing_fields_returns_400() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "Expected 4xx for empty body, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_create_deployment_without_auth_returns_401() {
    let resp = client()
        .post(format!("{}/api/v1/deployments", base_url()))
        .json(&json!({
            "model_id": "some_model_id",
            "target_id": "some_target_id",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ─── Get deployment detail ──────────────────────────────────

#[tokio::test]
async fn test_get_deployment_detail_returns_all_fields() {
    let token = login().await;

    // List existing deployments
    let list_resp = client()
        .get(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let deployments: Value = list_resp.json().await.unwrap();
    let arr = deployments.as_array().unwrap();

    if arr.is_empty() {
        eprintln!("No deployments found -- skipping detail test");
        return;
    }

    let deployment_id = arr[0]["id"].as_str().unwrap();

    let detail_resp = client()
        .get(format!(
            "{}/api/v1/deployments/{}",
            base_url(),
            deployment_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(detail_resp.status().is_success());
    let detail: Value = detail_resp.json().await.unwrap();

    assert_eq!(detail["id"], deployment_id);
    assert!(detail["model_id"].is_string(), "Missing model_id");
    assert!(detail["target_id"].is_string(), "Missing target_id");
    assert!(detail["status"].is_string(), "Missing status");
    assert!(
        detail["created_at"].is_string(),
        "Missing created_at timestamp"
    );
}

#[tokio::test]
async fn test_get_deployment_detail_for_nonexistent_id_returns_404() {
    let token = login().await;

    let resp = client()
        .get(format!(
            "{}/api/v1/deployments/nonexistent_deploy_id_99999",
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

// ─── Deployment binary download ─────────────────────────────

#[tokio::test]
async fn test_download_deployment_binary_returns_content() {
    let token = login().await;

    let list_resp = client()
        .get(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let deployments: Value = list_resp.json().await.unwrap();
    let arr = deployments.as_array().unwrap();

    if arr.is_empty() {
        eprintln!("No deployments available -- skipping binary download test");
        return;
    }

    let deployment_id = arr[0]["id"].as_str().unwrap();

    let dl_resp = client()
        .get(format!(
            "{}/api/v1/deployments/{}/download",
            base_url(),
            deployment_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    // Binary download might return 200 or 404 if not yet built
    if dl_resp.status().is_success() {
        let content_type = dl_resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("octet-stream")
                || content_type.contains("binary")
                || content_type.contains("application"),
            "Expected binary content-type, got: {}",
            content_type
        );

        let bytes = dl_resp.bytes().await.unwrap();
        assert!(!bytes.is_empty(), "Downloaded binary is empty");
    } else {
        eprintln!(
            "Binary not available for deployment {} (status {})",
            deployment_id,
            dl_resp.status()
        );
    }
}

#[tokio::test]
async fn test_deployment_binary_has_correct_content_type() {
    let token = login().await;

    let list_resp = client()
        .get(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let deployments: Value = list_resp.json().await.unwrap();
    let arr = deployments.as_array().unwrap();

    if arr.is_empty() {
        eprintln!("No deployments -- skipping content-type test");
        return;
    }

    // Find a deployment with status "deployed" if possible
    let deployed = arr
        .iter()
        .find(|d| d["status"].as_str() == Some("deployed"))
        .or_else(|| arr.first());

    let deployment_id = deployed.unwrap()["id"].as_str().unwrap();

    let resp = client()
        .get(format!(
            "{}/api/v1/deployments/{}/download",
            base_url(),
            deployment_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    if resp.status().is_success() {
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("octet-stream") || content_type.contains("binary"),
            "Expected octet-stream or binary content-type, got: {}",
            content_type
        );
    }
}

// ─── Deployment status transitions ──────────────────────────

#[tokio::test]
async fn test_deployment_status_transitions() {
    let token = login().await;

    let model_id = match first_model_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No models -- skipping status transition test");
            return;
        }
    };
    let target_id = match first_target_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No targets -- skipping status transition test");
            return;
        }
    };

    // Create deployment
    let create_resp = client()
        .post(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "model_id": model_id,
            "target_id": target_id,
        }))
        .send()
        .await
        .unwrap();

    if !create_resp.status().is_success() {
        eprintln!("Could not create deployment -- skipping status transition test");
        return;
    }

    let deployment: Value = create_resp.json().await.unwrap();
    let deploy_id = deployment["id"].as_str().unwrap();

    // Initial status should be pending or deploying
    let initial_status = deployment["status"].as_str().unwrap();
    assert!(
        ["pending", "deploying", "queued"].contains(&initial_status),
        "Expected initial status pending/deploying/queued, got: {}",
        initial_status
    );

    // Poll for status changes
    let valid_statuses = [
        "pending", "queued", "deploying", "deployed", "failed", "cancelled",
    ];
    let mut seen_statuses: Vec<String> = vec![initial_status.to_string()];

    for _ in 0..30 {
        sleep(Duration::from_secs(5)).await;

        let status_resp = client()
            .get(format!(
                "{}/api/v1/deployments/{}",
                base_url(),
                deploy_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        let status_body: Value = status_resp.json().await.unwrap();
        let current_status = status_body["status"].as_str().unwrap_or("unknown").to_string();

        assert!(
            valid_statuses.contains(&current_status.as_str()),
            "Invalid deployment status: {}",
            current_status
        );

        if !seen_statuses.contains(&current_status) {
            seen_statuses.push(current_status.clone());
        }

        if current_status == "deployed" || current_status == "failed" {
            break;
        }
    }

    eprintln!("Observed status transitions: {:?}", seen_statuses);
}

// ─── Multiple deployments to same target ────────────────────

#[tokio::test]
async fn test_multiple_deployments_to_same_target() {
    let token = login().await;

    let models_resp = client()
        .get(format!("{}/api/v1/models", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let models: Value = models_resp.json().await.unwrap();
    let models_arr = models.as_array().unwrap();

    if models_arr.len() < 2 {
        eprintln!("Need at least 2 models -- skipping multi-deployment test");
        return;
    }

    let target_id = match first_target_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No targets -- skipping");
            return;
        }
    };

    let model_a = models_arr[0]["id"].as_str().unwrap();
    let model_b = models_arr[1]["id"].as_str().unwrap();

    let resp_a = client()
        .post(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "model_id": model_a, "target_id": target_id }))
        .send()
        .await
        .unwrap();

    let resp_b = client()
        .post(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "model_id": model_b, "target_id": target_id }))
        .send()
        .await
        .unwrap();

    // Both should succeed or the second may be rejected -- either is valid behavior
    assert!(
        resp_a.status().is_success() || resp_a.status().is_client_error(),
        "Unexpected status for deployment A: {}",
        resp_a.status()
    );
    assert!(
        resp_b.status().is_success() || resp_b.status().is_client_error(),
        "Unexpected status for deployment B: {}",
        resp_b.status()
    );
}

// ─── List deployments after creation ────────────────────────

#[tokio::test]
async fn test_list_deployments_after_creating_one() {
    let token = login().await;

    let model_id = match first_model_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No models -- skipping");
            return;
        }
    };
    let target_id = match first_target_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No targets -- skipping");
            return;
        }
    };

    // Count before
    let before_resp = client()
        .get(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    let before: Value = before_resp.json().await.unwrap();
    let count_before = before.as_array().unwrap().len();

    // Create deployment
    let create_resp = client()
        .post(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "model_id": model_id, "target_id": target_id }))
        .send()
        .await
        .unwrap();

    if !create_resp.status().is_success() {
        eprintln!("Create deployment failed -- skipping");
        return;
    }

    // Count after
    let after_resp = client()
        .get(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    let after: Value = after_resp.json().await.unwrap();
    let count_after = after.as_array().unwrap().len();

    assert!(
        count_after > count_before,
        "Expected deployments list to grow after creation ({} -> {})",
        count_before,
        count_after
    );
}

// ─── Deployment includes expected fields ────────────────────

#[tokio::test]
async fn test_deployment_includes_model_target_status_created_at() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let deployments: Value = resp.json().await.unwrap();
    let arr = deployments.as_array().unwrap();

    if arr.is_empty() {
        eprintln!("No deployments -- skipping field check");
        return;
    }

    for (i, deployment) in arr.iter().enumerate() {
        assert!(
            deployment["id"].is_string(),
            "Deployment {} missing 'id'",
            i
        );
        assert!(
            deployment["model_id"].is_string(),
            "Deployment {} missing 'model_id'",
            i
        );
        assert!(
            deployment["target_id"].is_string(),
            "Deployment {} missing 'target_id'",
            i
        );
        assert!(
            deployment["status"].is_string(),
            "Deployment {} missing 'status'",
            i
        );
        assert!(
            deployment["created_at"].is_string(),
            "Deployment {} missing 'created_at'",
            i
        );
    }
}

// ─── Cancel / delete deployment ─────────────────────────────

#[tokio::test]
async fn test_cancel_or_delete_deployment() {
    let token = login().await;

    let model_id = match first_model_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No models -- skipping cancel test");
            return;
        }
    };
    let target_id = match first_target_id(&token).await {
        Some(id) => id,
        None => {
            eprintln!("No targets -- skipping cancel test");
            return;
        }
    };

    // Create a deployment to cancel
    let create_resp = client()
        .post(format!("{}/api/v1/deployments", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "model_id": model_id, "target_id": target_id }))
        .send()
        .await
        .unwrap();

    if !create_resp.status().is_success() {
        eprintln!("Could not create deployment -- skipping cancel test");
        return;
    }

    let deployment: Value = create_resp.json().await.unwrap();
    let deploy_id = deployment["id"].as_str().unwrap();

    // Try DELETE
    let delete_resp = client()
        .delete(format!(
            "{}/api/v1/deployments/{}",
            base_url(),
            deploy_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(
        delete_resp.status().is_success() || delete_resp.status() == 404,
        "Expected success or 404, got {}",
        delete_resp.status()
    );

    // Verify it is gone or cancelled
    let get_resp = client()
        .get(format!(
            "{}/api/v1/deployments/{}",
            base_url(),
            deploy_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let status = get_resp.status().as_u16();
    if status == 200 {
        let body: Value = get_resp.json().await.unwrap();
        assert!(
            ["cancelled", "deleted"].contains(&body["status"].as_str().unwrap_or("")),
            "Expected cancelled/deleted status, got: {}",
            body["status"]
        );
    } else {
        assert_eq!(status, 404, "Expected 404 after deletion, got {}", status);
    }
}
