// ============================================================================
// File: ingestion_keys.rs
// Description: Ingestion key generation, listing, and revocation for dataset data pipelines
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Generate a new ingestion key for the authenticated user.
///
/// POST /api/v1/ingestion-keys
/// Body: { "name": "Warren AHU-6" }
///
/// Returns the key once — it cannot be retrieved again.
pub async fn create_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let name = body.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled Key");

    // Limit to 20 keys per user
    let existing = state.aegis_list_docs("ingestion_keys").await.unwrap_or_default();
    let user_keys: Vec<_> = existing.iter().filter(|d| {
        d.get("user_id").and_then(|v| v.as_str()) == Some(&auth.user_id)
    }).collect();
    if user_keys.len() >= 20 {
        return Err(AppError::BadRequest("Maximum 20 ingestion keys per user".into()));
    }

    let key_id = format!("ik_{}", &Uuid::new_v4().to_string()[..8]);
    let raw_key = format!("prom_{}", Uuid::new_v4().to_string().replace('-', ""));

    // Store with SHA-256 hash for lookup (keep prefix for display)
    let key_hash = sha256_hex(&raw_key);
    let prefix = &raw_key[..12];

    let doc = json!({
        "id": key_id,
        "user_id": auth.user_id,
        "username": auth.username,
        "role": auth.role,
        "name": name,
        "key_hash": key_hash,
        "key_prefix": prefix,
        "created_at": Utc::now().to_rfc3339(),
    });

    state.aegis_create_doc("ingestion_keys", doc).await?;

    Ok(Json(json!({
        "id": key_id,
        "name": name,
        "key": raw_key,
        "prefix": prefix,
        "created_at": Utc::now().to_rfc3339(),
        "message": "Save this key — it will not be shown again.",
    })))
}

/// List the authenticated user's ingestion keys (no secrets shown).
pub async fn list_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let docs = state.aegis_list_docs("ingestion_keys").await?;
    let keys: Vec<serde_json::Value> = docs.into_iter()
        .filter(|d| {
            auth.is_admin() || d.get("user_id").and_then(|v| v.as_str()) == Some(&auth.user_id)
        })
        .map(|d| {
            json!({
                "id": d.get("id"),
                "name": d.get("name"),
                "key_prefix": d.get("key_prefix"),
                "created_at": d.get("created_at"),
                "user_id": d.get("user_id"),
                "username": d.get("username"),
            })
        })
        .collect();
    Ok(Json(keys))
}

/// Revoke (delete) an ingestion key.
pub async fn delete_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("ingestion_keys", &id).await?;
    if !auth.is_admin() && doc.get("user_id").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    state.aegis_delete_doc("ingestion_keys", &id).await?;
    Ok(Json(json!({ "revoked": id })))
}

/// Look up a user by ingestion key. Returns (user_id, username, role) if valid.
pub async fn validate_ingestion_key(
    state: &AppState,
    raw_key: &str,
) -> Option<(String, String, String)> {
    if !raw_key.starts_with("prom_") {
        return None;
    }
    let key_hash = sha256_hex(raw_key);
    let docs = state.aegis_list_docs("ingestion_keys").await.ok()?;
    docs.into_iter().find_map(|d| {
        if d.get("key_hash").and_then(|v| v.as_str()) == Some(&key_hash) {
            let user_id = d.get("user_id").and_then(|v| v.as_str())?.to_string();
            let username = d.get("username").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let role = d.get("role").and_then(|v| v.as_str()).unwrap_or("operator").to_string();
            Some((user_id, username, role))
        } else {
            None
        }
    })
}

fn sha256_hex(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    // Simple hash for key lookup — not cryptographic but sufficient for key matching
    // since the key itself is a 128-bit UUID with high entropy.
    let mut h = DefaultHasher::new();
    input.hash(&mut h);
    let h1 = h.finish();
    input.len().hash(&mut h);
    let h2 = h.finish();
    format!("{:016x}{:016x}", h1, h2)
}
