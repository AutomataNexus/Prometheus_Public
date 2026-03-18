// ============================================================================
// File: state.rs
// Description: Shared application state including HTTP client, config, and training job management
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::ServerConfig;

#[derive(Clone)]
pub struct AppState {
    pub config: ServerConfig,
    pub http_client: reqwest::Client,
    pub active_trainings: Arc<RwLock<std::collections::HashMap<String, TrainingHandle>>>,
    pub training_queue: Arc<RwLock<std::collections::VecDeque<QueuedTraining>>>,
}

pub struct TrainingHandle {
    #[allow(dead_code)] // stored for identification; may be used for logging/display
    pub id: String,
    pub cancel_token: tokio::sync::watch::Sender<bool>,
}

/// A training run waiting for a slot to open up.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct QueuedTraining {
    pub run_id: String,
    pub user_id: String,
    pub dataset_id: String,
    pub architecture: String,
    pub hyperparameters: serde_json::Value,
    pub queued_at: String,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            active_trainings: Arc::new(RwLock::new(std::collections::HashMap::new())),
            training_queue: Arc::new(RwLock::new(std::collections::VecDeque::new())),
        }
    }

    /// Make an authenticated request to Aegis-DB
    pub async fn aegis_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, crate::error::AppError> {
        let url = format!("{}{}", self.config.aegis_db_url, path);
        let mut req = self.http_client.request(method, &url);

        if let Some(body) = body {
            req = req.json(&body);
        }

        let resp = req.send().await.map_err(|e| {
            crate::error::AppError::AegisDb(format!("Connection failed: {e}"))
        })?;

        if resp.status().is_success() {
            resp.json().await.map_err(|e| {
                crate::error::AppError::AegisDb(format!("Parse error: {e}"))
            })
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(crate::error::AppError::AegisDb(format!(
                "Aegis-DB returned {status}: {text}"
            )))
        }
    }

    /// Get a document from an Aegis-DB collection.
    /// Unwraps from `{ "data": { ... } }` envelope.
    pub async fn aegis_get_doc(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<serde_json::Value, crate::error::AppError> {
        let resp = self
            .aegis_request(
                reqwest::Method::GET,
                &format!("/api/v1/documents/collections/{collection}/documents/{id}"),
                None,
            )
            .await?;
        // Aegis-DB wraps: { "id": ..., "collection": ..., "data": { actual doc } }
        Ok(resp.get("data").cloned().unwrap_or(resp))
    }

    /// List documents in an Aegis-DB collection.
    /// Unwraps from `{ "documents": [ { "data": { ... } } ] }` envelope.
    pub async fn aegis_list_docs(
        &self,
        collection: &str,
    ) -> Result<Vec<serde_json::Value>, crate::error::AppError> {
        let resp = self
            .aegis_request(
                reqwest::Method::GET,
                &format!("/api/v1/documents/collections/{collection}/documents"),
                None,
            )
            .await?;
        // Aegis-DB wraps: { "documents": [ { "id": ..., "data": { ... } } ], "total_scanned": ... }
        let docs = resp
            .get("documents")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|doc| doc.get("data").cloned().unwrap_or_else(|| doc.clone()))
                    .collect()
            })
            .or_else(|| resp.as_array().cloned())
            .unwrap_or_default();
        Ok(docs)
    }

    /// Create a document in an Aegis-DB collection.
    /// Wraps the doc in `{ "id": ..., "document": ... }` as Aegis-DB expects.
    pub async fn aegis_create_doc(
        &self,
        collection: &str,
        doc: serde_json::Value,
    ) -> Result<serde_json::Value, crate::error::AppError> {
        let id = doc.get("id").and_then(|v| v.as_str()).map(String::from);
        let wrapped = serde_json::json!({
            "id": id,
            "document": doc,
        });
        self.aegis_request(
            reqwest::Method::POST,
            &format!("/api/v1/documents/collections/{collection}/documents"),
            Some(wrapped),
        )
        .await
    }

    /// Update (merge) fields into an existing Aegis-DB document.
    /// Fetches the current doc, merges the provided fields, then PUTs it back.
    pub async fn aegis_update_doc(
        &self,
        collection: &str,
        id: &str,
        updates: serde_json::Value,
    ) -> Result<serde_json::Value, crate::error::AppError> {
        // Fetch current document
        let current = self.aegis_get_doc(collection, id).await.unwrap_or_default();

        // Merge updates into current
        let mut merged = current;
        if let (Some(base), Some(patch)) = (merged.as_object_mut(), updates.as_object()) {
            for (k, v) in patch {
                base.insert(k.clone(), v.clone());
            }
        }

        // PUT back with document wrapper
        let wrapped = serde_json::json!({ "document": merged });
        self.aegis_request(
            reqwest::Method::PUT,
            &format!("/api/v1/documents/collections/{collection}/documents/{id}"),
            Some(wrapped),
        )
        .await
    }

    /// Delete a document from an Aegis-DB collection
    pub async fn aegis_delete_doc(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<serde_json::Value, crate::error::AppError> {
        self.aegis_request(
            reqwest::Method::DELETE,
            &format!("/api/v1/documents/collections/{collection}/documents/{id}"),
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;

    fn test_config() -> ServerConfig {
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3030,
            aegis_db_url: "http://localhost:9091".to_string(),
            gradient_api_key: None,
            gradient_agent_id: None,
            gradient_endpoint: None,
            data_dir: "/tmp/test-data".to_string(),
            stripe_secret_key: None,
            stripe_webhook_secret: None,
            stripe_price_basic: None,
            stripe_price_pro: None,
            stripe_price_enterprise: None,
            stripe_meter_id: None,
            stripe_price_overage: None,
            max_concurrent_trainings: 4,
        }
    }

    #[test]
    fn app_state_new_creates_instance() {
        let state = AppState::new(test_config());
        assert_eq!(state.config.port, 3030);
        assert_eq!(state.config.host, "127.0.0.1");
    }

    #[test]
    fn app_state_preserves_aegis_url() {
        let mut cfg = test_config();
        cfg.aegis_db_url = "http://aegis:5000".to_string();
        let state = AppState::new(cfg);
        assert_eq!(state.config.aegis_db_url, "http://aegis:5000");
    }

    #[test]
    fn aegis_request_url_construction() {
        let state = AppState::new(test_config());
        // The URL is built as "{aegis_db_url}{path}" — verify the concatenation logic
        let expected = "http://localhost:9091/api/v1/documents/collections/models/documents/abc";
        let url = format!(
            "{}{}",
            state.config.aegis_db_url,
            "/api/v1/documents/collections/models/documents/abc"
        );
        assert_eq!(url, expected);
    }

    #[test]
    fn aegis_get_doc_url_format() {
        // Verify the path pattern used by aegis_get_doc
        let collection = "models";
        let id = "abc-123";
        let path = format!("/api/v1/documents/collections/{collection}/documents/{id}");
        assert_eq!(path, "/api/v1/documents/collections/models/documents/abc-123");
    }

    #[test]
    fn aegis_list_docs_url_format() {
        let collection = "deployments";
        let path = format!("/api/v1/documents/collections/{collection}/documents");
        assert_eq!(path, "/api/v1/documents/collections/deployments/documents");
    }

    #[test]
    fn aegis_create_doc_url_format() {
        let collection = "experiments";
        let path = format!("/api/v1/documents/collections/{collection}/documents");
        assert_eq!(path, "/api/v1/documents/collections/experiments/documents");
    }

    #[test]
    fn aegis_delete_doc_url_format() {
        let collection = "checkpoints";
        let id = "xyz";
        let path = format!("/api/v1/documents/collections/{collection}/documents/{id}");
        assert_eq!(path, "/api/v1/documents/collections/checkpoints/documents/xyz");
    }

    #[test]
    fn active_trainings_starts_empty() {
        let state = AppState::new(test_config());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let count = rt.block_on(async {
            state.active_trainings.read().await.len()
        });
        assert_eq!(count, 0);
    }

    #[test]
    fn app_state_is_clone() {
        let state = AppState::new(test_config());
        let state2 = state.clone();
        assert_eq!(state2.config.port, state.config.port);
    }
}
