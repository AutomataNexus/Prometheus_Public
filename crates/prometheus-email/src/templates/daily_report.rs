// ============================================================================
// File: daily_report.rs
// Description: Daily security and operations report email template for Shield metrics
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use super::layout;

pub struct DailyReport {
    /// Report date (e.g., "March 5, 2026").
    pub date: String,
    /// Total requests processed by Shield.
    pub total_requests: u64,
    /// Requests blocked by Shield.
    pub blocked_requests: u64,
    /// Unique IPs that sent requests.
    pub unique_ips: u64,
    /// Currently banned IPs.
    pub active_bans: u32,
    /// Breakdown of threat types detected.
    pub threat_breakdown: Vec<ThreatEntry>,
    /// Top offending IPs (IP, block count).
    pub top_blocked_ips: Vec<(String, u32)>,
    /// Active training runs.
    pub active_training: u32,
    /// Total deployed models.
    pub deployed_models: u32,
    /// System uptime string (e.g., "14d 6h 23m").
    pub uptime: String,
    /// Link to the full dashboard.
    pub dashboard_url: String,
    /// Whether the audit chain is intact.
    pub audit_chain_valid: bool,
    /// Total audit events in the chain.
    pub audit_chain_length: usize,
}

pub struct ThreatEntry {
    pub category: String,
    pub count: u32,
    pub color: String,
}

pub fn render(report: &DailyReport) -> String {
    let block_rate = if report.total_requests > 0 {
        (report.blocked_requests as f64 / report.total_requests as f64) * 100.0
    } else {
        0.0
    };

    let chain_status = if report.audit_chain_valid {
        r#"<span style="color:#16A34A;">&#10003; Intact</span>"#
    } else {
        r#"<span style="color:#DC2626;">&#10007; Broken — investigate immediately</span>"#
    };

    // Overview metrics
    let overview = format!(
        r#"<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="margin:16px 0;">
<tr>
  <td width="50%" style="padding:12px;background-color:#FFFDF7;border:1px solid #E8D4C4;border-radius:8px 0 0 8px;">
    <p style="margin:0;font-size:11px;color:#6b7280;text-transform:uppercase;letter-spacing:0.5px;">Requests</p>
    <p style="margin:4px 0 0;font-size:24px;font-weight:700;color:#111827;">{total}</p>
  </td>
  <td width="50%" style="padding:12px;background-color:#FFFDF7;border:1px solid #E8D4C4;border-left:0;border-radius:0 8px 8px 0;">
    <p style="margin:0;font-size:11px;color:#6b7280;text-transform:uppercase;letter-spacing:0.5px;">Blocked</p>
    <p style="margin:4px 0 0;font-size:24px;font-weight:700;color:#DC2626;">{blocked} <span style="font-size:14px;color:#6b7280;">({block_rate:.1}%)</span></p>
  </td>
</tr>
</table>
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="margin:0 0 16px;">
<tr>
  <td width="33%" style="padding:12px;background-color:#FFFDF7;border:1px solid #E8D4C4;border-radius:8px 0 0 8px;">
    <p style="margin:0;font-size:11px;color:#6b7280;text-transform:uppercase;">Unique IPs</p>
    <p style="margin:4px 0 0;font-size:18px;font-weight:700;color:#111827;">{unique_ips}</p>
  </td>
  <td width="33%" style="padding:12px;background-color:#FFFDF7;border:1px solid #E8D4C4;border-left:0;">
    <p style="margin:0;font-size:11px;color:#6b7280;text-transform:uppercase;">Active Bans</p>
    <p style="margin:4px 0 0;font-size:18px;font-weight:700;color:#EA580C;">{bans}</p>
  </td>
  <td width="34%" style="padding:12px;background-color:#FFFDF7;border:1px solid #E8D4C4;border-left:0;border-radius:0 8px 8px 0;">
    <p style="margin:0;font-size:11px;color:#6b7280;text-transform:uppercase;">Uptime</p>
    <p style="margin:4px 0 0;font-size:18px;font-weight:700;color:#14b8a6;">{uptime}</p>
  </td>
</tr>
</table>"#,
        total = report.total_requests,
        blocked = report.blocked_requests,
        unique_ips = report.unique_ips,
        bans = report.active_bans,
        uptime = report.uptime,
    );

    // Threat breakdown
    let mut threat_rows = String::new();
    for entry in &report.threat_breakdown {
        threat_rows.push_str(&layout::metric_row(&entry.category, &entry.count.to_string(), &entry.color));
    }
    let threats_section = if report.threat_breakdown.is_empty() {
        r#"<p style="margin:8px 0;font-size:14px;color:#6b7280;">No threats detected.</p>"#.to_string()
    } else {
        format!(
            r#"<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="margin:8px 0;border:1px solid #E8D4C4;border-radius:8px;overflow:hidden;">
{threat_rows}
</table>"#
        )
    };

    // Top blocked IPs
    let mut ip_rows = String::new();
    for (ip, count) in &report.top_blocked_ips {
        ip_rows.push_str(&format!(
            r#"<tr>
  <td style="padding:8px 12px;font-size:13px;color:#111827;font-family:monospace;border-bottom:1px solid #E8D4C4;">{ip}</td>
  <td align="right" style="padding:8px 12px;font-size:13px;font-weight:600;color:#DC2626;border-bottom:1px solid #E8D4C4;">{count} blocks</td>
</tr>"#
        ));
    }
    let ips_section = if report.top_blocked_ips.is_empty() {
        r#"<p style="margin:8px 0;font-size:14px;color:#6b7280;">No IPs blocked today.</p>"#.to_string()
    } else {
        format!(
            r#"<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="margin:8px 0;border:1px solid #E8D4C4;border-radius:8px;overflow:hidden;">
{ip_rows}
</table>"#
        )
    };

    // Platform status
    let platform = format!(
        r#"<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="margin:8px 0;border:1px solid #E8D4C4;border-radius:8px;overflow:hidden;">
{r1}
{r2}
{r3}
</table>"#,
        r1 = layout::metric_row("Active Training Runs", &report.active_training.to_string(), "#14b8a6"),
        r2 = layout::metric_row("Deployed Models", &report.deployed_models.to_string(), "#111827"),
        r3 = layout::metric_row("Audit Chain", &format!("{} ({} events)", chain_status, report.audit_chain_length), "#111827"),
    );

    let body = format!(
        r#"<h1 style="margin:0 0 4px;font-size:22px;font-weight:700;color:#111827;">Daily Security Report</h1>
<p style="margin:0 0 24px;font-size:15px;color:#6b7280;">{date}</p>

{heading_overview}
{overview}

{heading_threats}
{threats_section}

{heading_ips}
{ips_section}

{heading_platform}
{platform}

{btn}"#,
        date = report.date,
        heading_overview = layout::heading("Overview"),
        heading_threats = layout::heading("Threat Breakdown"),
        heading_ips = layout::heading("Top Blocked IPs"),
        heading_platform = layout::heading("Platform Status"),
        btn = layout::button_ghost("View Full Dashboard", &report.dashboard_url),
    );

    layout::wrap(
        &format!("Daily Security Report — {}", report.date),
        &format!("{} requests processed, {} blocked", report.total_requests, report.blocked_requests),
        &body,
        &format!(
            r#"<p style="margin:0 0 8px;font-size:12px;color:#6b7280;">Report generated at {} UTC</p>"#,
            chrono::Utc::now().format("%H:%M")
        ),
    )
}
