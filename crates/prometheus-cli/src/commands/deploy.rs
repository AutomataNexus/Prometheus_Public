// ============================================================================
// File: deploy.rs
// Description: CLI command for deploying trained models to edge devices
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use crate::{api::ApiClient, theme};
use serde_json::json;

pub async fn deploy(client: &ApiClient, model_id: &str, target: Option<&str>) -> anyhow::Result<()> {
    theme::print_info(&format!("Deploying model {model_id}..."));

    let body = json!({
        "model_id": model_id,
        "target": target.unwrap_or("armv7-unknown-linux-musleabihf"),
        "optimize": true,
    });

    let resp = client.post("/api/v1/deployments", body).await?;
    let dep_id = resp.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

    theme::print_success(&format!("Deployment created: {}", theme::styled_id(dep_id)));

    if let Some(status) = resp.get("status").and_then(|v| v.as_str()) {
        println!("{}", theme::styled_label("status", status));
    }

    Ok(())
}
