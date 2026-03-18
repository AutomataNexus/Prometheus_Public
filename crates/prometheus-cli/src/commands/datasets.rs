// ============================================================================
// File: datasets.rs
// Description: CLI commands for listing, uploading, and managing datasets
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use crate::{api::ApiClient, theme};
use serde_json::json;

pub async fn list_datasets(client: &ApiClient) -> anyhow::Result<()> {
    let datasets = client.get("/api/v1/datasets").await?;
    let empty = vec![];
    let arr = datasets.as_array().unwrap_or(&empty);

    println!("{}", theme::styled_header(&format!("Datasets ({})", arr.len())));

    if arr.is_empty() {
        theme::print_info("No datasets found. Upload one with /upload <file>");
        return Ok(());
    }

    println!("{}", theme::table_header(&[("ID", 14), ("Name", 30), ("Rows", 8), ("Features", 8), ("Status", 10)]));

    for ds in arr {
        let id = ds.get("id").and_then(|v| v.as_str()).unwrap_or("--");
        let name = ds.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed");
        let rows = ds.get("row_count").and_then(|v| v.as_u64()).map(|v| v.to_string()).unwrap_or_else(|| "--".into());
        let features = ds.get("columns").and_then(|v| v.as_array()).map(|a| a.len().to_string()).unwrap_or_else(|| "--".into());
        let status = ds.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");

        println!("  {}  {:<30}  {:>8}  {:>8}  {}",
            theme::styled_id(id),
            name,
            rows,
            features,
            theme::styled_status(status),
        );
    }
    Ok(())
}

pub async fn get_dataset(client: &ApiClient, id: &str) -> anyhow::Result<()> {
    let ds = client.get(&format!("/api/v1/datasets/{id}")).await?;

    println!("{}", theme::styled_header("Dataset Details"));
    println!("{}", theme::styled_label("id", ds.get("id").and_then(|v| v.as_str()).unwrap_or(id)));
    println!("{}", theme::styled_label("name", ds.get("name").and_then(|v| v.as_str()).unwrap_or("--")));
    println!("{}", theme::styled_label("rows", &ds.get("row_count").and_then(|v| v.as_u64()).map(|v| v.to_string()).unwrap_or_else(|| "--".into())));
    println!("{}", theme::styled_label("status", ds.get("status").and_then(|v| v.as_str()).unwrap_or("--")));

    let validated = ds.get("is_validated").and_then(|v| v.as_bool()).unwrap_or(false);
    let locked = ds.get("locked").and_then(|v| v.as_bool()).unwrap_or(false);
    if validated {
        println!("{}", theme::styled_label("validated", &theme::styled_status("ready")));
    } else {
        println!("{}", theme::styled_label("validated", "no — run /validate <id> before training"));
    }
    if locked {
        println!("{}", theme::styled_label("locked", "yes (compressed)"));
    }

    if let Some(cols) = ds.get("columns").and_then(|v| v.as_array()) {
        let col_names: Vec<&str> = cols.iter().filter_map(|c| c.as_str()).collect();
        println!("{}", theme::styled_label("columns", &col_names.join(", ")));
    }

    if let Some(created) = ds.get("created_at").and_then(|v| v.as_str()) {
        println!("{}", theme::styled_label("created", created));
    }

    Ok(())
}

pub async fn upload_dataset(client: &ApiClient, file_path: &str, name: Option<&str>) -> anyhow::Result<()> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        theme::print_error(&format!("File not found: {file_path}"));
        return Ok(());
    }

    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("dataset.csv")
        .to_string();

    let display_name = name.unwrap_or(&file_name);
    theme::print_info(&format!("Uploading {display_name}..."));

    let file_bytes = tokio::fs::read(file_path).await?;
    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name.clone())
        .mime_str("text/csv")?;

    let mut form = reqwest::multipart::Form::new().part("file", part);
    if let Some(n) = name {
        form = form.text("name", n.to_string());
    }

    let resp = client.post_multipart("/api/v1/datasets", form).await?;

    let id = resp.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    theme::print_success(&format!("Dataset uploaded: {}", theme::styled_id(id)));
    Ok(())
}

pub async fn validate_dataset(client: &ApiClient, id: &str) -> anyhow::Result<()> {
    theme::print_info(&format!("Validating dataset {id}..."));

    let resp = client.post(&format!("/api/v1/datasets/{id}/validate"), json!({})).await?;
    let valid = resp.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);

    if valid {
        let rows = resp.get("rows_checked").and_then(|v| v.as_u64()).unwrap_or(0);
        let cols = resp.get("columns_checked").and_then(|v| v.as_u64()).unwrap_or(0);
        theme::print_success(&format!("Dataset validated ({rows} rows, {cols} columns) — locked for training"));
    } else {
        theme::print_error("Validation failed:");
        if let Some(issues) = resp.get("issues").and_then(|v| v.as_array()) {
            for issue in issues {
                if let Some(msg) = issue.as_str() {
                    println!("    {msg}");
                }
            }
        }
    }
    Ok(())
}

pub async fn unlock_dataset(client: &ApiClient, id: &str) -> anyhow::Result<()> {
    client.post(&format!("/api/v1/datasets/{id}/unlock"), json!({})).await?;
    theme::print_success(&format!("Dataset {id} unlocked — will require re-validation before training"));
    Ok(())
}
