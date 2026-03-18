// ============================================================================
// File: auth.rs
// Description: CLI authentication flow with QR code and browser-based verification
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! CLI authentication: credentials + browser verification.
//!
//! Flow:
//! 1. User enters username/email + password in terminal
//! 2. CLI authenticates against server, receives token
//! 3. CLI creates a verification session, displays QR code + URL
//! 4. User opens /auth/verify?code=XXX in browser to confirm
//! 5. CLI polls until browser verification completes, saves token

use crate::{config::Config, theme};
use anyhow::Result;
use qrcode::QrCode;
use uuid::Uuid;

pub async fn login_flow(cfg: &Config) -> Result<()> {
    let client = reqwest::Client::new();

    // ── Step 1: Prompt for credentials ──────────────────────
    println!();
    println!("{}", theme::styled_header("Sign In to Prometheus"));
    println!();

    let username: String = dialoguer::Input::new()
        .with_prompt("  Username or email")
        .interact_text()?;

    if username.is_empty() {
        theme::print_error("Username cannot be empty.");
        return Ok(());
    }

    let password: String = dialoguer::Password::new()
        .with_prompt("  Password")
        .interact()?;

    if password.is_empty() {
        theme::print_error("Password cannot be empty.");
        return Ok(());
    }

    // ── Step 2: Authenticate with server ────────────────────
    theme::print_info("Authenticating...");

    let login_resp = client
        .post(format!("{}/api/v1/auth/login", cfg.server_url))
        .json(&serde_json::json!({
            "username": username,
            "password": password,
        }))
        .send()
        .await;

    let (token, user_display, role) = match login_resp {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let token = body.get("token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("No token in login response"))?
                .to_string();
            let uname = body.pointer("/user/username")
                .and_then(|v| v.as_str())
                .unwrap_or(&username)
                .to_string();
            let role = body.pointer("/user/role")
                .and_then(|v| v.as_str())
                .unwrap_or("operator")
                .to_string();
            (token, uname, role)
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
                .unwrap_or_else(|| format!("HTTP {status}"));
            theme::print_error(&format!("Login failed: {msg}"));
            return Ok(());
        }
        Err(e) => {
            theme::print_error(&format!("Cannot reach server: {e}"));
            return Ok(());
        }
    };

    theme::print_success(&format!("Credentials verified for {user_display}"));

    // ── Step 3: Create CLI verification session ─────────────
    let session_code = format!("cli_{}", &Uuid::new_v4().to_string()[..12]);

    let init_resp = client
        .post(format!("{}/api/v1/auth/cli/init", cfg.server_url))
        .json(&serde_json::json!({ "session_code": session_code }))
        .send()
        .await;

    let verify_url = match init_resp {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            body.get("verify_url")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("{}/auth/verify?code={}", cfg.server_url, session_code))
        }
        _ => format!("{}/auth/verify?code={}", cfg.server_url, session_code),
    };

    // ── Step 4: Display QR code and URL ─────────────────────
    println!();
    println!("{}", theme::styled_header("Verify in Browser"));
    println!();
    println!("  Scan the QR code or open the URL to confirm this CLI session.");
    println!();

    match QrCode::new(verify_url.as_bytes()) {
        Ok(code) => {
            let string = code
                .render::<char>()
                .quiet_zone(true)
                .module_dimensions(2, 1)
                .build();
            for line in string.lines() {
                println!("    {line}");
            }
        }
        Err(_) => {
            theme::print_warning("Could not generate QR code.");
        }
    }

    println!();
    println!("  \x1b[1mVerification URL:\x1b[0m");
    println!("  \x1b[38;2;20;184;166m{verify_url}\x1b[0m");
    println!();
    println!("  \x1b[2mCode: {session_code}\x1b[0m");
    println!();

    // Try to open browser automatically
    let _ = open::that(&verify_url);

    // ── Step 5: Poll for browser verification ───────────────
    theme::print_info("Waiting for browser verification...");
    let poll_url = format!("{}/api/v1/auth/cli/poll", cfg.server_url);

    let spinner_frames = [
        '\u{280B}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283C}', '\u{2834}', '\u{2826}',
        '\u{2827}', '\u{2807}', '\u{280F}',
    ];
    let mut frame = 0;

    for attempt in 0..120 {
        print!(
            "\r  \x1b[38;2;20;184;166m{}\x1b[0m Waiting for browser confirmation... ({}/120s)",
            spinner_frames[frame % spinner_frames.len()],
            attempt
        );
        let _ = std::io::Write::flush(&mut std::io::stdout());
        frame += 1;

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let poll_resp = client
            .get(&poll_url)
            .query(&[("code", &session_code)])
            .send()
            .await;

        if let Ok(resp) = poll_resp {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let status = body.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status == "verified" {
                    println!();
                    Config::save_token(&token)?;
                    println!();
                    theme::print_success(&format!("Authenticated as {user_display} ({role})"));
                    theme::print_info("Session verified. Token saved.");
                    return Ok(());
                }
            }
        }
    }

    println!();
    theme::print_error("Browser verification timed out after 120 seconds.");
    theme::print_info("Your credentials were valid. Try again with: prometheus login");
    Ok(())
}

pub fn logout(cfg: &Config) -> Result<()> {
    Config::clear_token()?;
    theme::print_success("Logged out. Credentials cleared.");
    let _ = cfg; // cfg available for future use (e.g., server logout call)
    Ok(())
}
