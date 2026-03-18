// ============================================================================
// File: security_alert.rs
// Description: Security alert email template with severity levels and threat details
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use super::layout;

pub struct SecurityAlert {
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub source_ip: Option<String>,
    pub timestamp: String,
    pub details: Vec<(String, String)>,
    pub action_taken: String,
    pub dashboard_url: String,
}

#[derive(Debug, Clone, Copy)]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl AlertSeverity {
    pub fn label(&self) -> &str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            Self::Critical => "#DC2626",
            Self::High => "#EA580C",
            Self::Medium => "#D97706",
            Self::Low => "#2563EB",
        }
    }

    pub fn bg(&self) -> &str {
        match self {
            Self::Critical => "#FEF2F2",
            Self::High => "#FFF7ED",
            Self::Medium => "#FFFBEB",
            Self::Low => "#EFF6FF",
        }
    }
}

pub fn render(alert: &SecurityAlert) -> String {
    let severity_badge = format!(
        r#"<span style="display:inline-block;padding:4px 12px;background-color:{bg};color:{color};font-size:11px;font-weight:700;border-radius:4px;letter-spacing:0.5px;">{label}</span>"#,
        bg = alert.severity.bg(),
        color = alert.severity.color(),
        label = alert.severity.label(),
    );

    let mut detail_rows = String::new();
    if let Some(ref ip) = alert.source_ip {
        detail_rows.push_str(&layout::metric_row("Source IP", ip, "#111827"));
    }
    detail_rows.push_str(&layout::metric_row("Detected At", &alert.timestamp, "#111827"));
    for (k, v) in &alert.details {
        detail_rows.push_str(&layout::metric_row(k, v, "#111827"));
    }
    detail_rows.push_str(&layout::metric_row("Action Taken", &alert.action_taken, "#14b8a6"));

    let body = format!(
        r#"<div style="margin-bottom:16px;">{severity_badge}</div>
<h1 style="margin:0 0 8px;font-size:22px;font-weight:700;color:#111827;">{title}</h1>
<p style="margin:0 0 24px;font-size:15px;color:#374151;line-height:1.6;">{description}</p>

<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="margin:20px 0;border:1px solid #E8D4C4;border-radius:8px;overflow:hidden;">
{detail_rows}
</table>

{btn}"#,
        title = alert.title,
        description = alert.description,
        btn = layout::button("View in Dashboard", &alert.dashboard_url),
    );

    let banner = if matches!(alert.severity, AlertSeverity::Critical | AlertSeverity::High) {
        layout::alert_banner(&alert.description)
    } else {
        String::new()
    };

    layout::wrap(
        &format!("[{}] {}", alert.severity.label(), alert.title),
        &format!("Security alert: {}", alert.title),
        &format!("{}{}", banner, body),
        r#"<p style="margin:0 0 8px;font-size:12px;color:#DC2626;font-weight:600;">This is an automated security notification.</p>"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AlertSeverity::label() ─────────────────────────────

    #[test]
    fn severity_critical_label() {
        assert_eq!(AlertSeverity::Critical.label(), "CRITICAL");
    }

    #[test]
    fn severity_high_label() {
        assert_eq!(AlertSeverity::High.label(), "HIGH");
    }

    #[test]
    fn severity_medium_label() {
        assert_eq!(AlertSeverity::Medium.label(), "MEDIUM");
    }

    #[test]
    fn severity_low_label() {
        assert_eq!(AlertSeverity::Low.label(), "LOW");
    }

    // ── AlertSeverity::color() ─────────────────────────────

    #[test]
    fn severity_critical_color() {
        assert_eq!(AlertSeverity::Critical.color(), "#DC2626");
    }

    #[test]
    fn severity_high_color() {
        assert_eq!(AlertSeverity::High.color(), "#EA580C");
    }

    #[test]
    fn severity_medium_color() {
        assert_eq!(AlertSeverity::Medium.color(), "#D97706");
    }

    #[test]
    fn severity_low_color() {
        assert_eq!(AlertSeverity::Low.color(), "#2563EB");
    }

    // ── AlertSeverity::bg() ────────────────────────────────

    #[test]
    fn severity_critical_bg() {
        assert_eq!(AlertSeverity::Critical.bg(), "#FEF2F2");
    }

    #[test]
    fn severity_high_bg() {
        assert_eq!(AlertSeverity::High.bg(), "#FFF7ED");
    }

    #[test]
    fn severity_medium_bg() {
        assert_eq!(AlertSeverity::Medium.bg(), "#FFFBEB");
    }

    #[test]
    fn severity_low_bg() {
        assert_eq!(AlertSeverity::Low.bg(), "#EFF6FF");
    }

    // ── render() ───────────────────────────────────────────

    fn sample_alert(severity: AlertSeverity) -> SecurityAlert {
        SecurityAlert {
            severity,
            title: "SQL Injection Attempt".to_string(),
            description: "Detected SQL injection in query parameter".to_string(),
            source_ip: Some("10.0.0.1".to_string()),
            timestamp: "2026-03-07 12:00:00 UTC".to_string(),
            details: vec![
                ("Endpoint".to_string(), "/api/v1/data".to_string()),
                ("Method".to_string(), "POST".to_string()),
            ],
            action_taken: "Request blocked".to_string(),
            dashboard_url: "https://prometheus.example.com/dashboard".to_string(),
        }
    }

    #[test]
    fn render_contains_title() {
        let alert = sample_alert(AlertSeverity::High);
        let html = render(&alert);
        assert!(html.contains("SQL Injection Attempt"));
    }

    #[test]
    fn render_contains_description() {
        let alert = sample_alert(AlertSeverity::Medium);
        let html = render(&alert);
        assert!(html.contains("Detected SQL injection in query parameter"));
    }

    #[test]
    fn render_contains_source_ip() {
        let alert = sample_alert(AlertSeverity::High);
        let html = render(&alert);
        assert!(html.contains("10.0.0.1"));
    }

    #[test]
    fn render_contains_timestamp() {
        let alert = sample_alert(AlertSeverity::Low);
        let html = render(&alert);
        assert!(html.contains("2026-03-07 12:00:00 UTC"));
    }

    #[test]
    fn render_contains_detail_entries() {
        let alert = sample_alert(AlertSeverity::Medium);
        let html = render(&alert);
        assert!(html.contains("Endpoint"));
        assert!(html.contains("/api/v1/data"));
        assert!(html.contains("Method"));
        assert!(html.contains("POST"));
    }

    #[test]
    fn render_contains_action_taken() {
        let alert = sample_alert(AlertSeverity::High);
        let html = render(&alert);
        assert!(html.contains("Request blocked"));
    }

    #[test]
    fn render_contains_dashboard_link() {
        let alert = sample_alert(AlertSeverity::High);
        let html = render(&alert);
        assert!(html.contains("https://prometheus.example.com/dashboard"));
    }

    #[test]
    fn render_critical_has_alert_banner() {
        let alert = sample_alert(AlertSeverity::Critical);
        let html = render(&alert);
        assert!(html.contains("Security Alert"));
    }

    #[test]
    fn render_high_has_alert_banner() {
        let alert = sample_alert(AlertSeverity::High);
        let html = render(&alert);
        assert!(html.contains("Security Alert"));
    }

    #[test]
    fn render_low_no_alert_banner() {
        let alert = sample_alert(AlertSeverity::Low);
        let html = render(&alert);
        // Low severity should not have the alert banner with "Security Alert" as strong text
        // But it does contain "Security Alert" in the footer. Check for the specific banner div.
        assert!(!html.contains("border-left:4px solid #EF4444"));
    }

    #[test]
    fn render_without_source_ip() {
        let mut alert = sample_alert(AlertSeverity::Medium);
        alert.source_ip = None;
        let html = render(&alert);
        // Should still render without error
        assert!(html.contains("SQL Injection Attempt"));
        assert!(!html.contains("Source IP"));
    }

    #[test]
    fn render_severity_badge_displayed() {
        let alert = sample_alert(AlertSeverity::Critical);
        let html = render(&alert);
        assert!(html.contains("CRITICAL"));
        assert!(html.contains(AlertSeverity::Critical.color()));
    }
}
