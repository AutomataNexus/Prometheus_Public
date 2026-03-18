// ============================================================================
// File: training.rs
// Description: CLI commands for managing and monitoring training runs
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use crate::{api::ApiClient, theme};
use serde_json::json;

pub async fn list_training(client: &ApiClient) -> anyhow::Result<()> {
    let runs = client.get("/api/v1/training").await?;
    let empty = vec![];
    let arr = runs.as_array().unwrap_or(&empty);

    println!("{}", theme::styled_header(&format!("Training Runs ({})", arr.len())));

    if arr.is_empty() {
        theme::print_info("No training runs. Start one with /train <dataset_id>");
        return Ok(());
    }

    println!("{}", theme::table_header(&[("ID", 14), ("Architecture", 18), ("Epoch", 10), ("Val Loss", 10), ("Status", 12)]));

    for run in arr {
        let id = run.get("id").and_then(|v| v.as_str()).unwrap_or("--");
        let arch = run.get("architecture").and_then(|v| v.as_str()).unwrap_or("--");
        let epoch = run.get("current_epoch").and_then(|v| v.as_u64())
            .map(|e| {
                let total = run.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(0);
                format!("{e}/{total}")
            })
            .unwrap_or_else(|| "--".into());
        let val_loss = run.get("best_val_loss").and_then(|v| v.as_f64())
            .map(|v| format!("{v:.6}"))
            .unwrap_or_else(|| "--".into());
        let status = run.get("status").and_then(|v| v.as_str()).unwrap_or("--");

        println!("  {}  {:<18}  {:>10}  {:>10}  {}",
            theme::styled_id(id), arch, epoch, val_loss, theme::styled_status(status));
    }
    Ok(())
}

pub async fn start_training(
    client: &ApiClient,
    dataset_id: &str,
    arch: &str,
    lr: Option<f64>,
    epochs: Option<u64>,
    batch_size: Option<u64>,
    hidden_dim: Option<u64>,
) -> anyhow::Result<()> {
    theme::print_info(&format!("Starting {arch} training on dataset {dataset_id}..."));

    let mut hp = json!({});
    if let Some(lr) = lr { hp["learning_rate"] = json!(lr); }
    if let Some(e) = epochs { hp["epochs"] = json!(e); }
    if let Some(bs) = batch_size { hp["batch_size"] = json!(bs); }
    if let Some(hd) = hidden_dim { hp["hidden_dim"] = json!(hd); }

    let body = json!({
        "dataset_id": dataset_id,
        "architecture": arch,
        "hyperparameters": hp,
    });

    let resp = client.post("/api/v1/training/start", body).await?;
    let run_id = resp.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let model_id = resp.get("model_id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("running");

    if status == "queued" {
        theme::print_warning(&format!("Training queued: {}", theme::styled_id(run_id)));
        println!("{}", theme::styled_label("model_id", model_id));
        println!("{}", theme::styled_label("architecture", arch));
        theme::print_info("Server is at max training capacity. You'll be notified when it starts.");
    } else {
        theme::print_success(&format!("Training started: {}", theme::styled_id(run_id)));
        println!("{}", theme::styled_label("model_id", model_id));
        println!("{}", theme::styled_label("architecture", arch));
        theme::print_info(&format!("Monitor with: /monitor {run_id}"));
    }

    Ok(())
}

pub async fn queue_status(client: &ApiClient) -> anyhow::Result<()> {
    let resp = client.get("/api/v1/training/queue").await?;

    println!("{}", theme::styled_header("Training Queue"));
    let active = resp.get("active_trainings").and_then(|v| v.as_u64()).unwrap_or(0);
    let max = resp.get("max_concurrent").and_then(|v| v.as_u64()).unwrap_or(0);
    let queued = resp.get("queued").and_then(|v| v.as_u64()).unwrap_or(0);
    let available = resp.get("capacity_available").and_then(|v| v.as_u64()).unwrap_or(0);

    println!("{}", theme::styled_label("active", &format!("{active}/{max}")));
    println!("{}", theme::styled_label("queued", &queued.to_string()));
    println!("{}", theme::styled_label("available", &available.to_string()));

    if queued > 0 {
        theme::print_warning(&format!("{queued} training run(s) waiting for a slot"));
    } else if active < max {
        theme::print_success("Capacity available for new training runs");
    } else {
        theme::print_warning("Server at max capacity — new runs will be queued");
    }

    Ok(())
}

pub async fn training_status(client: &ApiClient, id: &str) -> anyhow::Result<()> {
    let run = client.get(&format!("/api/v1/training/{id}")).await?;

    println!("{}", theme::styled_header("Training Run"));
    println!("{}", theme::styled_label("id", run.get("id").and_then(|v| v.as_str()).unwrap_or(id)));
    println!("{}", theme::styled_label("architecture", run.get("architecture").and_then(|v| v.as_str()).unwrap_or("--")));
    println!("{}", theme::styled_label("dataset_id", run.get("dataset_id").and_then(|v| v.as_str()).unwrap_or("--")));

    let status = run.get("status").and_then(|v| v.as_str()).unwrap_or("--");
    println!("  {:<18} {}", "\x1b[38;2;196;164;132mstatus\x1b[0m", theme::styled_status(status));

    let epoch = run.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0);
    let total = run.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(0);
    println!("{}", theme::styled_label("progress", &format!("{epoch}/{total} epochs")));

    if let Some(best) = run.get("best_val_loss").and_then(|v| v.as_f64()) {
        println!("{}", theme::styled_label("best_val_loss", &format!("{best:.6}")));
    }

    if status == "running" && total > 0 {
        let pct = (epoch as f64 / total as f64 * 100.0) as u32;
        let bar_len = 40;
        let filled = (bar_len as f64 * epoch as f64 / total as f64) as usize;
        let bar: String = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(bar_len - filled);
        println!("  \x1b[38;2;20;184;166m[{bar}] {pct}%\x1b[0m");
    }

    Ok(())
}

pub async fn stop_training(client: &ApiClient, id: &str) -> anyhow::Result<()> {
    client.post(&format!("/api/v1/training/{id}/stop"), json!({})).await?;
    theme::print_success(&format!("Stop signal sent to training run {id}"));
    Ok(())
}
