// ============================================================================
// File: datasets.rs
// Description: Dataset CRUD, file upload/download, and ingestion key validation endpoints
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{
    extract::{Multipart, Path, Query, State},
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use crate::error::{AppError, AppResult};
use crate::auth::middleware::AuthUser;
use axum::Extension;
use std::sync::Arc;
use prometheus_shield::Shield;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// List / CRUD
// ---------------------------------------------------------------------------

pub async fn list_datasets(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let docs = state.aegis_list_docs("datasets").await?;
    let visible: Vec<serde_json::Value> = if auth.is_admin() {
        docs
    } else {
        docs.into_iter().filter(|d| {
            d.get("created_by").and_then(|v| v.as_str()) == Some(&auth.user_id)
        }).collect()
    };
    // Redact sensitive fields in source_config before sending to client
    let redacted = visible.into_iter().map(|mut d| {
        if let Some(sc) = d.get("source_config").cloned() {
            if let Some(obj) = d.as_object_mut() {
                obj.insert(
                    "source_config".to_string(),
                    prometheus_shield::credential_vault::redact_source_config(&sc),
                );
            }
        }
        d
    }).collect();
    Ok(Json(redacted))
}

pub async fn upload_dataset(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    mut multipart: Multipart,
) -> AppResult<Json<serde_json::Value>> {
    // Enforce dataset count limit (admins bypass)
    if !auth.is_admin() {
        crate::api::billing::enforce_limit(
            &state, &auth.user_id, "datasets", "created_by",
            |t| t.max_datasets(), "Dataset",
        ).await?;
    }
    let mut file_data = Vec::new();
    let mut filename = String::new();
    // Collect all non-file fields as open-ended metadata
    let mut metadata = serde_json::Map::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {e}")))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "file" {
            filename = field
                .file_name()
                .unwrap_or("upload.csv")
                .to_string();
            file_data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Read error: {e}")))?
                .to_vec();
        } else if !field_name.is_empty() {
            let value = field.text().await.unwrap_or_default();
            if !value.is_empty() {
                // Try parsing as JSON first (supports arrays, numbers, objects)
                let json_val = serde_json::from_str::<serde_json::Value>(&value)
                    .unwrap_or_else(|_| {
                        // Treat comma-separated values as arrays if field is "tags"
                        if field_name == "tags" && value.contains(',') {
                            json!(value.split(',').map(|t| t.trim()).collect::<Vec<_>>())
                        } else {
                            json!(value)
                        }
                    });
                metadata.insert(field_name, json_val);
            }
        }
    }

    if file_data.is_empty() {
        return Err(AppError::BadRequest("No file uploaded".into()));
    }

    // Enforce dataset size limit (admins bypass)
    if !auth.is_admin() {
        let tier = crate::api::billing::get_user_tier(&state, &auth.user_id).await;
        let max_size = tier.max_dataset_size_bytes();
        if max_size != u64::MAX && file_data.len() as u64 > max_size {
            return Err(AppError::Forbidden(format!(
                "Dataset too large ({:.1} MB). Your plan allows up to {:.0} MB.",
                file_data.len() as f64 / (1024.0 * 1024.0),
                max_size as f64 / (1024.0 * 1024.0),
            )));
        }
    }

    // Derive dataset name from metadata or filename
    let dataset_name = metadata.get("name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            let device = metadata.get("device_id").and_then(|v| v.as_str());
            let location = metadata.get("location").and_then(|v| v.as_str());
            match (device, location) {
                (Some(d), Some(l)) => Some(format!("{d} - {l}")),
                (Some(d), None) => Some(format!("{d} - {}", filename.replace(".csv", ""))),
                _ => None,
            }
        })
        .unwrap_or_else(|| filename.replace(".csv", ""));

    let csv_text = String::from_utf8_lossy(&file_data);

    // Check if a dataset with same name already exists — append instead of duplicating
    let existing = find_existing_dataset(&state, &dataset_name, &auth.user_id).await;

    if let Some(existing_doc) = existing {
        // Check if dataset is paused — reject ingestion
        let status = existing_doc.get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("active");
        if status == "paused" {
            return Err(AppError::BadRequest(format!(
                "Dataset '{}' is paused. Resume ingestion before sending more data.", dataset_name
            )));
        }

        // Append new CSV rows to the existing file (skip header if columns match)
        let existing_path = existing_doc.get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let existing_id = existing_doc.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !existing_path.is_empty() && !existing_id.is_empty() {
            // Decompress if needed before appending
            let actual_path = if existing_path.ends_with(".zst") {
                crate::api::data_lifecycle::decompress_dataset(&state, existing_id)
                    .await
                    .unwrap_or_else(|_| existing_path.to_string())
            } else {
                existing_path.to_string()
            };

            // Read existing file to get current data
            let mut existing_data = tokio::fs::read_to_string(&actual_path)
                .await
                .unwrap_or_default();

            // Append new rows (skip header line if columns match)
            let new_csv = csv_text.as_ref();
            let rows_to_append = if let Some(newline_pos) = new_csv.find('\n') {
                // Skip header row of the incoming CSV
                &new_csv[newline_pos + 1..]
            } else {
                new_csv
            };

            if !rows_to_append.trim().is_empty() {
                if !existing_data.ends_with('\n') {
                    existing_data.push('\n');
                }
                existing_data.push_str(rows_to_append);
            }

            // Rewrite the file and recompute stats
            tokio::fs::write(&actual_path, existing_data.as_bytes())
                .await
                .map_err(|e| AppError::Internal(format!("Failed to append data: {e}")))?;

            let (headers, row_count, column_stats) = compute_csv_stats(existing_data.as_bytes());

            // Update the existing document in Aegis-DB
            let mut updated = existing_doc.clone();
            if let Some(obj) = updated.as_object_mut() {
                obj.insert("row_count".into(), json!(row_count));
                obj.insert("file_size_bytes".into(), json!(existing_data.len()));
                obj.insert("column_stats".into(), serde_json::Value::Object(column_stats));
                obj.insert("columns".into(), json!(headers));
                obj.insert("updated_at".into(), json!(Utc::now().to_rfc3339()));
            }

            // Update via delete + recreate (Aegis-DB document update)
            let _ = state.aegis_delete_doc("datasets", existing_id).await;
            state.aegis_create_doc("datasets", updated.clone()).await?;

            return Ok(Json(json!({
                "status": "appended",
                "dataset_id": existing_id,
                "rows_added": csv_text.lines().count().saturating_sub(1),
                "total_rows": row_count,
            })));
        }
    }

    // No existing dataset — create new
    let (headers, row_count, column_stats) = compute_csv_stats(csv_text.as_bytes());

    let dataset_id = format!("ds_{}", &Uuid::new_v4().to_string()[..8]);
    let data_dir = format!("{}/datasets", state.config.data_dir);
    let _ = tokio::fs::create_dir_all(&data_dir).await;
    let file_path = format!("{data_dir}/{dataset_id}.csv");
    tokio::fs::write(&file_path, &file_data)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to save file: {e}")))?;

    // Build the document — start with all user-supplied metadata, then add system fields
    let mut doc = serde_json::Map::new();
    doc.insert("id".into(), json!(dataset_id));
    doc.insert("name".into(), json!(dataset_name));
    // Merge all user metadata (device_id, location, equipment_type, tags, domain,
    // source_info — anything the client sends)
    for (k, v) in &metadata {
        if k != "name" {
            doc.insert(k.clone(), v.clone());
        }
    }
    // Set defaults for expected fields if not provided
    doc.entry(String::from("domain")).or_insert_with(|| json!("general"));
    doc.entry(String::from("tags")).or_insert_with(|| json!([]));
    // System fields
    doc.insert("source".into(), json!("csv_upload"));
    doc.insert("columns".into(), json!(headers));
    doc.insert("row_count".into(), json!(row_count));
    doc.insert("file_size_bytes".into(), json!(file_data.len()));
    doc.insert("column_stats".into(), serde_json::Value::Object(column_stats));
    doc.insert("file_path".into(), json!(file_path));
    doc.insert("created_at".into(), json!(Utc::now().to_rfc3339()));
    doc.insert("created_by".into(), json!(auth.user_id));

    let doc_value = serde_json::Value::Object(doc);
    state.aegis_create_doc("datasets", doc_value.clone()).await?;
    Ok(Json(doc_value))
}

/// Find an existing dataset by name for a specific user (for append-on-duplicate logic).
async fn find_existing_dataset(
    state: &AppState,
    name: &str,
    user_id: &str,
) -> Option<serde_json::Value> {
    let docs = state.aegis_list_docs("datasets").await.ok()?;
    docs.into_iter().find(|doc| {
        doc.get("name").and_then(|v| v.as_str()) == Some(name)
            && doc.get("created_by").and_then(|v| v.as_str()) == Some(user_id)
    })
}

pub async fn get_dataset(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let mut doc = state.aegis_get_doc("datasets", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    // Redact sensitive fields in source_config before sending to client
    if let Some(sc) = doc.get("source_config").cloned() {
        if let Some(obj) = doc.as_object_mut() {
            obj.insert(
                "source_config".to_string(),
                prometheus_shield::credential_vault::redact_source_config(&sc),
            );
        }
    }
    Ok(Json(doc))
}

/// Query params for paginated preview.
#[derive(serde::Deserialize, Default)]
pub struct PreviewQuery {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub sort_col: Option<usize>,
    /// "asc" or "desc"
    pub sort_dir: Option<String>,
}

pub async fn get_dataset_preview(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Query(query): Query<PreviewQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("datasets", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    let file_path = doc
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::NotFound("Dataset file not found".into()))?;

    let raw_bytes = crate::api::data_lifecycle::read_dataset_bytes(file_path)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read file: {e}")))?;
    let data = String::from_utf8_lossy(&raw_bytes);

    let mut reader = csv::Reader::from_reader(data.as_bytes());
    let headers: Vec<String> = reader.headers()
        .map(|h| h.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let mut data_rows: Vec<Vec<String>> = Vec::new();
    for record in reader.records().flatten() {
        data_rows.push(record.iter().map(|s| s.to_string()).collect());
    }

    let total_rows = data_rows.len();

    // Sort if requested
    if let Some(sort_col) = query.sort_col {
        if sort_col < headers.len() {
            let desc = query.sort_dir.as_deref() == Some("desc");
            data_rows.sort_by(|a, b| {
                let va = a.get(sort_col).map(|s| s.as_str()).unwrap_or("");
                let vb = b.get(sort_col).map(|s| s.as_str()).unwrap_or("");
                // Try numeric comparison first
                let cmp = match (va.parse::<f64>(), vb.parse::<f64>()) {
                    (Ok(na), Ok(nb)) => na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal),
                    _ => va.cmp(vb),
                };
                if desc { cmp.reverse() } else { cmp }
            });
        }
    }

    // Paginate
    let page_size = query.page_size.unwrap_or(100).min(500);
    let page = query.page.unwrap_or(0);
    let total_pages = (total_rows + page_size - 1).max(1) / page_size.max(1);
    let start = page * page_size;
    let end = (start + page_size).min(total_rows);
    let page_rows: Vec<Vec<String>> = if start < total_rows {
        data_rows[start..end].to_vec()
    } else {
        vec![]
    };

    Ok(Json(json!({
        "headers": headers,
        "rows": page_rows,
        "total_rows": total_rows,
        "page": page,
        "page_size": page_size,
        "total_pages": total_pages,
    })))
}

pub async fn delete_dataset(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    // Get file path before deleting the document
    let doc = state.aegis_get_doc("datasets", &id).await.ok();
    if !auth.is_admin() {
        if let Some(ref d) = doc {
            if d.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
                return Err(AppError::Forbidden("Access denied".into()));
            }
        }
    }
    state.aegis_delete_doc("datasets", &id).await?;
    // Delete the CSV file from disk
    if let Some(doc) = doc {
        if let Some(fp) = doc.get("file_path").and_then(|v| v.as_str()) {
            let _ = tokio::fs::remove_file(fp).await;
        }
    }
    Ok(Json(json!({ "deleted": id })))
}

/// Toggle dataset ingestion status between "active" and "paused".
pub async fn toggle_dataset_status(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let new_status = body.get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("paused");

    if new_status != "active" && new_status != "paused" {
        return Err(AppError::BadRequest("status must be 'active' or 'paused'".into()));
    }

    let mut doc = state.aegis_get_doc("datasets", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    if let Some(obj) = doc.as_object_mut() {
        obj.insert("status".into(), json!(new_status));
        obj.insert("updated_at".into(), json!(Utc::now().to_rfc3339()));
    }

    let _ = state.aegis_delete_doc("datasets", &id).await;
    state.aegis_create_doc("datasets", doc.clone()).await?;

    Ok(Json(json!({ "id": id, "status": new_status })))
}

// ---------------------------------------------------------------------------
// Connect to external data source — dispatcher
// ---------------------------------------------------------------------------

/// Supported source types:
///   "aegis_bridge"  — Native Aegis-to-Aegis via AegisControlBridge (default)
///   "influxdb"      — InfluxDB v3 SQL API
///   "postgresql"    — PostgreSQL
///   "tidb"          — TiDB (MySQL-compatible)
///   "sqlite"        — SQLite3 file or URL
///   "mongodb"       — MongoDB collection
///   "spacetimedb"   — SpaceTimeDB SQL API
pub async fn connect_source(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Extension(shield): Extension<Arc<Shield>>,
    Json(config): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    // Enforce dataset count limit (admins bypass)
    if !auth.is_admin() {
        crate::api::billing::enforce_limit(
            &state, &auth.user_id, "datasets", "created_by",
            |t| t.max_datasets(), "Dataset",
        ).await?;
    }

    let user_id = auth.user_id.clone();
    let source_type = config
        .get("source_type")
        .and_then(|v| v.as_str())
        .unwrap_or("aegis_bridge")
        .to_string();

    // Check if user wants to save this connection for future reuse
    let save_conn = config.get("save_connection").and_then(|v| v.as_bool()).unwrap_or(false);
    let conn_name = config.get("connection_name").and_then(|v| v.as_str()).map(String::from);

    let result = match source_type.as_str() {
        "aegis_bridge" => connect_aegis_bridge(state.clone(), &shield, &config, &user_id).await,
        "influxdb"     => connect_influxdb(state.clone(), &shield, &config, &user_id).await,
        "postgresql"   => connect_sql(state.clone(), &shield, &config, "postgresql", &user_id).await,
        "tidb"         => connect_sql(state.clone(), &shield, &config, "tidb", &user_id).await,
        "sqlite"       => connect_sql(state.clone(), &shield, &config, "sqlite", &user_id).await,
        "mongodb"      => connect_mongodb(state.clone(), &shield, &config, &user_id).await,
        "spacetimedb"  => connect_spacetimedb(state.clone(), &shield, &config, &user_id).await,
        other => Err(AppError::BadRequest(format!(
            "Unsupported source_type: '{other}'. Supported: aegis_bridge, influxdb, postgresql, tidb, sqlite, mongodb, spacetimedb"
        ))),
    };

    // If connection succeeded and user wants to save it, store encrypted config
    if save_conn && result.is_ok() {
        let name = conn_name.unwrap_or_else(|| format!("{} connection", source_type));
        let conn_id = format!("conn_{}", &Uuid::new_v4().to_string()[..8]);
        let encrypted_config = prometheus_shield::credential_vault::encrypt_source_config(
            &config, &user_id,
        );
        let conn_doc = json!({
            "id": conn_id,
            "name": name,
            "source_type": source_type,
            "config": encrypted_config,
            "created_at": Utc::now().to_rfc3339(),
            "created_by": user_id,
        });
        let _ = state.aegis_create_doc("connections", conn_doc).await;
    }

    result
}

// ---------------------------------------------------------------------------
// Aegis-to-Aegis via AegisControlBridge (default / native path)
// ---------------------------------------------------------------------------

/// Connects directly to an edge controller's Aegis-DB instance.
/// The AegisControlBridge on each edge controller writes metrics
/// into its local Aegis-DB. This handler queries that remote Aegis-DB
/// and imports the data as a training dataset.
///
/// Expected config:
///   { "source_type": "aegis_bridge", "controller_ip": "192.168.1.100",
///     "aegis_port": 9090, "collection": "hardware_metrics", "limit": 10000 }
async fn connect_aegis_bridge(
    state: AppState,
    shield: &Shield,
    config: &serde_json::Value,
    user_id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let controller_ip = config
        .get("controller_ip")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("controller_ip required".into()))?;
    // Shield: validate controller IP is not an SSRF target
    shield.validate_ip(controller_ip).map_err(|e| AppError::BadRequest(format!("{e}")))?;
    let aegis_port = config
        .get("aegis_port")
        .and_then(|v| v.as_u64())
        .unwrap_or(9090);
    let collection = config
        .get("collection")
        .and_then(|v| v.as_str())
        .unwrap_or("hardware_metrics");
    let limit = config
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10000);

    let aegis_url = format!("http://{}:{}", controller_ip, aegis_port);

    // Query the edge Aegis-DB for documents in the collection
    let resp = state
        .http_client
        .get(format!("{}/api/v1/documents/collections/{}/documents", aegis_url, collection))
        .query(&[("limit", limit.to_string())])
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!(
            "Failed to connect to edge Aegis-DB at {aegis_url}: {e}"
        )))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Edge Aegis-DB returned {status}: {body}"
        )));
    }

    let data: Vec<serde_json::Value> = resp.json().await.map_err(|e| {
        AppError::Internal(format!("Failed to parse Aegis-DB response: {e}"))
    })?;

    if data.is_empty() {
        return Err(AppError::BadRequest("No data in edge Aegis-DB collection".into()));
    }

    let controller_name = crate::api::deployment::get_target_name_pub(controller_ip)
        .unwrap_or_else(|| controller_ip.to_string());

    // Convert Aegis-DB documents → rows (JSON array of objects → CSV)
    let csv_content = json_rows_to_csv(&data);
    save_imported_dataset(
        state,
        &csv_content,
        &format!("{} - {}", controller_name, collection),
        "aegis_bridge",
        &json!({ "controller_ip": controller_ip, "aegis_port": aegis_port, "collection": collection }),
        user_id,
    ).await
}

// ---------------------------------------------------------------------------
// InfluxDB (v3 SQL API)
// ---------------------------------------------------------------------------

async fn connect_influxdb(
    state: AppState,
    shield: &Shield,
    config: &serde_json::Value,
    user_id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let url = require_str(config, "url")?;
    // Shield: validate InfluxDB URL against SSRF
    shield.validate_url(url).map_err(|e| AppError::BadRequest(format!("{e}")))?;
    let database = config.get("database").and_then(|v| v.as_str()).unwrap_or("NexusEdge");
    let measurement = config.get("measurement").and_then(|v| v.as_str()).unwrap_or("ProcessingEngineCommands");
    let limit = config.get("limit").and_then(|v| v.as_u64()).unwrap_or(10000);

    let query = format!(
        "SELECT * FROM \"{}\" ORDER BY time DESC LIMIT {}",
        measurement, limit
    );
    let resp = state
        .http_client
        .post(format!("{url}/api/v3/query_sql"))
        .json(&json!({ "q": query, "db": database }))
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("InfluxDB connection failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("InfluxDB returned {status}: {body}")));
    }

    let data: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| AppError::Internal(format!("Failed to parse InfluxDB response: {e}")))?;

    if data.is_empty() {
        return Err(AppError::BadRequest("No data returned from InfluxDB".into()));
    }

    let csv_content = json_rows_to_csv(&data);
    save_imported_dataset(
        state, &csv_content,
        &format!("{} - {}", database, measurement),
        "influxdb",
        &json!({ "url": url, "database": database, "measurement": measurement }),
        user_id,
    ).await
}

// ---------------------------------------------------------------------------
// SQL-based: PostgreSQL, TiDB (MySQL wire protocol), SQLite3
// ---------------------------------------------------------------------------

/// All three speak SQL. The server proxies the query through a lightweight
/// HTTP request to the target. For hackathon scope, the user provides:
///   - connection_string: full DSN / URL
///   - query: SQL SELECT to execute
///
/// The server executes via the target's HTTP/REST API or direct connection
/// and normalizes the result rows into CSV.
async fn connect_sql(
    state: AppState,
    shield: &Shield,
    config: &serde_json::Value,
    db_type: &str,
    user_id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let connection_string = require_str(config, "connection_string")?;
    let query = require_str(config, "query")?;
    // Shield: SQL firewall + connection string validation
    shield.validate_sql(query).map_err(|e| AppError::BadRequest(format!("{e}")))?;
    shield.validate_connection_string(connection_string).map_err(|e| AppError::BadRequest(format!("{e}")))?;
    let limit = config.get("limit").and_then(|v| v.as_u64()).unwrap_or(10000);

    // For SQL databases, we use the Aegis-DB SQL proxy endpoint.
    // Aegis-DB can federate queries to external SQL sources when given a DSN.
    let resp = state
        .aegis_request(
            reqwest::Method::POST,
            "/api/v1/query/federated",
            Some(json!({
                "dsn": connection_string,
                "db_type": db_type,
                "sql": format!("{} LIMIT {}", query.trim_end_matches(';'), limit),
            })),
        )
        .await;

    // If Aegis-DB federated query is available, use it
    if let Ok(data) = resp {
        if let Some(rows) = data.get("rows").and_then(|v| v.as_array()) {
            if rows.is_empty() {
                return Err(AppError::BadRequest("Query returned no rows".into()));
            }
            let csv_content = json_rows_to_csv(rows);
            return save_imported_dataset(
                state, &csv_content,
                &format!("{} query", db_type),
                db_type,
                &json!({ "connection_string": connection_string, "query": query }),
                user_id,
            ).await;
        }
    }

    // Fallback: direct HTTP query for databases that expose a REST API
    // TiDB has a status API, PostgreSQL can be accessed via PostgREST, etc.
    let api_url = config.get("api_url").and_then(|v| v.as_str());

    if let Some(api_url) = api_url {
        let resp = state
            .http_client
            .post(api_url)
            .json(&json!({ "query": query, "limit": limit }))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("{db_type} connection failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("{db_type} returned {status}: {body}")));
        }

        let data: serde_json::Value = resp.json().await
            .map_err(|e| AppError::Internal(format!("Failed to parse {db_type} response: {e}")))?;

        let rows = data.as_array()
            .or_else(|| data.get("rows").and_then(|v| v.as_array()))
            .or_else(|| data.get("results").and_then(|v| v.as_array()))
            .ok_or_else(|| AppError::Internal(format!("Unexpected {db_type} response format")))?;

        if rows.is_empty() {
            return Err(AppError::BadRequest("Query returned no rows".into()));
        }

        let csv_content = json_rows_to_csv(rows);
        return save_imported_dataset(
            state, &csv_content,
            &format!("{} query", db_type),
            db_type,
            &json!({ "api_url": api_url, "query": query }),
            user_id,
        ).await;
    }

    Err(AppError::BadRequest(format!(
        "For {db_type}, provide either a connection_string (for Aegis-DB federated query) or an api_url (for direct REST access)"
    )))
}

// ---------------------------------------------------------------------------
// MongoDB
// ---------------------------------------------------------------------------

/// Connects to a MongoDB instance via its Data API (Atlas) or a REST proxy.
///
/// Expected config:
///   { "source_type": "mongodb", "api_url": "https://data.mongodb-api.com/...",
///     "api_key": "...", "database": "mydb", "collection": "data", "limit": 10000 }
async fn connect_mongodb(
    state: AppState,
    shield: &Shield,
    config: &serde_json::Value,
    user_id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let api_url = require_str(config, "api_url")?;
    // Shield: validate MongoDB API URL against SSRF
    shield.validate_url(api_url).map_err(|e| AppError::BadRequest(format!("{e}")))?;
    let database = require_str(config, "database")?;
    let collection = require_str(config, "collection")?;
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let limit = config.get("limit").and_then(|v| v.as_u64()).unwrap_or(10000);
    let filter = config.get("filter").cloned().unwrap_or_else(|| json!({}));

    // MongoDB Data API: POST /action/find
    let mut req = state
        .http_client
        .post(format!("{}/action/find", api_url.trim_end_matches('/')))
        .json(&json!({
            "dataSource": database,
            "database": database,
            "collection": collection,
            "filter": filter,
            "limit": limit,
            "sort": { "timestamp": -1 },
        }))
        .timeout(std::time::Duration::from_secs(30));

    if !api_key.is_empty() {
        req = req.header("api-key", api_key);
    }

    let resp = req.send().await
        .map_err(|e| AppError::Internal(format!("MongoDB connection failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("MongoDB returned {status}: {body}")));
    }

    let body: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("Failed to parse MongoDB response: {e}")))?;

    let documents = body.get("documents")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Internal("MongoDB response missing 'documents' array".into()))?;

    if documents.is_empty() {
        return Err(AppError::BadRequest("No documents returned from MongoDB".into()));
    }

    let csv_content = json_rows_to_csv(documents);
    save_imported_dataset(
        state, &csv_content,
        &format!("{}.{}", database, collection),
        "mongodb",
        &json!({ "api_url": api_url, "database": database, "collection": collection }),
        user_id,
    ).await
}

// ---------------------------------------------------------------------------
// SpaceTimeDB
// ---------------------------------------------------------------------------

/// Connects to a SpaceTimeDB instance via its HTTP SQL API.
///
/// Expected config:
///   { "source_type": "spacetimedb", "url": "http://localhost:3000",
///     "database": "mydb", "query": "SELECT * FROM data", "limit": 10000 }
async fn connect_spacetimedb(
    state: AppState,
    shield: &Shield,
    config: &serde_json::Value,
    user_id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let url = require_str(config, "url")?;
    // Shield: validate SpaceTimeDB URL against SSRF
    shield.validate_url(url).map_err(|e| AppError::BadRequest(format!("{e}")))?;
    let database = require_str(config, "database")?;
    let query = config.get("query").and_then(|v| v.as_str())
        .unwrap_or("SELECT * FROM sensor_readings");
    // Shield: SQL firewall for SpaceTimeDB query
    shield.validate_sql(query).map_err(|e| AppError::BadRequest(format!("{e}")))?;
    let limit = config.get("limit").and_then(|v| v.as_u64()).unwrap_or(10000);
    let token = config.get("token").and_then(|v| v.as_str()).unwrap_or("");

    // SpaceTimeDB SQL endpoint
    let full_query = format!("{} LIMIT {}", query.trim_end_matches(';'), limit);
    let mut req = state
        .http_client
        .post(format!("{}/database/{}/sql", url.trim_end_matches('/'), database))
        .body(full_query.clone())
        .header("Content-Type", "text/plain")
        .timeout(std::time::Duration::from_secs(30));

    if !token.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let resp = req.send().await
        .map_err(|e| AppError::Internal(format!("SpaceTimeDB connection failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("SpaceTimeDB returned {status}: {body}")));
    }

    let body: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("Failed to parse SpaceTimeDB response: {e}")))?;

    // SpaceTimeDB returns rows in various formats; normalize
    let rows = body.as_array()
        .or_else(|| body.get("rows").and_then(|v| v.as_array()))
        .ok_or_else(|| AppError::Internal("Unexpected SpaceTimeDB response format".into()))?;

    if rows.is_empty() {
        return Err(AppError::BadRequest("No data returned from SpaceTimeDB".into()));
    }

    let csv_content = json_rows_to_csv(rows);
    save_imported_dataset(
        state, &csv_content,
        &format!("SpaceTimeDB {}", database),
        "spacetimedb",
        &json!({ "url": url, "database": database, "query": query }),
        user_id,
    ).await
}

// ---------------------------------------------------------------------------
// Model Recommendations — analyze dataset and suggest architectures
// ---------------------------------------------------------------------------

pub async fn recommend_models(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("datasets", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let name = doc.get("name").and_then(|v| v.as_str()).unwrap_or("Dataset");
    let row_count = doc.get("row_count").and_then(|v| v.as_u64()).unwrap_or(0);
    let columns: Vec<String> = doc.get("columns")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|c| c.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let feature_count = columns.len().saturating_sub(1);
    let column_stats = doc.get("column_stats").cloned().unwrap_or_else(|| json!({}));

    // Build dataset context for the AI agent
    let dataset_context = build_recommend_context(name, row_count, &columns, &column_stats);

    // Try DO Gradient AI agent first, fall back to local heuristics
    let recommendations = if let (Some(ref endpoint), Some(ref api_key)) =
        (&state.config.gradient_endpoint, &state.config.gradient_api_key)
    {
        match call_recommend_agent(&state, endpoint, api_key, &dataset_context).await {
            Ok(recs) => {
                tracing::info!("Recommendations from DO Gradient AI agent ({} results)", recs.len());
                recs
            }
            Err(e) => {
                tracing::warn!("DO Gradient AI agent call failed, using local fallback: {e}");
                build_local_recommendations(&columns, row_count, &column_stats)
            }
        }
    } else {
        tracing::info!("DO Gradient AI not configured, using local recommendations");
        build_local_recommendations(&columns, row_count, &column_stats)
    };

    Ok(Json(json!({
        "dataset_id": id,
        "dataset_name": name,
        "row_count": row_count,
        "feature_count": feature_count,
        "columns": columns,
        "recommendations": recommendations,
    })))
}

// ---------------------------------------------------------------------------
// DO Gradient AI agent call for recommendations
// ---------------------------------------------------------------------------

const RECOMMEND_SYSTEM_PROMPT: &str = r#"You are PrometheusForge, the AI recommendation engine for the Prometheus ML platform. Your job is to analyze a dataset and recommend the best neural network architectures from the 13 available in AxonML.

Available architectures:
- lstm_autoencoder: LSTM encoder-decoder for anomaly detection via reconstruction error. Best for: time-series anomaly detection, drift detection, unsupervised pattern learning.
- gru_predictor: GRU recurrent network for multi-horizon prediction. Best for: forecasting, event prediction, probability estimation over time windows.
- rnn: Vanilla RNN for simple sequence modeling. Best for: lightweight temporal pattern recognition, short sequences.
- sentinel: MLP health/quality scorer. Best for: multi-channel scoring, tabular data with many numeric features, regression to a single score.
- conv1d: 1D convolutional network for temporal/sequential features. Best for: signal processing, waveform analysis, sequence classification.
- conv2d: 2D convolutional network for spatial data. Best for: image classification, spatial pattern recognition.
- resnet: Deep residual network (ResNet-18). Best for: complex image classification, deep feature extraction with skip connections.
- vgg: VGG-style deep CNN. Best for: image classification where simplicity and depth matter.
- vit: Vision Transformer. Best for: image classification with global spatial attention, when patch-level relationships matter.
- bert: Bidirectional transformer encoder. Best for: text classification, sentiment analysis, intent detection, sequence labeling.
- gpt2: Autoregressive transformer decoder. Best for: text generation, language modeling, sequence completion.
- nexus: Multi-modal fusion with cross-attention. Best for: datasets combining different data types (e.g., numeric + text, sensor + image).
- phantom: Ultra-lightweight MLP for edge deployment. Best for: resource-constrained inference, when model size < 128KB is critical.

Prometheus is a GENERAL-PURPOSE ML platform. Users upload ANY kind of data: medical images, financial time series, NLP corpora, game analytics, genomics, industrial sensors, satellite imagery, etc. Never assume a specific domain.

You MUST respond with ONLY a JSON array of recommendation objects. No markdown, no explanation, no code fences. Each object must have exactly these fields:
- "architecture": one of the 13 architecture keys above
- "name": human-readable display name
- "match_score": integer 0-100 (how well this architecture fits the data)
- "description": 1-2 sentence description of why this architecture fits THIS specific dataset
- "use_case": short category label (e.g., "Anomaly Detection", "Image Classification", "Text Classification", "Forecasting", "Regression", "Sequence Modeling")
- "inputs": array of column names that would be model inputs
- "outputs": array of expected output names
- "inference_result": one sentence describing what the model produces at inference time
- "hyperparameters": object with keys: learning_rate, batch_size, epochs, hidden_dim, num_layers, sequence_length (if applicable), dropout, optimizer ("adam" or "adamw"), loss ("mse", "bce", or "cross_entropy")

Return 2-5 recommendations sorted by match_score descending. Tailor hyperparameters to the actual dataset size and characteristics."#;

fn build_recommend_context(
    name: &str,
    row_count: u64,
    columns: &[String],
    column_stats: &serde_json::Value,
) -> String {
    let mut ctx = format!(
        "Dataset: {name}\nRows: {row_count}\nColumns ({count}): {cols}",
        count = columns.len(),
        cols = columns.join(", "),
    );

    if let Some(stats) = column_stats.as_object() {
        ctx.push_str("\n\nColumn Statistics:");
        for (col, stat) in stats {
            let min = stat.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let max = stat.get("max").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let mean = stat.get("mean").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let std = stat.get("std").and_then(|v| v.as_f64()).unwrap_or(0.0);
            ctx.push_str(&format!("\n  {col}: min={min:.4}, max={max:.4}, mean={mean:.4}, std={std:.4}"));
        }
    }

    ctx
}

async fn call_recommend_agent(
    state: &AppState,
    endpoint: &str,
    api_key: &str,
    dataset_context: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let messages = vec![
        json!({ "role": "system", "content": RECOMMEND_SYSTEM_PROMPT }),
        json!({ "role": "user", "content": format!("Analyze this dataset and recommend the best architectures:\n\n{dataset_context}") }),
    ];

    let body = json!({
        "model": "agent",
        "messages": messages,
        "max_tokens": 8000,
    });

    let resp = state
        .http_client
        .post(endpoint)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("API returned {status}: {text}"));
    }

    let data: serde_json::Value = resp.json().await
        .map_err(|e| format!("Parse error: {e}"))?;

    // Extract content from OpenAI-compatible response
    let content = data.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .or_else(|| data.get("response").and_then(|r| r.as_str()))
        .ok_or_else(|| format!("Unexpected response format: {}", data))?;

    // Parse the JSON array from the response (strip markdown fences if present)
    let json_str = content.trim();
    let json_str = json_str
        .strip_prefix("```json").or_else(|| json_str.strip_prefix("```"))
        .unwrap_or(json_str);
    let json_str = json_str.strip_suffix("```").unwrap_or(json_str).trim();

    let recs: Vec<serde_json::Value> = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse AI recommendations as JSON: {e}\nRaw: {content}"))?;

    // Validate each recommendation has required fields
    let valid_archs = [
        "lstm_autoencoder", "gru_predictor", "rnn", "sentinel", "conv1d", "conv2d",
        "resnet", "vgg", "vit", "bert", "gpt2", "nexus", "phantom",
    ];
    let validated: Vec<serde_json::Value> = recs.into_iter().filter(|r| {
        r.get("architecture")
            .and_then(|a| a.as_str())
            .map(|a| valid_archs.contains(&a))
            .unwrap_or(false)
            && r.get("name").is_some()
            && r.get("match_score").is_some()
            && r.get("hyperparameters").is_some()
    }).collect();

    if validated.is_empty() {
        return Err("AI returned no valid recommendations".to_string());
    }

    Ok(validated)
}

// ---------------------------------------------------------------------------
// Local fallback recommendations (domain-agnostic)
// ---------------------------------------------------------------------------

fn build_local_recommendations(
    columns: &[String],
    row_count: u64,
    column_stats: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let stats = column_stats.as_object();
    let numeric_cols = stats.map(|s| s.len()).unwrap_or(0);
    let high_variance_cols = stats.map(|s| {
        s.values().filter(|v| v.get("std").and_then(|s| s.as_f64()).unwrap_or(0.0) > 0.1).count()
    }).unwrap_or(0);

    // Detect data characteristics from column names (domain-agnostic)
    let has_temporal = columns.iter().any(|c| {
        let cl = c.to_lowercase();
        cl.contains("time") || cl.contains("date") || cl == "ts" || cl == "epoch"
            || cl.contains("timestamp") || cl.contains("datetime")
    });
    let has_text = columns.iter().any(|c| {
        let cl = c.to_lowercase();
        cl.contains("text") || cl.contains("sentence") || cl.contains("description")
            || cl.contains("body") || cl.contains("content") || cl.contains("review")
            || cl.contains("title") || cl.contains("comment") || cl.contains("message")
            || cl.contains("abstract") || cl.contains("summary") || cl.contains("transcript")
    });
    let has_image = columns.iter().any(|c| {
        let cl = c.to_lowercase();
        cl.contains("image") || cl.contains("pixel") || cl.contains("img")
            || cl.contains("photo") || cl.contains("frame") || cl.contains("scan")
    });
    let has_label = columns.iter().any(|c| {
        let cl = c.to_lowercase();
        cl.contains("label") || cl.contains("class") || cl.contains("target")
            || cl.contains("category") || cl.contains("outcome") || cl.contains("diagnosis")
            || cl.contains("result") || cl.contains("status") || cl.contains("type")
    });
    let has_sequence = columns.iter().any(|c| {
        let cl = c.to_lowercase();
        cl.contains("sequence") || cl.contains("seq") || cl.contains("step")
            || cl.contains("position") || cl.contains("index") || cl.contains("order")
    });

    let non_meta_cols: Vec<String> = columns.iter().filter(|c| {
        let cl = c.to_lowercase();
        !cl.contains("time") && !cl.contains("date") && cl != "ts" && cl != "id"
            && cl != "epoch" && !cl.contains("timestamp")
    }).cloned().collect();

    let epochs = if row_count > 50000 { 15 } else if row_count > 10000 { 12 } else { 10 };
    let batch_size: u64 = if row_count > 10000 { 128 } else { 64 };

    let mut recommendations: Vec<serde_json::Value> = Vec::new();

    // Temporal anomaly detection
    if has_temporal && numeric_cols > 2 {
        recommendations.push(json!({
            "architecture": "lstm_autoencoder",
            "name": "LSTM Autoencoder",
            "match_score": if high_variance_cols > 3 { 95 } else { 80 },
            "description": "Learns to reconstruct normal patterns and detects anomalies by measuring reconstruction error. Works with any time-series or sequential numeric data.",
            "use_case": "Anomaly Detection",
            "inputs": &non_meta_cols,
            "outputs": ["anomaly_score", "reconstruction_error"],
            "inference_result": "Anomaly score (0.0 = normal, 1.0 = anomalous) based on reconstruction error",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": batch_size,
                "epochs": epochs,
                "hidden_dim": 8,
                "bottleneck_dim": 4,
                "num_layers": 1,
                "sequence_length": 5,
                "dropout": 0.0,
                "optimizer": "adam",
                "loss": "mse",
            },
        }));
    }

    // Temporal prediction
    if has_temporal && numeric_cols > 1 {
        recommendations.push(json!({
            "architecture": "gru_predictor",
            "name": "GRU Predictor",
            "match_score": if has_label { 70 } else { 85 },
            "description": "Predicts future values or event probabilities at multiple time horizons. Suitable for any forecasting or prediction task on sequential data.",
            "use_case": "Forecasting",
            "inputs": &non_meta_cols,
            "outputs": ["prediction_short", "prediction_mid", "prediction_long"],
            "inference_result": "Predicted probabilities or values at multiple future horizons (0.0-1.0)",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": batch_size,
                "epochs": epochs,
                "hidden_dim": 8,
                "num_layers": 1,
                "sequence_length": 5,
                "dropout": 0.0,
                "optimizer": "adamw",
                "loss": "bce",
            },
        }));
    }

    // Multi-feature scoring/regression
    if numeric_cols > 6 {
        recommendations.push(json!({
            "architecture": "sentinel",
            "name": "Sentinel Scorer",
            "match_score": if numeric_cols > 12 { 92 } else { 75 },
            "description": "MLP that computes a composite score from many numeric features by analyzing cross-feature dependencies. Works for any high-dimensional tabular data.",
            "use_case": "Scoring / Regression",
            "inputs": &non_meta_cols,
            "outputs": ["score", "feature_contributions"],
            "inference_result": "Composite score (0.0-1.0) with per-feature contribution breakdown",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": batch_size,
                "epochs": if row_count > 50000 { 30 } else { 25 },
                "hidden_dim": 8,
                "num_layers": 1,
                "sequence_length": 5,
                "dropout": 0.1,
                "optimizer": "adam",
                "loss": "mse",
            },
        }));
    }

    // Text classification
    if has_text {
        recommendations.push(json!({
            "architecture": "bert",
            "name": "BERT Text Classifier",
            "match_score": 90,
            "description": "Bidirectional transformer encoder for text understanding. Handles sentiment analysis, topic classification, intent detection, document categorization, and any text classification task.",
            "use_case": "Text Classification",
            "inputs": columns.iter().filter(|c| {
                let cl = c.to_lowercase();
                cl.contains("text") || cl.contains("sentence") || cl.contains("body")
                    || cl.contains("content") || cl.contains("review") || cl.contains("title")
                    || cl.contains("comment") || cl.contains("message") || cl.contains("abstract")
                    || cl.contains("transcript")
            }).cloned().collect::<Vec<_>>(),
            "outputs": ["class_prediction", "confidence"],
            "inference_result": "Predicted class label with confidence score for each input text",
            "hyperparameters": {
                "learning_rate": 0.00005,
                "batch_size": 32,
                "epochs": 10,
                "hidden_dim": 8,
                "num_layers": 1,
                "dropout": 0.1,
                "optimizer": "adamw",
                "loss": "cross_entropy",
            },
        }));
    }

    // Image classification
    if has_image {
        recommendations.push(json!({
            "architecture": "resnet",
            "name": "ResNet Image Classifier",
            "match_score": 88,
            "description": "Deep residual network with skip connections for image classification. Handles medical scans, satellite imagery, photographs, visual inspection, and any image data.",
            "use_case": "Image Classification",
            "inputs": ["image"],
            "outputs": ["class_prediction", "top_k_classes"],
            "inference_result": "Top-K class predictions with confidence scores for each image",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": 32,
                "epochs": 10,
                "hidden_dim": 8,
                "num_layers": 1,
                "dropout": 0.1,
                "optimizer": "adamw",
                "loss": "cross_entropy",
            },
        }));
    }

    // Sequence data (non-temporal)
    if has_sequence && !has_temporal && numeric_cols > 0 {
        recommendations.push(json!({
            "architecture": "rnn",
            "name": "RNN Sequence Model",
            "match_score": 75,
            "description": "Vanilla RNN for sequential pattern recognition. Suitable for ordered data like genomic sequences, user action logs, or any indexed sequential data.",
            "use_case": "Sequence Modeling",
            "inputs": &non_meta_cols,
            "outputs": ["prediction"],
            "inference_result": "Class prediction or value estimate from sequential input",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": batch_size,
                "epochs": epochs,
                "hidden_dim": 8,
                "num_layers": 1,
                "sequence_length": 5,
                "dropout": 0.0,
                "optimizer": "adam",
                "loss": "bce",
            },
        }));
    }

    // Classification with labels (tabular)
    if has_label && !has_text && !has_image && numeric_cols > 0 {
        recommendations.push(json!({
            "architecture": "conv1d",
            "name": "Conv1D Classifier",
            "match_score": 78,
            "description": "1D convolutional network for structured/tabular data classification. Captures local feature patterns efficiently.",
            "use_case": "Classification",
            "inputs": columns.iter().filter(|c| {
                let cl = c.to_lowercase();
                !cl.contains("label") && !cl.contains("class") && !cl.contains("target")
                    && !cl.contains("time") && cl != "id" && !cl.contains("category")
                    && !cl.contains("outcome")
            }).cloned().collect::<Vec<_>>(),
            "outputs": ["class_prediction", "confidence"],
            "inference_result": "Predicted class label with confidence score",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": batch_size,
                "epochs": 10,
                "hidden_dim": 8,
                "num_layers": 1,
                "dropout": 0.0,
                "optimizer": "adam",
                "loss": "cross_entropy",
            },
        }));
    }

    // Lightweight edge model (always offer if there's numeric data)
    if numeric_cols > 1 {
        recommendations.push(json!({
            "architecture": "phantom",
            "name": "Phantom Edge Model",
            "match_score": 65,
            "description": "Ultra-lightweight MLP optimized for edge/embedded deployment. Minimal memory footprint (~1.5 MB), fast inference. Trades accuracy for size and speed.",
            "use_case": "Edge Inference",
            "inputs": &non_meta_cols,
            "outputs": ["prediction"],
            "inference_result": "Single prediction value optimized for fast inference on resource-constrained devices",
            "hyperparameters": {
                "learning_rate": 0.001,
                "batch_size": batch_size,
                "epochs": 10,
                "hidden_dim": 8,
                "num_layers": 1,
                "dropout": 0.0,
                "optimizer": "adam",
                "loss": "bce",
            },
        }));
    }

    // Sort by match score descending
    recommendations.sort_by(|a, b| {
        let sa = a.get("match_score").and_then(|v| v.as_u64()).unwrap_or(0);
        let sb = b.get("match_score").and_then(|v| v.as_u64()).unwrap_or(0);
        sb.cmp(&sa)
    });

    recommendations
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Extract a required string field from config JSON.
fn require_str<'a>(config: &'a serde_json::Value, field: &str) -> AppResult<&'a str> {
    config
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest(format!("'{field}' is required")))
}

/// Convert an array of JSON objects into CSV text.
/// Extracts headers from the union of all keys in the first row,
/// skipping MongoDB's `_id` field.
fn json_rows_to_csv(rows: &[serde_json::Value]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    // Collect headers from the first row
    let headers: Vec<String> = rows[0]
        .as_object()
        .map(|o| o.keys().filter(|k| *k != "_id").cloned().collect())
        .unwrap_or_default();

    if headers.is_empty() {
        return String::new();
    }

    let mut csv = headers.join(",");
    csv.push('\n');

    for row in rows {
        let values: Vec<String> = headers
            .iter()
            .map(|h| {
                row.get(h)
                    .map(|v| match v {
                        serde_json::Value::String(s) => {
                            // Escape commas in string values
                            if s.contains(',') || s.contains('"') {
                                format!("\"{}\"", s.replace('"', "\"\""))
                            } else {
                                s.clone()
                            }
                        }
                        serde_json::Value::Null => String::new(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default()
            })
            .collect();
        csv.push_str(&values.join(","));
        csv.push('\n');
    }

    csv
}

/// Save imported data as a dataset: write CSV file, compute stats, store in Aegis-DB.
async fn save_imported_dataset(
    state: AppState,
    csv_content: &str,
    name: &str,
    source: &str,
    source_config: &serde_json::Value,
    user_id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let dataset_id = format!("ds_{}", &Uuid::new_v4().to_string()[..8]);
    let data_dir = format!("{}/datasets", state.config.data_dir);
    let _ = tokio::fs::create_dir_all(&data_dir).await;
    let file_path = format!("{data_dir}/{dataset_id}.csv");

    tokio::fs::write(&file_path, csv_content.as_bytes())
        .await
        .map_err(|e| AppError::Internal(format!("Failed to save dataset: {e}")))?;

    let (headers, row_count, column_stats) = compute_csv_stats(csv_content.as_bytes());

    // Encrypt sensitive fields (api_key, connection_string, token, etc.)
    // before storing in Aegis-DB — AutomataNexus cannot read raw credentials.
    let encrypted_config = prometheus_shield::credential_vault::encrypt_source_config(
        source_config, user_id,
    );

    let doc = json!({
        "id": dataset_id,
        "name": name,
        "domain": "general",
        "source": source,
        "source_config": encrypted_config,
        "columns": headers,
        "row_count": row_count,
        "file_size_bytes": csv_content.len(),
        "column_stats": column_stats,
        "file_path": file_path,
        "created_at": Utc::now().to_rfc3339(),
        "created_by": user_id,
    });

    state.aegis_create_doc("datasets", doc.clone()).await?;

    Ok(Json(json!({
        "status": "imported",
        "dataset_id": dataset_id,
        "rows_imported": row_count,
        "columns": headers.len(),
        "source": source,
    })))
}

// ---------------------------------------------------------------------------
// Dataset Validation & Locking
// ---------------------------------------------------------------------------

/// Validate a dataset — checks column type consistency, missing values, outliers.
/// Sets `is_validated` and `locked` on the doc.
pub async fn validate_dataset(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("datasets", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let file_path = doc
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::NotFound("Dataset file not found".into()))?;

    let raw_bytes = crate::api::data_lifecycle::read_dataset_bytes(file_path)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read file: {e}")))?;
    let data = String::from_utf8_lossy(&raw_bytes);

    let mut reader = csv::Reader::from_reader(data.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map(|h| h.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    if headers.is_empty() {
        return Ok(Json(json!({
            "valid": false,
            "errors": ["Dataset has no columns/headers"],
        })));
    }

    let num_cols = headers.len();
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut row_count = 0usize;
    let mut empty_counts: Vec<usize> = vec![0; num_cols];
    let mut numeric_counts: Vec<usize> = vec![0; num_cols];
    let mut string_counts: Vec<usize> = vec![0; num_cols];
    let mut col_widths_mismatch = 0usize;

    for record in reader.records().flatten() {
        row_count += 1;
        if record.len() != num_cols {
            col_widths_mismatch += 1;
            continue;
        }
        for (i, val) in record.iter().enumerate() {
            if i >= num_cols { break; }
            let trimmed = val.trim();
            if trimmed.is_empty() || trimmed == "null" || trimmed == "NULL" || trimmed == "NA" || trimmed == "NaN" {
                empty_counts[i] += 1;
            } else if trimmed.parse::<f64>().is_ok() {
                numeric_counts[i] += 1;
            } else {
                string_counts[i] += 1;
            }
        }
    }

    if row_count == 0 {
        return Ok(Json(json!({
            "valid": false,
            "errors": ["Dataset has no data rows"],
        })));
    }

    // Check column width mismatches
    if col_widths_mismatch > 0 {
        errors.push(format!(
            "{} row(s) have mismatched column count (expected {}, got different)",
            col_widths_mismatch, num_cols
        ));
    }

    // Per-column validation
    let mut column_report: Vec<serde_json::Value> = Vec::new();
    for i in 0..num_cols {
        let total_non_empty = numeric_counts[i] + string_counts[i];
        let empty_pct = if row_count > 0 {
            (empty_counts[i] as f64 / row_count as f64) * 100.0
        } else { 0.0 };

        // Determine inferred type
        let inferred_type = if total_non_empty == 0 {
            "empty"
        } else if string_counts[i] == 0 {
            "numeric"
        } else if numeric_counts[i] == 0 {
            "string"
        } else {
            "mixed"
        };

        // Flag issues
        let mut col_issues: Vec<String> = Vec::new();
        if inferred_type == "mixed" {
            let mix_pct = (numeric_counts[i].min(string_counts[i]) as f64 / total_non_empty as f64) * 100.0;
            col_issues.push(format!(
                "Mixed types: {} numeric + {} string values ({:.1}% minority)",
                numeric_counts[i], string_counts[i], mix_pct
            ));
            errors.push(format!("Column '{}': mixed data types detected", headers[i]));
        }

        if empty_pct > 50.0 {
            errors.push(format!("Column '{}': {:.1}% missing values", headers[i], empty_pct));
            col_issues.push(format!("{:.1}% missing", empty_pct));
        } else if empty_pct > 10.0 {
            warnings.push(format!("Column '{}': {:.1}% missing values", headers[i], empty_pct));
            col_issues.push(format!("{:.1}% missing (warning)", empty_pct));
        }

        if inferred_type == "empty" {
            errors.push(format!("Column '{}': entirely empty", headers[i]));
            col_issues.push("all values empty".into());
        }

        column_report.push(json!({
            "column": headers[i],
            "inferred_type": inferred_type,
            "numeric_count": numeric_counts[i],
            "string_count": string_counts[i],
            "empty_count": empty_counts[i],
            "empty_pct": format!("{:.1}%", empty_pct),
            "issues": col_issues,
        }));
    }

    let is_valid = errors.is_empty();

    // Update dataset doc
    let _ = state.aegis_update_doc("datasets", &id, json!({
        "is_validated": is_valid,
        "locked": is_valid,
        "validation_errors": errors,
        "validation_warnings": warnings,
        "validated_at": Utc::now().to_rfc3339(),
        "validated_rows": row_count,
    })).await;

    // Compress on lock — dataset is frozen, save disk space
    if is_valid {
        if let Err(e) = crate::api::data_lifecycle::compress_dataset(&state, &id).await {
            tracing::warn!("Auto-compress on validate failed for {}: {}", id, e);
        }
    }

    Ok(Json(json!({
        "valid": is_valid,
        "rows_scanned": row_count,
        "columns": column_report,
        "errors": errors,
        "warnings": warnings,
    })))
}

/// Unlock a validated dataset for editing. Clears validation status.
pub async fn unlock_dataset(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("datasets", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    // Decompress on unlock — dataset needs to be writable again
    if let Err(e) = crate::api::data_lifecycle::decompress_dataset(&state, &id).await {
        tracing::warn!("Auto-decompress on unlock failed for {}: {}", id, e);
    }

    let _ = state.aegis_update_doc("datasets", &id, json!({
        "is_validated": false,
        "locked": false,
        "unlocked_at": Utc::now().to_rfc3339(),
    })).await;

    Ok(Json(json!({ "id": id, "locked": false, "is_validated": false })))
}

/// Parse CSV bytes and compute column statistics (min, max, mean, std).
fn compute_csv_stats(csv_bytes: &[u8]) -> (Vec<String>, usize, serde_json::Map<String, serde_json::Value>) {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    let headers: Vec<String> = reader
        .headers()
        .map(|h| h.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let mut col_values: Vec<Vec<f64>> = vec![Vec::new(); headers.len()];
    let mut row_count = 0usize;

    for record in reader.records().flatten() {
        row_count += 1;
        for (i, val) in record.iter().enumerate() {
            if let Ok(v) = val.parse::<f64>() {
                if i < col_values.len() {
                    col_values[i].push(v);
                }
            }
        }
    }

    let mut column_stats = serde_json::Map::new();
    for (i, header) in headers.iter().enumerate() {
        if header == "time" || header == "timestamp" || header == "_id" {
            continue;
        }
        let vals = &col_values[i];
        if !vals.is_empty() {
            let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let sum: f64 = vals.iter().sum();
            let mean = sum / vals.len() as f64;
            let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
            let std_dev = variance.sqrt();
            column_stats.insert(
                header.clone(),
                json!({ "min": min, "max": max, "mean": mean, "std": std_dev }),
            );
        }
    }

    (headers, row_count, column_stats)
}

// ---------------------------------------------------------------------------
// Dataset Catalog — pre-loaded datasets available for one-click import
// ---------------------------------------------------------------------------

/// List available datasets from the pre-loaded catalog (/opt/datasets/).
/// Returns domain → datasets hierarchy without transferring actual data.
pub async fn list_catalog(
    Extension(_auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let catalog_root = std::path::Path::new("/opt/datasets");
    if !catalog_root.exists() {
        return Ok(Json(vec![]));
    }

    let mut catalog = Vec::new();

    // Scan domain directories
    let mut domains: Vec<_> = std::fs::read_dir(catalog_root)
        .map_err(|e| AppError::Internal(format!("Cannot read catalog: {e}")))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    domains.sort_by_key(|e| e.file_name());

    for domain_entry in &domains {
        let domain = domain_entry.file_name().to_string_lossy().to_string();
        let domain_path = domain_entry.path();

        // Find data files (CSV, JSON, Parquet, etc.) — skip .md and Zone.Identifier
        let mut items = Vec::new();
        collect_catalog_files(&domain_path, &domain, &mut items, 0);

        if !items.is_empty() {
            // Calculate domain total size
            let total_size: u64 = items.iter()
                .filter_map(|i| i.get("file_size_bytes").and_then(|v| v.as_u64()))
                .sum();

            catalog.push(json!({
                "domain": domain,
                "dataset_count": items.len(),
                "total_size_bytes": total_size,
                "total_size": format_size(total_size),
                "datasets": items,
            }));
        }
    }

    Ok(Json(catalog))
}

fn collect_catalog_files(
    dir: &std::path::Path,
    domain: &str,
    items: &mut Vec<serde_json::Value>,
    depth: u32,
) {
    if depth > 4 { return; } // Max recursion depth

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip metadata files
        if name.ends_with(".md") || name.contains("Zone.Identifier") || name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // Check if this is an image/audio collection directory
            let has_media = std::fs::read_dir(&path).ok()
                .map(|rd| rd.flatten().any(|e| {
                    let n = e.file_name().to_string_lossy().to_lowercase();
                    n.ends_with(".jpg") || n.ends_with(".png") || n.ends_with(".wav")
                        || n.ends_with(".tif") || n.ends_with(".tiff")
                }))
                .unwrap_or(false);

            if has_media {
                // Count files and size
                let (count, size) = dir_stats(&path);
                items.push(json!({
                    "name": name,
                    "domain": domain,
                    "file_type": "directory",
                    "description": format!("{count} files"),
                    "file_size_bytes": size,
                    "file_size": format_size(size),
                    "path": path.to_string_lossy(),
                    "compressed": false,
                }));
            } else {
                // Recurse into subdirectories
                collect_catalog_files(&path, domain, items, depth + 1);
            }
        } else {
            let ext = path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            match ext.as_str() {
                "csv" | "json" | "jsonl" | "parquet" | "xlsx" | "xls" | "npz" | "mat" | "txt" => {
                    let size = path.metadata().map(|m| m.len()).unwrap_or(0);

                    // Read metadata from DATASET_INFO.md
                    let info_path = dir.join(format!("{name}.DATASET_INFO.md"));
                    let info = if info_path.exists() {
                        parse_dataset_info(&info_path)
                    } else {
                        DatasetInfoParsed { rows: None, cols: None, source: None, description: None, schema_preview: None }
                    };

                    items.push(json!({
                        "name": name.strip_suffix(&format!(".{ext}")).unwrap_or(&name),
                        "filename": name,
                        "domain": domain,
                        "file_type": ext,
                        "file_size_bytes": size,
                        "file_size": format_size(size),
                        "path": path.to_string_lossy(),
                        "row_count": info.rows,
                        "column_count": info.cols,
                        "source": info.source,
                        "description": info.description,
                        "columns_preview": info.schema_preview,
                        "compressed": false,
                    }));
                }
                _ => {}
            }
        }
    }
}

fn dir_stats(dir: &std::path::Path) -> (u64, u64) {
    let mut count = 0u64;
    let mut size = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if !name.ends_with(".md") && !name.contains("zone.identifier") {
                    count += 1;
                    size += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
    }
    (count, size)
}

struct DatasetInfoParsed {
    rows: Option<u64>,
    cols: Option<u64>,
    source: Option<String>,
    description: Option<String>,
    schema_preview: Option<String>,
}

fn parse_dataset_info(path: &std::path::Path) -> DatasetInfoParsed {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return DatasetInfoParsed { rows: None, cols: None, source: None, description: None, schema_preview: None },
    };
    let mut rows = None;
    let mut cols = None;
    let mut source = None;
    let mut description = None;
    let mut in_source = false;
    let mut in_schema = false;
    let mut schema_lines = Vec::new();

    for line in content.lines() {
        // Rows x columns
        if line.contains("rows") && line.contains("columns") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, p) in parts.iter().enumerate() {
                if *p == "rows" && i > 0 {
                    rows = parts[i-1].replace(",", "").parse().ok();
                }
                if *p == "columns" && i > 1 {
                    cols = parts[i-2].replace("x", "").trim().parse().ok();
                }
            }
        }
        // Source section
        if line.starts_with("## Source") {
            in_source = true;
            in_schema = false;
            continue;
        }
        if line.starts_with("## Schema") || line.starts_with("## Contents") {
            in_source = false;
            // Use the source line as a brief description if we have one
        }
        if in_source && !line.is_empty() && !line.starts_with("##") && source.is_none() {
            let src = line.trim().to_string();
            // Strip third-party platform names from source descriptions
            let src = src.replace("Kaggle", "").replace("kaggle", "")
                .replace("()", "").replace("( )", "").trim().to_string();
            if !src.is_empty() && src != "-" {
                source = Some(src);
            }
            in_source = false;
        }
        // Schema table
        if line.starts_with("## Schema") {
            in_schema = true;
            continue;
        }
        if in_schema && line.starts_with('|') && !line.contains("---") && schema_lines.len() < 6 {
            schema_lines.push(line.trim().to_string());
        }
        if in_schema && line.starts_with("##") {
            in_schema = false;
        }
    }

    // Build description from source + schema preview
    if let Some(ref src) = source {
        description = Some(src.clone());
    }
    let schema_preview = if schema_lines.len() > 1 {
        // Skip header row, take column names
        let col_names: Vec<String> = schema_lines[1..].iter().filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() > 1 { Some(parts[1].trim().replace('`', "")) } else { None }
        }).collect();
        if !col_names.is_empty() {
            Some(col_names.join(", "))
        } else {
            None
        }
    } else {
        None
    };

    DatasetInfoParsed { rows, cols, source, description, schema_preview }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Import a catalog dataset — copies from /opt/datasets/ into user's datasets.
/// Keeps it compressed (.ozl or .zst) until training time.
pub async fn import_catalog_dataset(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let source_path = body.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'path' field".into()))?;

    let name = body.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("imported_dataset");

    let domain = body.get("domain")
        .and_then(|v| v.as_str())
        .unwrap_or("general");

    // Security: ensure path is within /opt/datasets/
    let canonical = std::fs::canonicalize(source_path)
        .map_err(|e| AppError::BadRequest(format!("Invalid path: {e}")))?;
    if !canonical.starts_with("/opt/datasets") {
        return Err(AppError::BadRequest("Path must be within /opt/datasets/".into()));
    }

    let dataset_id = format!("ds_{}", &Uuid::new_v4().to_string()[..8]);
    let dest_dir = format!("{}/datasets", state.config.data_dir);
    let _ = tokio::fs::create_dir_all(&dest_dir).await;

    let file_type = body.get("file_type")
        .and_then(|v| v.as_str())
        .unwrap_or("csv");

    let (dest_path, file_size, row_count, columns) = if file_type == "directory" {
        // Archive the directory with tar + zstd compression
        let tar_path = format!("{dest_dir}/{dataset_id}.tar.zst");

        // Create tar archive in memory, then zstd compress
        let source = canonical.to_string_lossy().to_string();
        let tar_tmp = format!("{dest_dir}/{dataset_id}.tar");

        let output = tokio::process::Command::new("tar")
            .args(["cf", &tar_tmp, "-C", &source, "."])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("tar failed: {e}")))?;

        if !output.status.success() {
            return Err(AppError::Internal("tar archive creation failed".into()));
        }

        // Compress with zstd
        let raw = tokio::fs::read(&tar_tmp).await
            .map_err(|e| AppError::Internal(format!("Cannot read tar: {e}")))?;
        let compressed = zstd::encode_all(raw.as_slice(), 15)
            .map_err(|e| AppError::Internal(format!("zstd compress failed: {e}")))?;
        tokio::fs::write(&tar_path, &compressed).await
            .map_err(|e| AppError::Internal(format!("Cannot write compressed: {e}")))?;
        let _ = tokio::fs::remove_file(&tar_tmp).await;

        let file_size = compressed.len() as u64;
        let (count, _) = dir_stats(&canonical);
        (tar_path, file_size, count, vec!["archive".to_string()])
    } else {
        // Copy the file and optionally compress
        let ext = canonical.extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "csv".to_string());
        let dest_path = format!("{dest_dir}/{dataset_id}.{ext}.zst");

        let raw = tokio::fs::read(&canonical).await
            .map_err(|e| AppError::Internal(format!("Cannot read source: {e}")))?;
        let original_size = raw.len();

        // Compute CSV stats before compressing
        let (cols, rows, _stats): (Vec<String>, usize, serde_json::Map<String, serde_json::Value>) = if ext == "csv" {
            compute_csv_stats(&raw)
        } else {
            (vec![], 0, serde_json::Map::new())
        };

        // Compress with zstd for storage
        let compressed = zstd::encode_all(raw.as_slice(), 15)
            .map_err(|e| AppError::Internal(format!("zstd compress failed: {e}")))?;
        tokio::fs::write(&dest_path, &compressed).await
            .map_err(|e| AppError::Internal(format!("Cannot write: {e}")))?;

        (dest_path, original_size as u64, rows as u64, cols)
    };

    // Create Aegis-DB document
    let doc = json!({
        "name": name,
        "domain": domain,
        "source": "catalog",
        "source_path": source_path,
        "file_path": dest_path,
        "file_type": file_type,
        "file_size_bytes": file_size,
        "row_count": row_count,
        "columns": columns,
        "compressed": true,
        "created_by": auth.user_id,
        "created_at": Utc::now().to_rfc3339(),
        "status": "active",
    });

    state.aegis_create_doc("datasets", json!({
        "id": dataset_id,
        "document": doc,
    })).await?;

    Ok(Json(json!({
        "status": "imported",
        "dataset_id": dataset_id,
        "name": name,
        "domain": domain,
        "compressed": true,
        "file_size_bytes": file_size,
    })))
}

// ---------------------------------------------------------------------------
// Saved Connections — reusable encrypted data source configurations
// ---------------------------------------------------------------------------

/// Save a connection config for future reuse.
/// Sensitive fields are encrypted with the user's vault key.
pub async fn save_connection(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let conn_name = body.get("name").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("name required".into()))?;
    let source_type = body.get("source_type").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("source_type required".into()))?;
    let config = body.get("config")
        .ok_or_else(|| AppError::BadRequest("config required".into()))?;

    let conn_id = format!("conn_{}", &Uuid::new_v4().to_string()[..8]);
    let encrypted_config = prometheus_shield::credential_vault::encrypt_source_config(
        config, &auth.user_id,
    );

    let doc = json!({
        "id": conn_id,
        "name": conn_name,
        "source_type": source_type,
        "config": encrypted_config,
        "created_at": Utc::now().to_rfc3339(),
        "created_by": auth.user_id,
    });

    state.aegis_create_doc("connections", doc).await?;

    Ok(Json(json!({
        "id": conn_id,
        "name": conn_name,
        "source_type": source_type,
    })))
}

/// List saved connections for the authenticated user (credentials redacted).
pub async fn list_connections(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let docs = state.aegis_list_docs("connections").await?;
    let user_conns: Vec<serde_json::Value> = docs.into_iter()
        .filter(|d| d.get("created_by").and_then(|v| v.as_str()) == Some(&auth.user_id))
        .map(|mut d| {
            if let Some(cfg) = d.get("config").cloned() {
                if let Some(obj) = d.as_object_mut() {
                    obj.insert(
                        "config".to_string(),
                        prometheus_shield::credential_vault::redact_source_config(&cfg),
                    );
                }
            }
            d
        })
        .collect();
    Ok(Json(user_conns))
}

/// Delete a saved connection.
pub async fn delete_connection(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("connections", &id).await?;
    if doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) && !auth.is_admin() {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    state.aegis_delete_doc("connections", &id).await?;
    Ok(Json(json!({ "deleted": id })))
}

/// Use a saved connection to import data — decrypts credentials server-side.
pub async fn use_connection(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Extension(shield): Extension<Arc<Shield>>,
    Path(id): Path<String>,
    Json(overrides): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let conn_doc = state.aegis_get_doc("connections", &id).await?;
    if conn_doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) && !auth.is_admin() {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let source_type = conn_doc.get("source_type").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("Saved connection missing source_type".into()))?
        .to_string();
    let encrypted_config = conn_doc.get("config")
        .ok_or_else(|| AppError::Internal("Saved connection missing config".into()))?;

    // Decrypt credentials server-side
    let mut decrypted = prometheus_shield::credential_vault::decrypt_source_config(
        encrypted_config, &auth.user_id,
    ).map_err(|e| AppError::Internal(format!("Failed to decrypt connection: {e}")))?;

    // Apply any overrides (e.g., different query, limit)
    if let (Some(base), Some(patch)) = (decrypted.as_object_mut(), overrides.as_object()) {
        for (k, v) in patch {
            base.insert(k.clone(), v.clone());
        }
    }

    // Add source_type for the dispatcher
    if let Some(obj) = decrypted.as_object_mut() {
        obj.insert("source_type".to_string(), json!(source_type));
    }

    // Route through connect_source dispatcher
    connect_source(
        State(state),
        Extension(auth),
        Extension(shield),
        Json(decrypted),
    ).await
}
