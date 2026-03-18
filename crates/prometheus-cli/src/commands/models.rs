// ============================================================================
// File: models.rs
// Description: CLI commands for listing and inspecting trained models
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use crate::{api::ApiClient, theme};

pub async fn list_models(client: &ApiClient) -> anyhow::Result<()> {
    let models = client.get("/api/v1/models").await?;
    let empty = vec![];
    let arr = models.as_array().unwrap_or(&empty);

    println!("{}", theme::styled_header(&format!("Models ({})", arr.len())));

    if arr.is_empty() {
        theme::print_info("No models found. Start training with /train <dataset_id>");
        return Ok(());
    }

    println!("{}", theme::table_header(&[("ID", 14), ("Architecture", 18), ("F1", 6), ("Val Loss", 10), ("Size", 10), ("Status", 10)]));

    for m in arr {
        let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("--");
        let arch = m.get("architecture").and_then(|v| v.as_str()).unwrap_or("--");
        let f1 = m.get("metrics").and_then(|v| v.get("f1")).and_then(|v| v.as_f64())
            .map(|v| format!("{v:.3}")).unwrap_or_else(|| "--".into());
        let val_loss = m.get("metrics").and_then(|v| v.get("val_loss")).and_then(|v| v.as_f64())
            .map(|v| format!("{v:.6}")).unwrap_or_else(|| "--".into());
        let size = m.get("file_size_bytes").and_then(|v| v.as_u64())
            .map(|b| format_bytes(b)).unwrap_or_else(|| "--".into());
        let status = m.get("status").and_then(|v| v.as_str()).unwrap_or("--");

        println!("  {}  {:<18}  {:>6}  {:>10}  {:>10}  {}",
            theme::styled_id(id), arch, f1, val_loss, size, theme::styled_status(status));
    }
    Ok(())
}

pub async fn get_model(client: &ApiClient, id: &str) -> anyhow::Result<()> {
    let m = client.get(&format!("/api/v1/models/{id}")).await?;

    println!("{}", theme::styled_header("Model Details"));
    println!("{}", theme::styled_label("id", m.get("id").and_then(|v| v.as_str()).unwrap_or(id)));
    println!("{}", theme::styled_label("name", m.get("name").and_then(|v| v.as_str()).unwrap_or("--")));
    println!("{}", theme::styled_label("architecture", m.get("architecture").and_then(|v| v.as_str()).unwrap_or("--")));
    println!("{}", theme::styled_label("dataset_id", m.get("dataset_id").and_then(|v| v.as_str()).unwrap_or("--")));
    println!("{}", theme::styled_label("status", m.get("status").and_then(|v| v.as_str()).unwrap_or("--")));

    if let Some(metrics) = m.get("metrics") {
        println!();
        println!("  {}", theme::styled_header("Metrics"));
        for key in ["precision", "recall", "f1", "val_loss", "train_loss"] {
            if let Some(v) = metrics.get(key).and_then(|v| v.as_f64()) {
                println!("{}", theme::styled_label(key, &format!("{v:.6}")));
            }
        }
    }

    if let Some(size) = m.get("file_size_bytes").and_then(|v| v.as_u64()) {
        println!("{}", theme::styled_label("size", &format_bytes(size)));
    }
    if let Some(created) = m.get("created_at").and_then(|v| v.as_str()) {
        println!("{}", theme::styled_label("created", created));
    }
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
