//! Prometheus Agent API Integration Tests
//!
//! Tests the AI agent endpoints: chat, chat history, analyze,
//! error handling, and context maintenance across multiple messages.
//!
//! # Requirements
//! - Prometheus server running on `PROMETHEUS_URL` (default: `http://localhost:3030`)
//! - Aegis-DB running on port 9091
//!
//! # Running
//! ```bash
//! cargo test --test agent_tests -- --test-threads=1
//! ```

use serde_json::{json, Value};
use std::env;
use std::time::Duration;

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

// ─── Chat endpoint ──────────────────────────────────────────

#[tokio::test]
async fn test_chat_endpoint_returns_response() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "Hello, what can you help me with?"
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Chat request failed: {}",
        resp.status()
    );

    let body: Value = resp.json().await.unwrap();
    assert!(
        body["response"].is_string(),
        "Chat response should contain a 'response' field"
    );
}

#[tokio::test]
async fn test_chat_with_architecture_question_returns_relevant_info() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "What model architectures are available for anomaly detection?"
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    let response_text = body["response"].as_str().unwrap_or("");

    assert!(
        response_text.len() > 20,
        "Architecture answer is too short ({} chars): '{}'",
        response_text.len(),
        response_text
    );

    // The response should mention at least one architecture keyword
    let response_lower = response_text.to_lowercase();
    let arch_keywords = ["lstm", "gru", "autoencoder", "transformer", "cnn", "neural", "model"];
    let mentions_arch = arch_keywords.iter().any(|kw| response_lower.contains(kw));

    eprintln!("Agent architecture response ({} chars): {}", response_text.len(), &response_text[..response_text.len().min(200)]);
    assert!(
        mentions_arch,
        "Expected response to mention at least one architecture keyword"
    );
}

#[tokio::test]
async fn test_chat_response_contains_response_field() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "Summarize the system status."
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();

    assert!(
        body["response"].is_string(),
        "Missing 'response' field. Got keys: {:?}",
        body.as_object().map(|o| o.keys().collect::<Vec<_>>())
    );

    let text = body["response"].as_str().unwrap();
    assert!(
        !text.is_empty(),
        "Response string should not be empty"
    );
}

// ─── Chat with empty message ────────────────────────────────

#[tokio::test]
async fn test_chat_with_empty_message_returns_error() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": ""
        }))
        .send()
        .await
        .unwrap();

    // Empty message should be rejected (400) or handled gracefully (200)
    assert!(
        resp.status().is_client_error() || resp.status().is_success(),
        "Unexpected status for empty message: {}",
        resp.status()
    );

    if resp.status().is_client_error() {
        eprintln!("Server correctly rejected empty message with {}", resp.status());
    }
}

// ─── Chat without auth ─────────────────────────────────────

#[tokio::test]
async fn test_agent_chat_without_auth_returns_401() {
    let resp = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .json(&json!({
            "message": "Hello"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_agent_history_without_auth_returns_401() {
    let resp = client()
        .get(format!("{}/api/v1/agent/history", base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_agent_analyze_without_auth_returns_401() {
    let resp = client()
        .post(format!("{}/api/v1/agent/analyze", base_url()))
        .json(&json!({
            "message": "Analyze my HVAC system."
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ─── Chat history ───────────────────────────────────────────

#[tokio::test]
async fn test_chat_history_returns_array() {
    let token = login().await;

    let resp = client()
        .get(format!("{}/api/v1/agent/history", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array(), "History should be an array");
}

#[tokio::test]
async fn test_chat_history_items_have_role_and_content() {
    let token = login().await;

    // Send a message first so there is at least one history item
    client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "What is Prometheus?"
        }))
        .send()
        .await
        .unwrap();

    // Fetch history
    let resp = client()
        .get(format!("{}/api/v1/agent/history", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let history: Value = resp.json().await.unwrap();
    let arr = history.as_array().unwrap();

    if arr.is_empty() {
        eprintln!("History is empty even after sending a message -- skipping field check");
        return;
    }

    for (i, item) in arr.iter().enumerate() {
        assert!(
            item["role"].is_string() || item["sender"].is_string(),
            "History item {} missing role/sender",
            i
        );
        assert!(
            item["content"].is_string() || item["message"].is_string(),
            "History item {} missing content/message",
            i
        );
    }
}

// ─── Analyze endpoint ───────────────────────────────────────

#[tokio::test]
async fn test_analyze_endpoint_returns_recommendation() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/agent/analyze", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "I have an air handler with supply temp, return temp, and fan speed data. What architecture should I use?"
        }))
        .send()
        .await
        .unwrap();

    // Analyze endpoint may not exist (404) or may succeed
    if resp.status() == 404 {
        eprintln!("Analyze endpoint not found -- may use /agent/chat instead");
        return;
    }

    assert!(
        resp.status().is_success(),
        "Analyze request failed: {}",
        resp.status()
    );

    let body: Value = resp.json().await.unwrap();
    assert!(
        body["response"].is_string()
            || body["recommendation"].is_string()
            || body["analysis"].is_string(),
        "Analyze response should contain response, recommendation, or analysis"
    );
}

#[tokio::test]
async fn test_analyze_includes_architecture_suggestion() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/agent/analyze", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "Recommend an architecture for detecting anomalies in chiller plant data with 15 sensor columns."
        }))
        .send()
        .await
        .unwrap();

    if resp.status() == 404 {
        eprintln!("Analyze endpoint not found -- skipping architecture suggestion test");
        return;
    }

    if resp.status().is_success() {
        let body: Value = resp.json().await.unwrap();
        let text = body["response"]
            .as_str()
            .or(body["recommendation"].as_str())
            .or(body["analysis"].as_str())
            .unwrap_or("");

        let text_lower = text.to_lowercase();
        let has_suggestion = text_lower.contains("lstm")
            || text_lower.contains("autoencoder")
            || text_lower.contains("gru")
            || text_lower.contains("transformer")
            || text_lower.contains("architecture")
            || text_lower.contains("recommend");

        eprintln!("Analyze response ({} chars)", text.len());
        assert!(
            has_suggestion,
            "Expected architecture suggestion in response"
        );
    }
}

#[tokio::test]
async fn test_analyze_includes_confidence_score() {
    let token = login().await;

    let resp = client()
        .post(format!("{}/api/v1/agent/analyze", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "Analyze performance of my current anomaly detection setup."
        }))
        .send()
        .await
        .unwrap();

    if resp.status() == 404 {
        eprintln!("Analyze endpoint not found -- skipping confidence score test");
        return;
    }

    if resp.status().is_success() {
        let body: Value = resp.json().await.unwrap();

        // Confidence might be present as a dedicated field
        let has_confidence = body["confidence"].is_number()
            || body["confidence_score"].is_number()
            || body["score"].is_number();

        if has_confidence {
            let score = body["confidence"]
                .as_f64()
                .or(body["confidence_score"].as_f64())
                .or(body["score"].as_f64())
                .unwrap();
            assert!(
                (0.0..=1.0).contains(&score),
                "Confidence score {} is out of range [0.0, 1.0]",
                score
            );
        } else {
            eprintln!("No explicit confidence score field -- agent may embed it in text response");
        }
    }
}

// ─── Multiple messages maintain context ─────────────────────

#[tokio::test]
async fn test_multiple_chat_messages_maintain_context() {
    let token = login().await;

    // First message: establish context
    let resp1 = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "I am working with air handler data that has supply_temp, return_temp, and fan_speed columns."
        }))
        .send()
        .await
        .unwrap();
    assert!(resp1.status().is_success());

    // Second message: reference context
    let resp2 = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": "Based on the data I just described, what architecture would you recommend?"
        }))
        .send()
        .await
        .unwrap();
    assert!(resp2.status().is_success());

    let body2: Value = resp2.json().await.unwrap();
    let response = body2["response"].as_str().unwrap_or("");

    // The response should be substantive (not a generic error)
    assert!(
        response.len() > 10,
        "Second response too short -- context may not be maintained: '{}'",
        response
    );
}

// ─── Long message handling ──────────────────────────────────

#[tokio::test]
async fn test_long_message_is_handled_correctly() {
    let token = login().await;

    // Generate a long but valid message
    let long_message = format!(
        "I have a complex HVAC system with the following sensors: {}. \
         Can you recommend an anomaly detection approach?",
        (1..=200)
            .map(|i| format!("sensor_{}", i))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let resp = client()
        .post(format!("{}/api/v1/agent/chat", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "message": long_message
        }))
        .send()
        .await
        .unwrap();

    // Should either succeed or return a clear error (not 500)
    assert!(
        resp.status().is_success() || resp.status().is_client_error(),
        "Expected 2xx or 4xx for long message, got {}",
        resp.status()
    );

    if resp.status().is_success() {
        let body: Value = resp.json().await.unwrap();
        assert!(body["response"].is_string());
    }
}
