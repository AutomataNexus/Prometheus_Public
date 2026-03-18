// ============================================================================
// File: agent.rs
// Description: Interactive chat interface for the PrometheusForge AI agent
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use crate::{api::ApiClient, theme};
use serde_json::json;
use std::io::{self, Write, BufRead};

pub async fn agent_chat(client: &ApiClient, message: Option<&str>) -> anyhow::Result<()> {
    match message {
        Some(msg) => {
            // Single message mode
            send_message(client, msg).await
        }
        None => {
            // Interactive chat mode
            println!("{}", theme::styled_header("PrometheusForge AI Agent"));
            theme::print_info("Type your message. Press Enter to send. Type /back to return.");
            println!();

            let stdin = io::stdin();
            loop {
                print!("\x1b[38;2;196;164;132myou>\x1b[0m ");
                io::stdout().flush()?;

                let mut line = String::new();
                if stdin.lock().read_line(&mut line)? == 0 {
                    break;
                }
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if line == "/back" || line == "/quit" {
                    break;
                }

                send_message(client, line).await?;
                println!();
            }
            Ok(())
        }
    }
}

async fn send_message(client: &ApiClient, msg: &str) -> anyhow::Result<()> {
    let body = json!({
        "message": msg,
    });

    match client.post("/api/v1/agent/chat", body).await {
        Ok(resp) => {
            let reply = resp.get("response")
                .or_else(|| resp.get("reply"))
                .or_else(|| resp.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("No response from PrometheusForge.");

            println!("\x1b[38;2;20;184;166m  forge>\x1b[0m {reply}");
        }
        Err(e) => {
            theme::print_error(&format!("Agent error: {e}"));
        }
    }
    Ok(())
}
