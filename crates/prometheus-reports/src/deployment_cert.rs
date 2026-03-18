// ============================================================================
// File: deployment_cert.rs
// Description: Deployment certificate PDF attesting model deployment to target device
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Deployment certificate PDF generation.
//!
//! Produces a professional, single-page certificate that attests to the
//! deployment of a trained model onto a target device.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{file_to_data_uri, render_html_to_pdf, Branding, ReportConfig, ReportError};

// ── Data ────────────────────────────────────────────────────────────────────

/// All data required to generate a deployment certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentCertData {
    /// Human-readable model name.
    pub model_name: String,
    /// SHA-256 (or similar) hash of the model artefact.
    pub model_hash: String,
    /// Friendly name of the deployment target / device.
    pub target_name: String,
    /// IP address or hostname of the target.
    pub target_ip: String,
    /// Target architecture (e.g. `aarch64-linux-gnu`, `x86_64`, `cortex-m4`).
    pub target_arch: String,
    /// Username or service account that initiated the deployment.
    pub deployed_by: String,
    /// Timestamp of the deployment.
    pub deployed_at: DateTime<Utc>,
    /// Size of the deployed binary in bytes.
    pub binary_size_bytes: u64,
    /// Quantization type applied (e.g. `INT8`, `FP16`, `None`).
    pub quantization_type: String,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Generate a deployment certificate PDF and return the path on disk.
pub fn generate_deployment_cert(
    data: &DeploymentCertData,
    config: &ReportConfig,
) -> Result<String, ReportError> {
    let html = build_html(data, &config.branding)?;
    let cert_id = Uuid::new_v4();
    let filename = format!(
        "deployment-cert-{}-{}.pdf",
        slug(&data.model_name),
        cert_id
    );
    let output_path = format!("{}/{}", config.output_dir, filename);
    render_html_to_pdf(&html, &output_path, config.chrome_path.as_deref())
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn slug(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ── HTML Template ───────────────────────────────────────────────────────────

fn build_html(data: &DeploymentCertData, branding: &Branding) -> Result<String, ReportError> {
    let primary = &branding.primary_color;
    let accent = &branding.accent_color;
    let company = &branding.company_name;

    let logo_html = if let Some(ref path) = branding.logo_path {
        match file_to_data_uri(path) {
            Ok(uri) => format!(r#"<img src="{}" alt="Logo" class="logo"/>"#, uri),
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    let deployed_at_str = data.deployed_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let binary_size_str = format_bytes(data.binary_size_bytes);
    let cert_id = Uuid::new_v4();

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"/>
<title>Deployment Certificate — {model_name}</title>
<style>
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap');

  *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}

  html, body {{
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    font-size: 14px;
    line-height: 1.6;
    color: #3c3632;
    background: #FFFDF7;
  }}

  .page {{
    max-width: 780px;
    margin: 0 auto;
    padding: 56px 64px;
  }}

  /* ── Decorative border ─────────────────────────── */
  .cert-border {{
    border: 3px solid {accent};
    border-radius: 16px;
    padding: 48px 52px;
    position: relative;
    background: #FFFDF7;
  }}
  .cert-border::before {{
    content: '';
    position: absolute;
    inset: 6px;
    border: 1px solid #E8D4C4;
    border-radius: 12px;
    pointer-events: none;
  }}

  /* ── Header ────────────────────────────────────── */
  .cert-header {{
    text-align: center;
    margin-bottom: 36px;
  }}
  .logo {{
    height: 52px;
    margin-bottom: 12px;
  }}
  .cert-company {{
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 2.5px;
    color: {accent};
    margin-bottom: 6px;
  }}
  .cert-main-title {{
    font-size: 30px;
    font-weight: 800;
    color: {primary};
    letter-spacing: -0.5px;
  }}
  .cert-divider {{
    width: 80px;
    height: 3px;
    background: {primary};
    margin: 14px auto 0;
    border-radius: 2px;
  }}

  /* ── Model hash callout ────────────────────────── */
  .hash-section {{
    text-align: center;
    margin: 32px 0;
  }}
  .hash-label {{
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: #8a7f78;
    margin-bottom: 6px;
  }}
  .hash-value {{
    display: inline-block;
    background: #f5f0eb;
    border: 1.5px solid #E8D4C4;
    border-radius: 8px;
    padding: 10px 24px;
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 15px;
    font-weight: 600;
    color: {primary};
    letter-spacing: 0.5px;
    word-break: break-all;
  }}

  /* ── Details table ─────────────────────────────── */
  .details-section {{
    margin: 32px 0;
  }}
  .details-title {{
    font-size: 14px;
    font-weight: 700;
    color: {primary};
    border-left: 4px solid {accent};
    padding-left: 12px;
    margin-bottom: 14px;
  }}
  .details-table {{
    width: 100%;
    border-collapse: collapse;
  }}
  .details-table tr:nth-child(even) {{
    background: #fdf9f4;
  }}
  .details-table td {{
    padding: 10px 16px;
    border-bottom: 1px solid #f0e6dc;
    vertical-align: top;
  }}
  .details-table .dt-key {{
    width: 200px;
    font-weight: 600;
    color: #5c5550;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }}
  .details-table .dt-val {{
    font-size: 14px;
    color: #3c3632;
  }}
  .details-table .dt-val.mono {{
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
    color: {primary};
  }}

  /* ── Quantization badge ────────────────────────── */
  .quant-badge {{
    display: inline-block;
    background: {primary};
    color: #FFF;
    font-size: 12px;
    font-weight: 700;
    padding: 3px 12px;
    border-radius: 12px;
    letter-spacing: 0.4px;
  }}

  /* ── Attestation ───────────────────────────────── */
  .attestation {{
    text-align: center;
    margin: 36px 0 24px;
    padding: 20px;
    background: #f5f0eb;
    border-radius: 10px;
    border: 1px solid #E8D4C4;
  }}
  .attestation p {{
    font-size: 13px;
    color: #5c5550;
    max-width: 500px;
    margin: 0 auto;
    line-height: 1.7;
  }}
  .attestation strong {{
    color: #3c3632;
  }}

  /* ── Signature line ────────────────────────────── */
  .signature-row {{
    display: flex;
    justify-content: space-between;
    margin-top: 40px;
    padding-top: 4px;
  }}
  .sig-block {{
    text-align: center;
    width: 220px;
  }}
  .sig-line {{
    border-top: 1.5px solid #C4A484;
    margin-bottom: 6px;
  }}
  .sig-label {{
    font-size: 11px;
    color: #8a7f78;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }}

  /* ── Footer ────────────────────────────────────── */
  .cert-footer {{
    text-align: center;
    margin-top: 28px;
    font-size: 11px;
    color: #8a7f78;
  }}
  .cert-id {{
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 10px;
    color: #b0a69e;
    margin-top: 4px;
  }}
</style>
</head>
<body>
<div class="page">
<div class="cert-border">

  <!-- Header -->
  <div class="cert-header">
    {logo_html}
    <div class="cert-company">{company}</div>
    <div class="cert-main-title">Deployment Certificate</div>
    <div class="cert-divider"></div>
  </div>

  <!-- Model Hash -->
  <div class="hash-section">
    <div class="hash-label">Model Artefact Hash</div>
    <div class="hash-value">{model_hash}</div>
  </div>

  <!-- Deployment Details -->
  <div class="details-section">
    <div class="details-title">Deployment Details</div>
    <table class="details-table">
      <tr>
        <td class="dt-key">Model Name</td>
        <td class="dt-val">{model_name}</td>
      </tr>
      <tr>
        <td class="dt-key">Target Device</td>
        <td class="dt-val">{target_name}</td>
      </tr>
      <tr>
        <td class="dt-key">Target IP / Host</td>
        <td class="dt-val mono">{target_ip}</td>
      </tr>
      <tr>
        <td class="dt-key">Target Architecture</td>
        <td class="dt-val mono">{target_arch}</td>
      </tr>
      <tr>
        <td class="dt-key">Binary Size</td>
        <td class="dt-val">{binary_size}</td>
      </tr>
      <tr>
        <td class="dt-key">Quantization</td>
        <td class="dt-val"><span class="quant-badge">{quantization}</span></td>
      </tr>
      <tr>
        <td class="dt-key">Deployed By</td>
        <td class="dt-val">{deployed_by}</td>
      </tr>
      <tr>
        <td class="dt-key">Deployed At</td>
        <td class="dt-val">{deployed_at}</td>
      </tr>
    </table>
  </div>

  <!-- Attestation -->
  <div class="attestation">
    <p>
      This document certifies that the model <strong>{model_name}</strong>
      (hash <strong>{hash_short}&hellip;</strong>) has been successfully deployed
      to <strong>{target_name}</strong> (<strong>{target_arch}</strong>) by
      <strong>{deployed_by}</strong> on <strong>{deployed_at}</strong>.
      The artefact integrity has been verified against the recorded hash.
    </p>
  </div>

  <!-- Signature lines -->
  <div class="signature-row">
    <div class="sig-block">
      <div class="sig-line"></div>
      <div class="sig-label">Deploying Engineer</div>
    </div>
    <div class="sig-block">
      <div class="sig-line"></div>
      <div class="sig-label">Platform Verification</div>
    </div>
  </div>

  <!-- Footer -->
  <div class="cert-footer">
    <div>&copy; {company} &mdash; Prometheus Platform &middot; Confidential</div>
    <div class="cert-id">Certificate ID: {cert_id}</div>
  </div>

</div><!-- .cert-border -->
</div><!-- .page -->
</body>
</html>"##,
        model_name = html_escape(&data.model_name),
        model_hash = html_escape(&data.model_hash),
        hash_short = if data.model_hash.len() > 16 {
            &data.model_hash[..16]
        } else {
            &data.model_hash
        },
        target_name = html_escape(&data.target_name),
        target_ip = html_escape(&data.target_ip),
        target_arch = html_escape(&data.target_arch),
        deployed_by = html_escape(&data.deployed_by),
        deployed_at = deployed_at_str,
        binary_size = binary_size_str,
        quantization = html_escape(&data.quantization_type),
        cert_id = cert_id,
    );

    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_cert_data() -> DeploymentCertData {
        DeploymentCertData {
            model_name: "AnomalyDetector-v2".to_string(),
            model_hash: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6".to_string(),
            target_name: "EdgeNode-07".to_string(),
            target_ip: "192.168.1.42".to_string(),
            target_arch: "aarch64-linux-gnu".to_string(),
            deployed_by: "alice".to_string(),
            deployed_at: Utc::now(),
            binary_size_bytes: 15_728_640, // ~15 MB
            quantization_type: "INT8".to_string(),
        }
    }

    // ── slug tests ──────────────────────────────────────

    #[test]
    fn slug_basic() {
        assert_eq!(slug("My Model"), "my-model");
    }

    #[test]
    fn slug_special_chars() {
        assert_eq!(slug("v2.1-beta"), "v2-1-beta");
    }

    // ── format_bytes tests ──────────────────────────────

    #[test]
    fn format_bytes_bytes() {
        assert_eq!(format_bytes(500), "500 B");
    }

    #[test]
    fn format_bytes_kilobytes() {
        assert_eq!(format_bytes(2048), "2.0 KB");
    }

    #[test]
    fn format_bytes_megabytes() {
        // 5 * 1024 * 1024 = 5242880
        assert_eq!(format_bytes(5_242_880), "5.00 MB");
    }

    #[test]
    fn format_bytes_gigabytes() {
        // 2 * 1024^3 = 2147483648
        assert_eq!(format_bytes(2_147_483_648), "2.00 GB");
    }

    #[test]
    fn format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    // ── html_escape tests ───────────────────────────────

    #[test]
    fn html_escape_all_entities() {
        assert_eq!(
            html_escape(r#"<a href="x">&'y"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;y"
        );
    }

    // ── build_html tests ────────────────────────────────

    #[test]
    fn html_contains_model_hash() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6"));
    }

    #[test]
    fn html_contains_model_hash_short() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        // hash_short is first 16 chars
        assert!(html.contains("a1b2c3d4e5f6a7b8"));
    }

    #[test]
    fn html_contains_target_name() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("EdgeNode-07"));
    }

    #[test]
    fn html_contains_target_ip() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("192.168.1.42"));
    }

    #[test]
    fn html_contains_target_arch() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("aarch64-linux-gnu"));
    }

    #[test]
    fn html_contains_deployed_by() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("alice"));
    }

    #[test]
    fn html_contains_deployment_date() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        // deployed_at is formatted as "%Y-%m-%d %H:%M:%S UTC"
        assert!(html.contains("UTC"));
    }

    #[test]
    fn html_contains_binary_size() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("15.00 MB"));
    }

    #[test]
    fn html_contains_quantization_type() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("INT8"));
    }

    #[test]
    fn html_contains_company_name() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("AutomataNexus"));
    }

    #[test]
    fn html_contains_deployment_certificate_title() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("Deployment Certificate"));
    }

    #[test]
    fn html_contains_model_name() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("AnomalyDetector-v2"));
    }

    #[test]
    fn html_escapes_special_chars_in_model_name() {
        let mut data = sample_cert_data();
        data.model_name = "model<evil>&test".to_string();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("model&lt;evil&gt;&amp;test"));
        assert!(!html.contains("model<evil>"));
    }

    #[test]
    fn html_short_hash_when_less_than_16() {
        let mut data = sample_cert_data();
        data.model_hash = "abc123".to_string();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        // Short hash should be used in full (less than 16 chars)
        assert!(html.contains("abc123"));
    }

    #[test]
    fn html_uses_custom_branding_colors() {
        let data = sample_cert_data();
        let branding = Branding {
            logo_path: None,
            company_name: "TestCorp".to_string(),
            primary_color: "#ff0000".to_string(),
            accent_color: "#00ff00".to_string(),
        };
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("#ff0000"));
        assert!(html.contains("#00ff00"));
        assert!(html.contains("TestCorp"));
    }

    // ── Additional format_bytes tests ──────────────────────

    #[test]
    fn format_bytes_boundary_kb() {
        // Exactly 1 KB
        assert_eq!(format_bytes(1024), "1.0 KB");
    }

    #[test]
    fn format_bytes_boundary_mb() {
        // Exactly 1 MB
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    }

    #[test]
    fn format_bytes_boundary_gb() {
        // Exactly 1 GB
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn format_bytes_large_value() {
        // 10 GB
        assert_eq!(format_bytes(10 * 1024 * 1024 * 1024), "10.00 GB");
    }

    #[test]
    fn format_bytes_one_byte() {
        assert_eq!(format_bytes(1), "1 B");
    }

    // ── Additional slug tests ──────────────────────────────

    #[test]
    fn slug_empty_string() {
        assert_eq!(slug(""), "");
    }

    #[test]
    fn slug_all_special() {
        assert_eq!(slug("!!!"), "---");
    }

    // ── html_escape edge cases ─────────────────────────────

    #[test]
    fn html_escape_empty_string() {
        assert_eq!(html_escape(""), "");
    }

    #[test]
    fn html_escape_no_entities() {
        assert_eq!(html_escape("hello world 123"), "hello world 123");
    }

    // ── Additional build_html tests ────────────────────────

    #[test]
    fn html_is_well_formed() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("</body>"));
    }

    #[test]
    fn html_contains_certificate_id() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("Certificate ID:"));
    }

    #[test]
    fn html_contains_attestation_section() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("certifies that the model"));
        assert!(html.contains("has been successfully deployed"));
    }

    #[test]
    fn html_contains_signature_lines() {
        let data = sample_cert_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("Deploying Engineer"));
        assert!(html.contains("Platform Verification"));
    }
}
