// ============================================================================
// File: client.rs
// Description: Resend API HTTP client for sending transactional emails
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use crate::error::EmailError;
use serde_json::json;

pub struct ResendClient {
    api_key: String,
    from: String,
    reply_to: Option<String>,
    http: reqwest::Client,
}

impl ResendClient {
    pub fn new(api_key: &str, from: &str, reply_to: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            from: from.to_string(),
            reply_to: reply_to.map(|s| s.to_string()),
            http: reqwest::Client::new(),
        }
    }

    /// Send an email via the Resend API. Returns the Resend message ID on success.
    pub async fn send(
        &self,
        to: &[&str],
        subject: &str,
        html: &str,
    ) -> Result<String, EmailError> {
        self.send_with_options(to, subject, html, None, None).await
    }

    /// Send with optional CC and custom reply-to.
    pub async fn send_with_options(
        &self,
        to: &[&str],
        subject: &str,
        html: &str,
        cc: Option<&[&str]>,
        reply_to_override: Option<&str>,
    ) -> Result<String, EmailError> {
        let mut body = json!({
            "from": self.from,
            "to": to,
            "subject": subject,
            "html": html,
        });

        let reply = reply_to_override
            .map(|s| s.to_string())
            .or_else(|| self.reply_to.clone());

        if let Some(reply_to) = reply {
            body["reply_to"] = json!(reply_to);
        }
        if let Some(cc_addrs) = cc {
            if !cc_addrs.is_empty() {
                body["cc"] = json!(cc_addrs);
            }
        }

        let resp = self
            .http
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            let id = result["id"].as_str().unwrap_or("unknown").to_string();
            tracing::info!(message_id = %id, to = ?to, subject, "Email sent via Resend");
            Ok(id)
        } else {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            tracing::error!(%status, body = %err_body, "Resend API error");
            Err(EmailError::ResendApi(format!("{}: {}", status, err_body)))
        }
    }
}
