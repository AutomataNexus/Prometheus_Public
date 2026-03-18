// ============================================================================
// File: lib.rs
// Description: Email service library with typed methods for Prometheus transactional emails
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
pub mod client;
pub mod config;
pub mod error;
pub mod templates;

pub use config::EmailConfig;
pub use error::EmailError;
pub use templates::{AlertSeverity, DailyReport, SecurityAlert, ThreatEntry};

use client::ResendClient;

/// High-level email service for Prometheus transactional emails.
/// Wraps the Resend API client with typed methods for each email type.
pub struct EmailService {
    client: ResendClient,
    config: EmailConfig,
}

impl EmailService {
    pub fn new(config: EmailConfig) -> Self {
        let client = ResendClient::new(
            &config.resend_api_key,
            &config.from,
            Some(&config.reply_to),
        );
        Self { client, config }
    }

    /// Try to create from environment variables.
    pub fn from_env() -> Result<Self, EmailError> {
        let config = EmailConfig::from_env()?;
        Ok(Self::new(config))
    }

    // ----- Account lifecycle -----

    /// Send welcome email after account creation.
    pub async fn send_welcome(&self, to: &str, username: &str) -> Result<String, EmailError> {
        let login_url = format!("{}/login", self.config.base_url);
        let html = templates::welcome::render(username, &login_url);
        self.client.send(&[to], "Welcome to Prometheus", &html).await
    }

    /// Send email verification code.
    pub async fn send_verification(
        &self,
        to: &str,
        username: &str,
        code: &str,
        expires_minutes: u32,
    ) -> Result<String, EmailError> {
        let verify_url = format!("{}/verify?code={}&email={}", self.config.base_url, code, to);
        let html = templates::verification::render(username, code, &verify_url, expires_minutes);
        self.client.send(&[to], "Verify Your Email — Prometheus", &html).await
    }

    /// Send password reset link.
    pub async fn send_password_reset(
        &self,
        to: &str,
        username: &str,
        reset_token: &str,
        expires_minutes: u32,
    ) -> Result<String, EmailError> {
        let reset_url = format!("{}/reset-password?token={}", self.config.base_url, reset_token);
        let html = templates::password_reset::render(username, &reset_url, expires_minutes);
        self.client.send(&[to], "Reset Your Password — Prometheus", &html).await
    }

    // ----- Support -----

    /// Send support inquiry confirmation to the user.
    pub async fn send_support_confirmation(
        &self,
        to: &str,
        username: &str,
        ticket_id: &str,
        subject: &str,
        message: &str,
    ) -> Result<String, EmailError> {
        let html = templates::support::render_confirmation(username, ticket_id, subject, message);
        self.client
            .send(&[to], &format!("Support Ticket {} — Prometheus", ticket_id), &html)
            .await
    }

    /// Send support response to the user.
    pub async fn send_support_response(
        &self,
        to: &str,
        username: &str,
        ticket_id: &str,
        subject: &str,
        response_body: &str,
        responder_name: &str,
    ) -> Result<String, EmailError> {
        let html = templates::support::render_response(
            username, ticket_id, subject, response_body, responder_name,
        );
        self.client
            .send(&[to], &format!("Re: {} [{}]", subject, ticket_id), &html)
            .await
    }

    // ----- Security -----

    /// Send a security breach/alert notification.
    pub async fn send_security_alert(
        &self,
        alert: &SecurityAlert,
    ) -> Result<String, EmailError> {
        let recipients: Vec<&str> = self.config.security_recipients.iter().map(|s| s.as_str()).collect();
        if recipients.is_empty() {
            tracing::warn!("No security email recipients configured, skipping alert");
            return Ok("skipped".into());
        }
        let html = templates::security_alert::render(alert);
        let subject = format!("[{}] {}", alert.severity.label(), alert.title);
        self.client.send(&recipients, &subject, &html).await
    }

    /// Send the daily security report.
    pub async fn send_daily_report(
        &self,
        report: &DailyReport,
    ) -> Result<String, EmailError> {
        let recipients: Vec<&str> = self.config.security_recipients.iter().map(|s| s.as_str()).collect();
        if recipients.is_empty() {
            tracing::warn!("No security email recipients configured, skipping daily report");
            return Ok("skipped".into());
        }
        let html = templates::daily_report::render(report);
        let subject = format!("Daily Security Report — {}", report.date);
        self.client.send(&recipients, &subject, &html).await
    }

    /// Send a generic notification email with custom subject and HTML body.
    pub async fn send_notification(
        &self,
        to: &str,
        subject: &str,
        html: &str,
    ) -> Result<String, EmailError> {
        self.client.send(&[to], subject, html).await
    }

    /// Access the underlying config (for building dashboard URLs, etc.).
    pub fn config(&self) -> &EmailConfig {
        &self.config
    }
}
