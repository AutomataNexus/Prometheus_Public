// ============================================================================
// File: email.rs
// Description: Email sending endpoints for account lifecycle notifications using prometheus-email
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{Extension, Json};
use std::sync::Arc;
use prometheus_email::EmailService;
use prometheus_shield::Shield;
use serde::Deserialize;
use crate::error::AppResult;

// ---------- Account lifecycle ----------

#[derive(Deserialize)]
pub struct WelcomeRequest {
    pub to: String,
    pub username: String,
}

pub async fn send_welcome(
    Extension(shield): Extension<Arc<Shield>>,
    Extension(email): Extension<Arc<EmailService>>,
    Json(body): Json<WelcomeRequest>,
) -> AppResult<Json<serde_json::Value>> {
    shield.validate_outbound_email(
        &[&body.to],
        "Welcome to Prometheus",
        &[("username", &body.username)],
    )?;
    let id = email.send_welcome(&body.to, &Shield::escape_email_content(&body.username)).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct VerificationRequest {
    pub to: String,
    pub username: String,
    pub code: String,
    #[serde(default = "default_expires")]
    pub expires_minutes: u32,
}

fn default_expires() -> u32 { 15 }

pub async fn send_verification(
    Extension(shield): Extension<Arc<Shield>>,
    Extension(email): Extension<Arc<EmailService>>,
    Json(body): Json<VerificationRequest>,
) -> AppResult<Json<serde_json::Value>> {
    shield.validate_outbound_email(
        &[&body.to],
        "Verify Your Email",
        &[("username", &body.username), ("code", &body.code)],
    )?;
    let id = email
        .send_verification(
            &body.to,
            &Shield::escape_email_content(&body.username),
            &Shield::escape_email_content(&body.code),
            body.expires_minutes,
        )
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct PasswordResetRequest {
    pub to: String,
    pub username: String,
    pub reset_token: String,
    #[serde(default = "default_expires")]
    pub expires_minutes: u32,
}

pub async fn send_password_reset(
    Extension(shield): Extension<Arc<Shield>>,
    Extension(email): Extension<Arc<EmailService>>,
    Json(body): Json<PasswordResetRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Validate the token doesn't contain injection chars
    shield.validate_email_header("reset_token", &body.reset_token)?;
    shield.validate_outbound_email(
        &[&body.to],
        "Reset Your Password",
        &[("username", &body.username)],
    )?;
    let id = email
        .send_password_reset(
            &body.to,
            &Shield::escape_email_content(&body.username),
            &body.reset_token,
            body.expires_minutes,
        )
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

// ---------- Support ----------

#[derive(Deserialize)]
pub struct SupportConfirmRequest {
    pub to: String,
    pub username: String,
    pub ticket_id: String,
    pub subject: String,
    pub message: String,
}

pub async fn send_support_confirmation(
    Extension(shield): Extension<Arc<Shield>>,
    Extension(email): Extension<Arc<EmailService>>,
    Json(body): Json<SupportConfirmRequest>,
) -> AppResult<Json<serde_json::Value>> {
    shield.validate_email_header("ticket_id", &body.ticket_id)?;
    shield.validate_outbound_email(
        &[&body.to],
        &body.subject,
        &[
            ("username", &body.username),
            ("message", &body.message),
        ],
    )?;
    let id = email
        .send_support_confirmation(
            &body.to,
            &Shield::escape_email_content(&body.username),
            &Shield::escape_email_content(&body.ticket_id),
            &Shield::escape_email_content(&body.subject),
            &Shield::escape_email_content(&body.message),
        )
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct SupportResponseRequest {
    pub to: String,
    pub username: String,
    pub ticket_id: String,
    pub subject: String,
    pub response_body: String,
    pub responder_name: String,
}

pub async fn send_support_response(
    Extension(shield): Extension<Arc<Shield>>,
    Extension(email): Extension<Arc<EmailService>>,
    Json(body): Json<SupportResponseRequest>,
) -> AppResult<Json<serde_json::Value>> {
    shield.validate_email_header("ticket_id", &body.ticket_id)?;
    shield.validate_outbound_email(
        &[&body.to],
        &body.subject,
        &[
            ("username", &body.username),
            ("response_body", &body.response_body),
            ("responder_name", &body.responder_name),
        ],
    )?;
    let id = email
        .send_support_response(
            &body.to,
            &Shield::escape_email_content(&body.username),
            &Shield::escape_email_content(&body.ticket_id),
            &Shield::escape_email_content(&body.subject),
            &Shield::escape_email_content(&body.response_body),
            &Shield::escape_email_content(&body.responder_name),
        )
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

// ---------- Security ----------

#[derive(Deserialize)]
pub struct SecurityAlertRequest {
    pub severity: String,
    pub title: String,
    pub description: String,
    pub source_ip: Option<String>,
    pub timestamp: String,
    #[serde(default)]
    pub details: Vec<(String, String)>,
    pub action_taken: String,
}

pub async fn send_security_alert(
    Extension(shield): Extension<Arc<Shield>>,
    Extension(email): Extension<Arc<EmailService>>,
    Json(body): Json<SecurityAlertRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Validate header-level fields
    shield.validate_email_header("title", &body.title)?;
    shield.validate_email_content("description", &body.description)?;
    shield.validate_email_content("action_taken", &body.action_taken)?;

    // Validate detail values
    for (key, value) in &body.details {
        shield.validate_email_content(key, value)?;
    }

    let severity = match body.severity.to_lowercase().as_str() {
        "critical" => prometheus_email::AlertSeverity::Critical,
        "high" => prometheus_email::AlertSeverity::High,
        "medium" => prometheus_email::AlertSeverity::Medium,
        _ => prometheus_email::AlertSeverity::Low,
    };

    let alert = prometheus_email::SecurityAlert {
        severity,
        title: Shield::escape_email_content(&body.title),
        description: Shield::escape_email_content(&body.description),
        source_ip: body.source_ip.map(|ip| Shield::escape_email_content(&ip)),
        timestamp: Shield::escape_email_content(&body.timestamp),
        details: body.details.into_iter().map(|(k, v)| {
            (Shield::escape_email_content(&k), Shield::escape_email_content(&v))
        }).collect(),
        action_taken: Shield::escape_email_content(&body.action_taken),
        dashboard_url: email.config().base_url.clone(),
    };

    let id = email.send_security_alert(&alert).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct DailyReportRequest {
    pub date: String,
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub unique_ips: u64,
    pub active_bans: u32,
    #[serde(default)]
    pub threat_breakdown: Vec<ThreatEntryInput>,
    #[serde(default)]
    pub top_blocked_ips: Vec<(String, u32)>,
    #[serde(default)]
    pub active_training: u32,
    #[serde(default)]
    pub deployed_models: u32,
    pub uptime: String,
    #[serde(default = "default_true")]
    pub audit_chain_valid: bool,
    #[serde(default)]
    pub audit_chain_length: usize,
}

fn default_true() -> bool { true }

#[derive(Deserialize)]
pub struct ThreatEntryInput {
    pub category: String,
    pub count: u32,
    #[serde(default = "default_color")]
    pub color: String,
}

fn default_color() -> String { "#DC2626".into() }

pub async fn send_daily_report(
    Extension(shield): Extension<Arc<Shield>>,
    Extension(email): Extension<Arc<EmailService>>,
    Json(body): Json<DailyReportRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Validate string fields for injection
    shield.validate_email_header("date", &body.date)?;
    shield.validate_email_header("uptime", &body.uptime)?;

    for entry in &body.threat_breakdown {
        shield.validate_email_content("category", &entry.category)?;
    }
    for (ip, _) in &body.top_blocked_ips {
        shield.validate_email_header("ip", ip)?;
    }

    let report = prometheus_email::DailyReport {
        date: Shield::escape_email_content(&body.date),
        total_requests: body.total_requests,
        blocked_requests: body.blocked_requests,
        unique_ips: body.unique_ips,
        active_bans: body.active_bans,
        threat_breakdown: body
            .threat_breakdown
            .into_iter()
            .map(|t| prometheus_email::ThreatEntry {
                category: Shield::escape_email_content(&t.category),
                count: t.count,
                color: Shield::escape_email_content(&t.color),
            })
            .collect(),
        top_blocked_ips: body.top_blocked_ips.into_iter().map(|(ip, count)| {
            (Shield::escape_email_content(&ip), count)
        }).collect(),
        active_training: body.active_training,
        deployed_models: body.deployed_models,
        uptime: Shield::escape_email_content(&body.uptime),
        dashboard_url: email.config().base_url.clone(),
        audit_chain_valid: body.audit_chain_valid,
        audit_chain_length: body.audit_chain_length,
    };

    let id = email.send_daily_report(&report).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}
