// ============================================================================
// File: mod.rs
// Description: CLI command module re-exports and configuration command handler
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! CLI command implementations.

mod datasets;
mod models;
mod training;
mod deploy;
mod agent;

pub use datasets::*;
pub use models::*;
pub use training::*;
pub use deploy::*;
pub use agent::*;

use crate::{api::ApiClient, config::Config, theme};

pub fn config_cmd(cfg: &Config, key: Option<&str>, value: Option<&str>) -> anyhow::Result<()> {
    match (key, value) {
        (None, _) => {
            println!("{}", theme::styled_header("Configuration"));
            println!("{}", theme::styled_label("server_url", &cfg.server_url));
            println!("{}", theme::styled_label("data_dir", &cfg.data_dir));
            println!("{}", theme::styled_label("credentials",
                &if cfg.load_token().is_some() { "stored" } else { "none" }.to_string()));
            if let Some(path) = Config::config_path() {
                println!("{}", theme::styled_label("config_file", &path.display().to_string()));
            }
            Ok(())
        }
        (Some(key), Some(value)) => {
            let mut cfg = cfg.clone();
            match key {
                "server_url" => cfg.server_url = value.to_string(),
                "data_dir" => cfg.data_dir = value.to_string(),
                _ => {
                    theme::print_error(&format!("Unknown config key: {key}"));
                    return Ok(());
                }
            }
            cfg.save()?;
            theme::print_success(&format!("Set {key} = {value}"));
            Ok(())
        }
        (Some(key), None) => {
            match key {
                "server_url" => println!("{}", cfg.server_url),
                "data_dir" => println!("{}", cfg.data_dir),
                _ => theme::print_error(&format!("Unknown config key: {key}")),
            }
            Ok(())
        }
    }
}

pub async fn health(client: &ApiClient) -> anyhow::Result<()> {
    match client.get("/health").await {
        Ok(resp) => {
            let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
            if status == "ok" {
                theme::print_success(&format!("Server is healthy ({})", status));
            } else {
                theme::print_warning(&format!("Server status: {status}"));
            }
            if let Some(version) = resp.get("version").and_then(|v| v.as_str()) {
                println!("{}", theme::styled_label("version", version));
            }
        }
        Err(e) => {
            theme::print_error(&format!("Cannot reach server: {e}"));
        }
    }
    Ok(())
}

pub async fn whoami(client: &ApiClient) -> anyhow::Result<()> {
    if client.token().is_none() {
        theme::print_warning("Not authenticated. Run /login first.");
        return Ok(());
    }
    match client.get("/api/v1/auth/me").await {
        Ok(user) => {
            println!("{}", theme::styled_header("Current User"));
            if let Some(username) = user.get("username").and_then(|v| v.as_str()) {
                println!("{}", theme::styled_label("username", username));
            }
            if let Some(role) = user.get("role").and_then(|v| v.as_str()) {
                println!("{}", theme::styled_label("role", role));
            }
            if let Some(email) = user.get("email").and_then(|v| v.as_str()) {
                println!("{}", theme::styled_label("email", email));
            }
            if let Some(id) = user.get("id").and_then(|v| v.as_str()) {
                println!("{}", theme::styled_label("id", id));
            }
        }
        Err(_) => theme::print_error("Session expired or invalid. Run /login."),
    }
    Ok(())
}
