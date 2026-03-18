// ============================================================================
// File: lib.rs
// Description: PDF report generation library using headless Chrome rendering
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! # prometheus-reports
//!
//! PDF report generation for the Prometheus platform via headless Chrome.
//!
//! This crate provides facilities for generating professional PDF reports
//! including training summaries and deployment certificates, rendered using
//! the `headless_chrome` crate.

pub mod deployment_cert;
pub mod training_report;

pub use deployment_cert::{generate_deployment_cert, DeploymentCertData};
pub use training_report::{generate_training_report, EpochLossPair, TrainingReportData};

use serde::{Deserialize, Serialize};

/// Top-level configuration for report generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    /// Directory where generated PDFs will be written.
    pub output_dir: String,
    /// Optional path to a Chrome/Chromium binary. When `None`, headless_chrome
    /// will attempt to locate an installed browser automatically.
    pub chrome_path: Option<String>,
    /// Visual branding applied to every report.
    pub branding: Branding,
}

/// Branding information embedded into generated reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branding {
    /// Filesystem path to a logo image (PNG or SVG). The image is base64-encoded
    /// and inlined into the HTML before rendering.
    pub logo_path: Option<String>,
    /// Company name displayed in report headers and footers.
    pub company_name: String,
    /// Primary theme color (hex, e.g. `#14b8a6`).
    pub primary_color: String,
    /// Accent / secondary theme color (hex, e.g. `#C4A484`).
    pub accent_color: String,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            output_dir: "/tmp/prometheus-reports".to_string(),
            chrome_path: None,
            branding: Branding::default(),
        }
    }
}

impl Default for Branding {
    fn default() -> Self {
        Self {
            logo_path: None,
            company_name: "AutomataNexus".to_string(),
            primary_color: "#14b8a6".to_string(),
            accent_color: "#C4A484".to_string(),
        }
    }
}

/// Errors specific to the report generation pipeline.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("failed to launch headless Chrome: {0}")]
    BrowserLaunch(String),

    #[error("page navigation failed: {0}")]
    Navigation(String),

    #[error("PDF rendering failed: {0}")]
    PdfRender(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("template rendering error: {0}")]
    Template(String),
}

/// Read a file from disk and return its contents as a base64-encoded data URI
/// suitable for embedding in an `<img>` tag.
pub(crate) fn file_to_data_uri(path: &str) -> Result<String, ReportError> {
    let bytes = std::fs::read(path)?;
    let mime = if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    };
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // ── ReportConfig defaults ───────────────────────────

    #[test]
    fn report_config_default_output_dir() {
        let cfg = ReportConfig::default();
        assert_eq!(cfg.output_dir, "/tmp/prometheus-reports");
    }

    #[test]
    fn report_config_default_chrome_path_is_none() {
        let cfg = ReportConfig::default();
        assert!(cfg.chrome_path.is_none());
    }

    #[test]
    fn report_config_default_branding_populated() {
        let cfg = ReportConfig::default();
        assert_eq!(cfg.branding.company_name, "AutomataNexus");
    }

    // ── Branding defaults ───────────────────────────────

    #[test]
    fn branding_default_company_name() {
        let b = Branding::default();
        assert_eq!(b.company_name, "AutomataNexus");
    }

    #[test]
    fn branding_default_primary_color() {
        let b = Branding::default();
        assert_eq!(b.primary_color, "#14b8a6");
    }

    #[test]
    fn branding_default_accent_color() {
        let b = Branding::default();
        assert_eq!(b.accent_color, "#C4A484");
    }

    #[test]
    fn branding_default_logo_path_is_none() {
        let b = Branding::default();
        assert!(b.logo_path.is_none());
    }

    // ── ReportError display ─────────────────────────────

    #[test]
    fn report_error_browser_launch_display() {
        let err = ReportError::BrowserLaunch("no chrome".to_string());
        assert_eq!(err.to_string(), "failed to launch headless Chrome: no chrome");
    }

    #[test]
    fn report_error_navigation_display() {
        let err = ReportError::Navigation("timeout".to_string());
        assert_eq!(err.to_string(), "page navigation failed: timeout");
    }

    #[test]
    fn report_error_pdf_render_display() {
        let err = ReportError::PdfRender("oom".to_string());
        assert_eq!(err.to_string(), "PDF rendering failed: oom");
    }

    #[test]
    fn report_error_template_display() {
        let err = ReportError::Template("missing var".to_string());
        assert_eq!(err.to_string(), "template rendering error: missing var");
    }

    // ── file_to_data_uri tests ──────────────────────────

    #[test]
    fn file_to_data_uri_png() {
        let dir = std::env::temp_dir().join("prometheus-test-data-uri");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&[0x89, 0x50, 0x4E, 0x47]).unwrap(); // PNG magic bytes
        drop(f);

        let uri = file_to_data_uri(path.to_str().unwrap()).unwrap();
        assert!(uri.starts_with("data:image/png;base64,"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn file_to_data_uri_svg() {
        let dir = std::env::temp_dir().join("prometheus-test-data-uri");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.svg");
        std::fs::write(&path, "<svg></svg>").unwrap();

        let uri = file_to_data_uri(path.to_str().unwrap()).unwrap();
        assert!(uri.starts_with("data:image/svg+xml;base64,"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn file_to_data_uri_jpeg() {
        let dir = std::env::temp_dir().join("prometheus-test-data-uri");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("photo.jpg");
        std::fs::write(&path, &[0xFF, 0xD8, 0xFF]).unwrap();

        let uri = file_to_data_uri(path.to_str().unwrap()).unwrap();
        assert!(uri.starts_with("data:image/jpeg;base64,"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn file_to_data_uri_unknown_extension() {
        let dir = std::env::temp_dir().join("prometheus-test-data-uri");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("data.bin");
        std::fs::write(&path, &[0x00, 0x01, 0x02]).unwrap();

        let uri = file_to_data_uri(path.to_str().unwrap()).unwrap();
        assert!(uri.starts_with("data:application/octet-stream;base64,"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn file_to_data_uri_base64_roundtrip() {
        use base64::Engine as _;
        let dir = std::env::temp_dir().join("prometheus-test-data-uri");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roundtrip.png");
        let content = b"hello world";
        std::fs::write(&path, content).unwrap();

        let uri = file_to_data_uri(path.to_str().unwrap()).unwrap();
        // Extract base64 portion
        let b64 = uri.strip_prefix("data:image/png;base64,").unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        assert_eq!(decoded, content);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn file_to_data_uri_missing_file() {
        let result = file_to_data_uri("/nonexistent/path/missing.png");
        assert!(result.is_err());
    }

    // ── Serialization roundtrip tests ───────────────────

    #[test]
    fn report_config_serialization_roundtrip() {
        let cfg = ReportConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: ReportConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.output_dir, cfg.output_dir);
        assert_eq!(parsed.branding.company_name, cfg.branding.company_name);
    }

    #[test]
    fn branding_serialization_roundtrip() {
        let b = Branding {
            logo_path: Some("/path/to/logo.png".to_string()),
            company_name: "TestCo".to_string(),
            primary_color: "#abc".to_string(),
            accent_color: "#def".to_string(),
        };
        let json = serde_json::to_string(&b).unwrap();
        let parsed: Branding = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.logo_path, Some("/path/to/logo.png".to_string()));
        assert_eq!(parsed.company_name, "TestCo");
    }

    // ── ReportConfig custom values ─────────────────────────

    #[test]
    fn report_config_custom_output_dir() {
        let cfg = ReportConfig {
            output_dir: "/custom/reports".to_string(),
            ..ReportConfig::default()
        };
        assert_eq!(cfg.output_dir, "/custom/reports");
    }

    #[test]
    fn report_config_custom_chrome_path() {
        let cfg = ReportConfig {
            chrome_path: Some("/usr/bin/chromium".to_string()),
            ..ReportConfig::default()
        };
        assert_eq!(cfg.chrome_path, Some("/usr/bin/chromium".to_string()));
    }

    // ── ReportError IO variant ─────────────────────────────

    #[test]
    fn report_error_io_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ReportError = io_err.into();
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn report_error_io_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no access");
        let report_err: ReportError = ReportError::from(io_err);
        assert!(report_err.to_string().contains("no access"));
    }

    // ── ReportError is Debug ───────────────────────────────

    #[test]
    fn report_error_is_debug() {
        let err = ReportError::BrowserLaunch("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("BrowserLaunch"));
    }

    // ── Branding with all fields ───────────────────────────

    #[test]
    fn branding_with_logo_path() {
        let b = Branding {
            logo_path: Some("/images/logo.svg".to_string()),
            ..Branding::default()
        };
        assert_eq!(b.logo_path, Some("/images/logo.svg".to_string()));
        // Other fields should still be default
        assert_eq!(b.company_name, "AutomataNexus");
    }

    #[test]
    fn branding_is_clone() {
        let b = Branding::default();
        let b2 = b.clone();
        assert_eq!(b2.company_name, b.company_name);
        assert_eq!(b2.primary_color, b.primary_color);
    }

    #[test]
    fn report_config_is_clone() {
        let cfg = ReportConfig::default();
        let cfg2 = cfg.clone();
        assert_eq!(cfg2.output_dir, cfg.output_dir);
    }

    // ── file_to_data_uri with .jpeg extension ──────────────

    #[test]
    fn file_to_data_uri_jpeg_extension() {
        let dir = std::env::temp_dir().join("prometheus-test-data-uri-extra");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("photo.jpeg");
        std::fs::write(&path, &[0xFF, 0xD8, 0xFF]).unwrap();

        let uri = file_to_data_uri(path.to_str().unwrap()).unwrap();
        assert!(uri.starts_with("data:image/jpeg;base64,"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn file_to_data_uri_empty_file() {
        let dir = std::env::temp_dir().join("prometheus-test-data-uri-extra");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty.png");
        std::fs::write(&path, &[]).unwrap();

        let uri = file_to_data_uri(path.to_str().unwrap()).unwrap();
        // Empty file should still produce a valid data URI
        assert!(uri.starts_with("data:image/png;base64,"));

        std::fs::remove_file(&path).ok();
    }
}

/// Render an HTML string to a PDF file using headless Chrome.
///
/// Returns the absolute path to the generated PDF.
pub(crate) fn render_html_to_pdf(
    html: &str,
    output_path: &str,
    chrome_path: Option<&str>,
) -> Result<String, ReportError> {
    use base64::Engine as _;
    use headless_chrome::{Browser, LaunchOptions};

    let launch_options = LaunchOptions {
        headless: true,
        sandbox: false,
        path: chrome_path.map(std::path::PathBuf::from),
        ..LaunchOptions::default()
    };

    let browser =
        Browser::new(launch_options).map_err(|e| ReportError::BrowserLaunch(e.to_string()))?;

    let tab = browser
        .new_tab()
        .map_err(|e| ReportError::BrowserLaunch(e.to_string()))?;

    // Encode the HTML as a data URI so we don't need a temp file server.
    let encoded_html =
        base64::engine::general_purpose::STANDARD.encode(html.as_bytes());
    let data_url = format!("data:text/html;base64,{}", encoded_html);

    tab.navigate_to(&data_url)
        .map_err(|e| ReportError::Navigation(e.to_string()))?;

    tab.wait_until_navigated()
        .map_err(|e| ReportError::Navigation(e.to_string()))?;

    let pdf_bytes = tab
        .print_to_pdf(None)
        .map_err(|e| ReportError::PdfRender(e.to_string()))?;

    // Ensure the parent directory exists.
    if let Some(parent) = std::path::Path::new(output_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(output_path, &pdf_bytes)?;

    // Return the canonical path.
    let canonical = std::fs::canonicalize(output_path)?;
    Ok(canonical.to_string_lossy().into_owned())
}
