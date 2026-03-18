// ============================================================================
// File: data_lifecycle.rs
// Description: Automated dataset retention, compression, and cleanup based on subscription tier
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use std::path::Path;
use chrono::{Utc, Duration};
use serde_json::json;
use tracing;
use crate::state::AppState;

/// Compression level for inactive datasets (1-22, higher = smaller but slower).
/// Level 15 is a good balance for background compression.
const ZSTD_LEVEL: i32 = 15;

/// Free users: delete datasets after this many days of inactivity.
const FREE_RETENTION_DAYS: i64 = 30;

/// Paid users: compress datasets after this many hours of session inactivity.
const COMPRESS_AFTER_HOURS: i64 = 24;

/// Compress a dataset file in-place using zstd.
/// Renames `file.csv` → `file.csv.zst` and updates the Aegis-DB doc.
pub async fn compress_dataset(state: &AppState, dataset_id: &str) -> anyhow::Result<()> {
    let doc = state.aegis_get_doc("datasets", dataset_id).await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let file_path = doc.get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No file_path"))?;

    // Already compressed
    if file_path.ends_with(".zst") {
        return Ok(());
    }

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(anyhow::anyhow!("File not found: {file_path}"));
    }

    let raw_data = tokio::fs::read(file_path).await?;
    let original_size = raw_data.len();

    // Compress with zstd
    let compressed = zstd::encode_all(raw_data.as_slice(), ZSTD_LEVEL)?;
    let compressed_size = compressed.len();

    let zst_path = format!("{}.zst", file_path);
    tokio::fs::write(&zst_path, &compressed).await?;
    tokio::fs::remove_file(file_path).await?;

    let ratio = if original_size > 0 {
        (1.0 - compressed_size as f64 / original_size as f64) * 100.0
    } else {
        0.0
    };

    // Update doc with compressed path and metadata
    let _ = state.aegis_update_doc("datasets", dataset_id, json!({
        "file_path": zst_path,
        "compressed": true,
        "original_size_bytes": original_size,
        "compressed_size_bytes": compressed_size,
        "compression_ratio": format!("{:.1}%", ratio),
        "compressed_at": Utc::now().to_rfc3339(),
    })).await;

    tracing::info!(
        "Compressed dataset {}: {} → {} ({:.1}% reduction)",
        dataset_id, original_size, compressed_size, ratio
    );

    Ok(())
}

/// Decompress a dataset file on-demand.
/// Renames `file.csv.zst` → `file.csv` and updates the doc.
pub async fn decompress_dataset(state: &AppState, dataset_id: &str) -> anyhow::Result<String> {
    let doc = state.aegis_get_doc("datasets", dataset_id).await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let file_path = doc.get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No file_path"))?;

    // Not compressed — return as-is
    if !file_path.ends_with(".zst") {
        return Ok(file_path.to_string());
    }

    let compressed_data = tokio::fs::read(file_path).await?;
    let raw_data = zstd::decode_all(compressed_data.as_slice())?;

    let original_path = file_path.strip_suffix(".zst").unwrap().to_string();
    tokio::fs::write(&original_path, &raw_data).await?;
    tokio::fs::remove_file(file_path).await?;

    // Update doc back to uncompressed state
    let _ = state.aegis_update_doc("datasets", dataset_id, json!({
        "file_path": original_path,
        "compressed": false,
        "decompressed_at": Utc::now().to_rfc3339(),
    })).await;

    tracing::info!("Decompressed dataset {}: {}", dataset_id, original_path);

    Ok(original_path)
}

/// Read dataset file contents, decompressing in-memory if needed (no disk write).
/// Use this for previews and training reads — doesn't permanently decompress.
pub async fn read_dataset_bytes(file_path: &str) -> anyhow::Result<Vec<u8>> {
    let data = tokio::fs::read(file_path).await?;
    if file_path.ends_with(".zst") {
        Ok(zstd::decode_all(data.as_slice())?)
    } else {
        Ok(data)
    }
}

/// Background lifecycle task — runs periodically to:
/// 1. Compress inactive datasets for paid/admin users
/// 2. Delete old datasets for free users
pub async fn run_lifecycle_sweep(state: AppState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await; // every hour

        if let Err(e) = sweep_once(&state).await {
            tracing::error!("Data lifecycle sweep error: {e}");
        }
    }
}

async fn sweep_once(state: &AppState) -> anyhow::Result<()> {
    let datasets = state.aegis_list_docs("datasets").await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let now = Utc::now();
    let mut compressed_count = 0u32;
    let mut deleted_count = 0u32;
    let mut bytes_saved = 0u64;

    for ds in &datasets {
        let ds_id = match ds.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };

        let file_path = match ds.get("file_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => continue,
        };

        let already_compressed = ds.get("compressed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Parse last activity time (updated_at or created_at)
        let last_activity = ds.get("updated_at")
            .or_else(|| ds.get("created_at"))
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let last_activity = match last_activity {
            Some(t) => t,
            None => continue,
        };

        // Get user tier
        let user_id = ds.get("created_by").and_then(|v| v.as_str()).unwrap_or("");
        if user_id.is_empty() {
            continue;
        }

        let tier = crate::api::billing::get_user_tier(state, user_id).await;
        let is_free = matches!(tier, crate::api::billing::SubscriptionTier::Free);

        if is_free {
            // Free users: delete after retention period
            let age = now - last_activity;
            if age > Duration::days(FREE_RETENTION_DAYS) {
                // Delete the file
                let _ = tokio::fs::remove_file(file_path).await;
                let _ = state.aegis_delete_doc("datasets", ds_id).await;
                deleted_count += 1;
                tracing::info!(
                    "Deleted inactive free-tier dataset {} (age: {}d)",
                    ds_id, age.num_days()
                );
            }
        } else if !already_compressed {
            // Paid/admin users: compress after inactivity period
            let idle = now - last_activity;
            if idle > Duration::hours(COMPRESS_AFTER_HOURS) {
                let original_size = ds.get("file_size_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                match compress_dataset(state, ds_id).await {
                    Ok(()) => {
                        compressed_count += 1;
                        let compressed_size = state.aegis_get_doc("datasets", ds_id).await
                            .ok()
                            .and_then(|d| d.get("compressed_size_bytes").and_then(|v| v.as_u64()))
                            .unwrap_or(original_size);
                        bytes_saved += original_size.saturating_sub(compressed_size);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to compress dataset {}: {}", ds_id, e);
                    }
                }
            }
        }
    }

    if compressed_count > 0 || deleted_count > 0 {
        tracing::info!(
            "Lifecycle sweep: compressed={}, deleted={}, bytes_saved={}",
            compressed_count, deleted_count, bytes_saved
        );
    }

    Ok(())
}
