// ============================================================================
// File: health.rs
// Description: Health check and system metrics endpoints for monitoring and readiness probes
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{extract::State, Json};
use chrono::Utc;
use serde_json::json;
use crate::state::AppState;

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "service": "prometheus-server",
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

pub async fn system_metrics(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    // Read real system metrics from /proc on Linux
    let cpu_usage = read_cpu_usage().await;
    let (mem_used, mem_total) = read_memory().await;
    let (disk_used, disk_total) = read_disk().await;
    let uptime = read_uptime().await;

    // Check Aegis-DB connectivity
    let aegis_status = match state
        .http_client
        .get(format!("{}/health", state.config.aegis_db_url))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => "connected",
        _ => "disconnected",
    };

    Json(json!({
        "cpu_usage_percent": cpu_usage,
        "memory_used_mb": mem_used,
        "memory_total_mb": mem_total,
        "disk_used_gb": disk_used,
        "disk_total_gb": disk_total,
        "uptime_seconds": uptime,
        "aegis_db_status": aegis_status,
        "active_trainings": state.active_trainings.read().await.len(),
        "max_concurrent_trainings": state.config.max_concurrent_trainings,
        "stripe_enabled": state.config.stripe_enabled(),
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

async fn read_cpu_usage() -> f64 {
    // Read /proc/stat for CPU usage
    match tokio::fs::read_to_string("/proc/loadavg").await {
        Ok(content) => content
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0),
        Err(_) => 0.0,
    }
}

async fn read_memory() -> (u64, u64) {
    match tokio::fs::read_to_string("/proc/meminfo").await {
        Ok(content) => {
            let mut total_kb = 0u64;
            let mut available_kb = 0u64;
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line.split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    available_kb = line.split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                }
            }
            let total_mb = total_kb / 1024;
            let used_mb = total_mb.saturating_sub(available_kb / 1024);
            (used_mb, total_mb)
        }
        Err(_) => (0, 0),
    }
}

async fn read_disk() -> (u64, u64) {
    // Use statvfs via nix or fallback to df parsing
    match tokio::process::Command::new("df")
        .args(["-BG", "/"])
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().nth(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let total = parts[1].trim_end_matches('G').parse::<u64>().unwrap_or(0);
                    let used = parts[2].trim_end_matches('G').parse::<u64>().unwrap_or(0);
                    return (used, total);
                }
            }
            (0, 0)
        }
        Err(_) => (0, 0),
    }
}

async fn read_uptime() -> u64 {
    match tokio::fs::read_to_string("/proc/uptime").await {
        Ok(content) => content
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<f64>().ok())
            .map(|v| v as u64)
            .unwrap_or(0),
        Err(_) => 0,
    }
}
