// ============================================================================
// File: training_report.rs
// Description: Training summary PDF with hyperparameters, metrics, and loss curve
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Training summary PDF report generation.
//!
//! Produces a professional, multi-section PDF that summarises a model training
//! run, including hyperparameters, final metrics, an inline SVG loss curve, and
//! actionable recommendations.

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{file_to_data_uri, render_html_to_pdf, Branding, ReportConfig, ReportError};

// ── Data ────────────────────────────────────────────────────────────────────

/// A single point on a training curve (epoch index + loss value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochLossPair {
    pub epoch: u32,
    pub loss: f64,
}

/// All data required to generate a training summary report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReportData {
    pub model_name: String,
    pub architecture: String,
    pub hyperparameters: HashMap<String, String>,
    pub epochs: u32,
    pub training_time_secs: f64,
    pub loss: f64,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub training_curves: Vec<EpochLossPair>,
    pub recommendations: Vec<String>,
    pub dataset_name: String,
    pub equipment_type: String,
    pub created_at: DateTime<Utc>,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Generate a training summary PDF and return the path to the file on disk.
pub fn generate_training_report(
    data: &TrainingReportData,
    config: &ReportConfig,
) -> Result<String, ReportError> {
    let html = build_html(data, &config.branding)?;
    let report_id = Uuid::new_v4();
    let filename = format!(
        "training-report-{}-{}.pdf",
        slug(&data.model_name),
        report_id
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

fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}h {}m {}s", h, m, s)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{:.1}s", secs)
    }
}

fn metric_bar(label: &str, value: f64, color: &str) -> String {
    let pct = (value * 100.0).min(100.0).max(0.0);
    format!(
        r#"<div class="metric-row">
            <div class="metric-label">{label}</div>
            <div class="metric-bar-track">
                <div class="metric-bar-fill" style="width:{pct:.1}%;background:{color};"></div>
            </div>
            <div class="metric-value">{pct:.2}%</div>
        </div>"#,
    )
}

/// Build an inline SVG polyline chart for the training loss curve.
fn build_loss_curve_svg(curves: &[EpochLossPair], primary: &str, _accent: &str) -> String {
    if curves.is_empty() {
        return String::from(
            r#"<div class="chart-placeholder">No training curve data available.</div>"#,
        );
    }

    let svg_w: f64 = 700.0;
    let svg_h: f64 = 260.0;
    let pad_l: f64 = 60.0;
    let pad_r: f64 = 20.0;
    let pad_t: f64 = 20.0;
    let pad_b: f64 = 40.0;
    let plot_w = svg_w - pad_l - pad_r;
    let plot_h = svg_h - pad_t - pad_b;

    let max_epoch = curves.iter().map(|p| p.epoch).max().unwrap_or(1) as f64;
    let max_loss = curves
        .iter()
        .map(|p| p.loss)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_loss = curves
        .iter()
        .map(|p| p.loss)
        .fold(f64::INFINITY, f64::min);
    let loss_range = if (max_loss - min_loss).abs() < 1e-9 {
        1.0
    } else {
        max_loss - min_loss
    };

    let x = |epoch: u32| -> f64 { pad_l + (epoch as f64 / max_epoch) * plot_w };
    let y = |loss: f64| -> f64 { pad_t + (1.0 - (loss - min_loss) / loss_range) * plot_h };

    let mut points = String::new();
    for p in curves {
        let _ = write!(points, "{:.1},{:.1} ", x(p.epoch), y(p.loss));
    }

    // Y-axis tick count
    let y_ticks = 5usize;
    let mut y_axis_labels = String::new();
    for i in 0..=y_ticks {
        let frac = i as f64 / y_ticks as f64;
        let loss_val = min_loss + frac * loss_range;
        let yy = pad_t + (1.0 - frac) * plot_h;
        let _ = write!(
            y_axis_labels,
            "<text x=\"{:.0}\" y=\"{:.0}\" text-anchor=\"end\" font-size=\"11\" fill=\"#7c6f64\">{:.4}</text>\
            <line x1=\"{:.0}\" y1=\"{:.0}\" x2=\"{:.0}\" y2=\"{:.0}\" stroke=\"#E8D4C4\" stroke-dasharray=\"4,3\"/>",
            pad_l - 8.0, yy + 4.0, loss_val,
            pad_l, yy, svg_w - pad_r, yy,
        );
    }

    // X-axis labels (up to 10 ticks)
    let x_tick_count = max_epoch.min(10.0) as usize;
    let mut x_axis_labels = String::new();
    if x_tick_count > 0 {
        let step = (max_epoch / x_tick_count as f64).ceil() as u32;
        let mut epoch = 0u32;
        while epoch <= max_epoch as u32 {
            let xx = x(epoch);
            let _ = write!(
                x_axis_labels,
                "<text x=\"{:.0}\" y=\"{:.0}\" text-anchor=\"middle\" font-size=\"11\" fill=\"#7c6f64\">{}</text>",
                xx, svg_h - 6.0, epoch,
            );
            epoch += step.max(1);
        }
    }

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_w} {svg_h}" class="loss-chart">
  <rect width="{svg_w}" height="{svg_h}" rx="8" fill="#FFFDF7"/>
  {y_axis_labels}
  {x_axis_labels}
  <!-- Axis lines -->
  <line x1="{pad_l}" y1="{pad_t}" x2="{pad_l}" y2="{bot}" stroke="#C4A484" stroke-width="1.5"/>
  <line x1="{pad_l}" y1="{bot}" x2="{right}" y2="{bot}" stroke="#C4A484" stroke-width="1.5"/>
  <!-- Loss curve -->
  <polyline points="{points}" fill="none" stroke="{primary}" stroke-width="2.5" stroke-linejoin="round" stroke-linecap="round"/>
  <!-- Axis titles -->
  <text x="{xlabel_x:.0}" y="{svg_h}" text-anchor="middle" font-size="12" fill="#5c5550" font-weight="600">Epoch</text>
  <text x="14" y="{ylabel_y:.0}" text-anchor="middle" font-size="12" fill="#5c5550" font-weight="600" transform="rotate(-90, 14, {ylabel_y:.0})">Loss</text>
</svg>"##,
        bot = pad_t + plot_h,
        right = svg_w - pad_r,
        xlabel_x = pad_l + plot_w / 2.0,
        ylabel_y = pad_t + plot_h / 2.0,
    )
}

// ── HTML Template ───────────────────────────────────────────────────────────

fn build_html(data: &TrainingReportData, branding: &Branding) -> Result<String, ReportError> {
    let primary = &branding.primary_color;
    let accent = &branding.accent_color;
    let company = &branding.company_name;

    // Optional logo
    let logo_html = if let Some(ref path) = branding.logo_path {
        match file_to_data_uri(path) {
            Ok(uri) => format!(r#"<img src="{}" alt="Logo" class="logo"/>"#, uri),
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    // Hyperparameters table rows
    let mut hp_rows = String::new();
    let mut keys: Vec<&String> = data.hyperparameters.keys().collect();
    keys.sort();
    for key in &keys {
        let val = &data.hyperparameters[*key];
        let _ = write!(
            hp_rows,
            r#"<tr><td class="hp-key">{}</td><td class="hp-val">{}</td></tr>"#,
            html_escape(key),
            html_escape(val),
        );
    }

    // Metrics bars
    let metrics_html = [
        metric_bar("Accuracy", data.accuracy, primary),
        metric_bar("Precision", data.precision, primary),
        metric_bar("Recall", data.recall, accent),
        metric_bar("F1 Score", data.f1, accent),
    ]
    .join("\n");

    // Loss curve SVG
    let loss_svg = build_loss_curve_svg(&data.training_curves, primary, accent);

    // Recommendations list
    let mut rec_items = String::new();
    for (i, rec) in data.recommendations.iter().enumerate() {
        let _ = write!(
            rec_items,
            r#"<li><span class="rec-num">{}</span> {}</li>"#,
            i + 1,
            html_escape(rec),
        );
    }

    let created = data.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let duration = format_duration(data.training_time_secs);

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"/>
<title>Training Report — {model_name}</title>
<style>
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');

  *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}

  html, body {{
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    font-size: 14px;
    line-height: 1.6;
    color: #3c3632;
    background: #FFFDF7;
  }}

  .page {{
    max-width: 820px;
    margin: 0 auto;
    padding: 48px 56px;
  }}

  /* ── Header ────────────────────────────────────── */
  .header {{
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 3px solid {primary};
    padding-bottom: 16px;
    margin-bottom: 32px;
  }}
  .header-left {{
    display: flex;
    align-items: center;
    gap: 16px;
  }}
  .logo {{ height: 48px; }}
  .header-title {{
    font-size: 22px;
    font-weight: 700;
    color: {primary};
  }}
  .header-subtitle {{
    font-size: 12px;
    color: #8a7f78;
    margin-top: 2px;
  }}
  .header-right {{
    text-align: right;
    font-size: 12px;
    color: #8a7f78;
  }}

  /* ── Section ───────────────────────────────────── */
  .section {{
    margin-bottom: 28px;
  }}
  .section-title {{
    font-size: 16px;
    font-weight: 700;
    color: {primary};
    border-left: 4px solid {accent};
    padding-left: 12px;
    margin-bottom: 14px;
  }}

  /* ── Overview grid ─────────────────────────────── */
  .overview-grid {{
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 14px;
  }}
  .overview-card {{
    background: #FFF;
    border: 1px solid #E8D4C4;
    border-radius: 8px;
    padding: 14px 16px;
  }}
  .overview-label {{
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: #8a7f78;
    margin-bottom: 4px;
  }}
  .overview-value {{
    font-size: 18px;
    font-weight: 700;
    color: #3c3632;
  }}

  /* ── Hyperparameters table ─────────────────────── */
  .hp-table {{
    width: 100%;
    border-collapse: collapse;
  }}
  .hp-table th {{
    text-align: left;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #8a7f78;
    padding: 6px 12px;
    border-bottom: 2px solid #E8D4C4;
  }}
  .hp-table td {{
    padding: 8px 12px;
    border-bottom: 1px solid #f0e6dc;
  }}
  .hp-key {{
    font-weight: 600;
    color: #5c5550;
  }}
  .hp-val {{
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 13px;
    color: {primary};
  }}

  /* ── Metric bars ───────────────────────────────── */
  .metric-row {{
    display: flex;
    align-items: center;
    margin-bottom: 10px;
  }}
  .metric-label {{
    width: 100px;
    font-weight: 600;
    font-size: 13px;
    color: #5c5550;
  }}
  .metric-bar-track {{
    flex: 1;
    height: 14px;
    background: #f0e6dc;
    border-radius: 7px;
    overflow: hidden;
    margin: 0 12px;
  }}
  .metric-bar-fill {{
    height: 100%;
    border-radius: 7px;
    transition: width 0.4s;
  }}
  .metric-value {{
    width: 72px;
    text-align: right;
    font-weight: 700;
    font-size: 13px;
    color: #3c3632;
  }}

  /* ── Loss chart ────────────────────────────────── */
  .chart-container {{
    background: #FFF;
    border: 1px solid #E8D4C4;
    border-radius: 10px;
    padding: 18px;
  }}
  .loss-chart {{
    width: 100%;
    height: auto;
  }}
  .chart-placeholder {{
    text-align: center;
    padding: 40px;
    color: #8a7f78;
    font-style: italic;
  }}

  /* ── Recommendations ───────────────────────────── */
  .rec-list {{
    list-style: none;
    padding: 0;
  }}
  .rec-list li {{
    background: #FFF;
    border: 1px solid #E8D4C4;
    border-radius: 8px;
    padding: 12px 16px;
    margin-bottom: 8px;
    font-size: 13px;
    line-height: 1.5;
  }}
  .rec-num {{
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: {primary};
    color: #FFF;
    font-size: 11px;
    font-weight: 700;
    margin-right: 8px;
  }}

  /* ── Final Loss callout ────────────────────────── */
  .loss-callout {{
    display: inline-block;
    background: {primary};
    color: #FFF;
    font-size: 20px;
    font-weight: 700;
    padding: 6px 18px;
    border-radius: 8px;
    margin-top: 6px;
  }}

  /* ── Footer ────────────────────────────────────── */
  .footer {{
    margin-top: 36px;
    padding-top: 14px;
    border-top: 1px solid #E8D4C4;
    font-size: 11px;
    color: #8a7f78;
    display: flex;
    justify-content: space-between;
  }}
</style>
</head>
<body>
<div class="page">
  <!-- Header -->
  <div class="header">
    <div class="header-left">
      {logo_html}
      <div>
        <div class="header-title">Training Report</div>
        <div class="header-subtitle">{company} &middot; Prometheus Platform</div>
      </div>
    </div>
    <div class="header-right">
      Generated: {created}<br/>
      Model: <strong>{model_name}</strong>
    </div>
  </div>

  <!-- Overview -->
  <div class="section">
    <div class="section-title">Overview</div>
    <div class="overview-grid">
      <div class="overview-card">
        <div class="overview-label">Model Name</div>
        <div class="overview-value">{model_name}</div>
      </div>
      <div class="overview-card">
        <div class="overview-label">Architecture</div>
        <div class="overview-value">{architecture}</div>
      </div>
      <div class="overview-card">
        <div class="overview-label">Dataset</div>
        <div class="overview-value">{dataset_name}</div>
      </div>
      <div class="overview-card">
        <div class="overview-label">Equipment Type</div>
        <div class="overview-value">{equipment_type}</div>
      </div>
      <div class="overview-card">
        <div class="overview-label">Epochs</div>
        <div class="overview-value">{epochs}</div>
      </div>
      <div class="overview-card">
        <div class="overview-label">Training Time</div>
        <div class="overview-value">{duration}</div>
      </div>
    </div>
  </div>

  <!-- Hyperparameters -->
  <div class="section">
    <div class="section-title">Hyperparameters</div>
    <table class="hp-table">
      <thead><tr><th>Parameter</th><th>Value</th></tr></thead>
      <tbody>
        {hp_rows}
      </tbody>
    </table>
  </div>

  <!-- Metrics -->
  <div class="section">
    <div class="section-title">Final Metrics</div>
    {metrics_html}
    <div style="margin-top: 12px;">
      <span style="font-size:13px;color:#5c5550;font-weight:600;">Final Loss</span><br/>
      <span class="loss-callout">{loss:.6}</span>
    </div>
  </div>

  <!-- Training Curve -->
  <div class="section">
    <div class="section-title">Training Loss Curve</div>
    <div class="chart-container">
      {loss_svg}
    </div>
  </div>

  <!-- Recommendations -->
  <div class="section">
    <div class="section-title">Recommendations</div>
    <ul class="rec-list">
      {rec_items}
    </ul>
  </div>

  <!-- Footer -->
  <div class="footer">
    <span>&copy; {company} &mdash; Prometheus Platform</span>
    <span>Confidential &middot; Internal Use Only</span>
  </div>
</div>
</body>
</html>"##,
        model_name = html_escape(&data.model_name),
        architecture = html_escape(&data.architecture),
        dataset_name = html_escape(&data.dataset_name),
        equipment_type = html_escape(&data.equipment_type),
        epochs = data.epochs,
        loss = data.loss,
    );

    Ok(html)
}

/// Minimal HTML entity escaping.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn sample_report_data() -> TrainingReportData {
        let mut hp = HashMap::new();
        hp.insert("learning_rate".to_string(), "0.001".to_string());
        hp.insert("batch_size".to_string(), "32".to_string());

        TrainingReportData {
            model_name: "AnomalyDetector-v2".to_string(),
            architecture: "ResNet-50".to_string(),
            hyperparameters: hp,
            epochs: 100,
            training_time_secs: 7384.5,
            loss: 0.0234,
            accuracy: 0.96,
            precision: 0.94,
            recall: 0.92,
            f1: 0.93,
            training_curves: vec![
                EpochLossPair { epoch: 0, loss: 1.5 },
                EpochLossPair { epoch: 25, loss: 0.8 },
                EpochLossPair { epoch: 50, loss: 0.3 },
                EpochLossPair { epoch: 75, loss: 0.1 },
                EpochLossPair { epoch: 100, loss: 0.0234 },
            ],
            recommendations: vec![
                "Consider increasing dropout for better generalization".to_string(),
                "Try learning rate scheduling".to_string(),
            ],
            dataset_name: "vibration-dataset-2024".to_string(),
            equipment_type: "CNC Mill".to_string(),
            created_at: Utc::now(),
        }
    }

    // ── slug tests ──────────────────────────────────────

    #[test]
    fn slug_lowercases_alpha() {
        assert_eq!(slug("Hello"), "hello");
    }

    #[test]
    fn slug_replaces_spaces() {
        assert_eq!(slug("my model"), "my-model");
    }

    #[test]
    fn slug_replaces_special_chars() {
        assert_eq!(slug("model@v2!"), "model-v2-");
    }

    #[test]
    fn slug_preserves_digits() {
        assert_eq!(slug("model123"), "model123");
    }

    // ── format_duration tests ───────────────────────────

    #[test]
    fn format_duration_seconds_only() {
        assert_eq!(format_duration(45.7), "45.7s");
    }

    #[test]
    fn format_duration_minutes_and_seconds() {
        // 125 seconds = 2m 5s
        assert_eq!(format_duration(125.0), "2m 5s");
    }

    #[test]
    fn format_duration_hours_minutes_seconds() {
        // 7384.5 seconds = 2h 3m 4s
        assert_eq!(format_duration(7384.5), "2h 3m 4s");
    }

    #[test]
    fn format_duration_exact_hour() {
        assert_eq!(format_duration(3600.0), "1h 0m 0s");
    }

    #[test]
    fn format_duration_zero() {
        assert_eq!(format_duration(0.0), "0.0s");
    }

    // ── html_escape tests ───────────────────────────────

    #[test]
    fn html_escape_ampersand() {
        assert_eq!(html_escape("A & B"), "A &amp; B");
    }

    #[test]
    fn html_escape_angle_brackets() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn html_escape_quotes() {
        assert_eq!(html_escape(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn html_escape_single_quotes() {
        assert_eq!(html_escape("it's"), "it&#39;s");
    }

    #[test]
    fn html_escape_no_change() {
        assert_eq!(html_escape("plain text"), "plain text");
    }

    // ── metric_bar tests ────────────────────────────────

    #[test]
    fn metric_bar_contains_label() {
        let bar = metric_bar("Accuracy", 0.95, "#14b8a6");
        assert!(bar.contains("Accuracy"));
    }

    #[test]
    fn metric_bar_percentage_clamped_high() {
        let bar = metric_bar("Over", 1.5, "#000");
        // Should be clamped to 100.0%
        assert!(bar.contains("100.0%"));
    }

    #[test]
    fn metric_bar_percentage_clamped_low() {
        let bar = metric_bar("Under", -0.5, "#000");
        // Should be clamped to 0.0%
        assert!(bar.contains("0.0%"));
    }

    #[test]
    fn metric_bar_value_display() {
        let bar = metric_bar("F1", 0.93, "#C4A484");
        assert!(bar.contains("93.00%"));
    }

    #[test]
    fn metric_bar_contains_color() {
        let bar = metric_bar("Test", 0.5, "#ff0000");
        assert!(bar.contains("#ff0000"));
    }

    // ── build_loss_curve_svg tests ──────────────────────

    #[test]
    fn loss_curve_svg_empty_data() {
        let svg = build_loss_curve_svg(&[], "#14b8a6", "#C4A484");
        assert!(svg.contains("No training curve data available"));
    }

    #[test]
    fn loss_curve_svg_contains_polyline() {
        let curves = vec![
            EpochLossPair { epoch: 0, loss: 1.0 },
            EpochLossPair { epoch: 10, loss: 0.5 },
        ];
        let svg = build_loss_curve_svg(&curves, "#14b8a6", "#C4A484");
        assert!(svg.contains("<polyline"));
    }

    #[test]
    fn loss_curve_svg_uses_primary_color() {
        let curves = vec![
            EpochLossPair { epoch: 0, loss: 1.0 },
            EpochLossPair { epoch: 5, loss: 0.2 },
        ];
        let svg = build_loss_curve_svg(&curves, "#ff5500", "#aaa");
        assert!(svg.contains("#ff5500"));
    }

    #[test]
    fn loss_curve_svg_single_point() {
        // Single data point — loss_range should become 1.0 to avoid division by zero
        let curves = vec![EpochLossPair { epoch: 0, loss: 0.5 }];
        let svg = build_loss_curve_svg(&curves, "#14b8a6", "#C4A484");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<polyline"));
    }

    #[test]
    fn loss_curve_svg_contains_axis_labels() {
        let curves = vec![
            EpochLossPair { epoch: 0, loss: 2.0 },
            EpochLossPair { epoch: 50, loss: 0.1 },
        ];
        let svg = build_loss_curve_svg(&curves, "#14b8a6", "#C4A484");
        assert!(svg.contains("Epoch"));
        assert!(svg.contains("Loss"));
    }

    // ── build_html integration tests ────────────────────

    #[test]
    fn build_html_contains_model_name() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("AnomalyDetector-v2"));
    }

    #[test]
    fn build_html_contains_architecture() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("ResNet-50"));
    }

    #[test]
    fn build_html_contains_dataset_name() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("vibration-dataset-2024"));
    }

    #[test]
    fn build_html_contains_equipment_type() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("CNC Mill"));
    }

    #[test]
    fn build_html_contains_hyperparameters() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("learning_rate"));
        assert!(html.contains("0.001"));
        assert!(html.contains("batch_size"));
        assert!(html.contains("32"));
    }

    #[test]
    fn build_html_contains_metrics() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("Accuracy"));
        assert!(html.contains("Precision"));
        assert!(html.contains("Recall"));
        assert!(html.contains("F1 Score"));
    }

    #[test]
    fn build_html_contains_training_duration() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("2h 3m 4s"));
    }

    #[test]
    fn build_html_contains_recommendations() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("Consider increasing dropout"));
        assert!(html.contains("learning rate scheduling"));
    }

    #[test]
    fn build_html_contains_company_name() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("AutomataNexus"));
    }

    #[test]
    fn build_html_contains_loss_curve_svg() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("<svg"));
        assert!(html.contains("<polyline"));
    }

    #[test]
    fn build_html_escapes_special_chars() {
        let mut data = sample_report_data();
        data.model_name = "model<script>&test".to_string();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("model&lt;script&gt;&amp;test"));
        assert!(!html.contains("<script>"));
    }

    // ── Additional slug tests ──────────────────────────────

    #[test]
    fn slug_empty_string() {
        assert_eq!(slug(""), "");
    }

    #[test]
    fn slug_unicode_chars() {
        // Non-alphanumeric unicode should become hyphens
        assert_eq!(slug("model_v2"), "model-v2");
    }

    #[test]
    fn slug_mixed_case_numbers() {
        assert_eq!(slug("ResNet50"), "resnet50");
    }

    // ── Additional format_duration tests ───────────────────

    #[test]
    fn format_duration_sub_second() {
        assert_eq!(format_duration(0.5), "0.5s");
    }

    #[test]
    fn format_duration_exact_minute() {
        assert_eq!(format_duration(60.0), "1m 0s");
    }

    #[test]
    fn format_duration_large_value() {
        // 86400 seconds = 24h 0m 0s
        assert_eq!(format_duration(86400.0), "24h 0m 0s");
    }

    // ── Additional html_escape tests ───────────────────────

    #[test]
    fn html_escape_empty() {
        assert_eq!(html_escape(""), "");
    }

    #[test]
    fn html_escape_all_entities_combined() {
        assert_eq!(
            html_escape(r#"<a href="x">&'y"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;y"
        );
    }

    // ── Additional metric_bar tests ────────────────────────

    #[test]
    fn metric_bar_zero_value() {
        let bar = metric_bar("Zero", 0.0, "#000");
        assert!(bar.contains("0.00%"));
    }

    #[test]
    fn metric_bar_full_value() {
        let bar = metric_bar("Full", 1.0, "#000");
        assert!(bar.contains("100.00%"));
    }

    #[test]
    fn metric_bar_html_structure() {
        let bar = metric_bar("Test", 0.5, "#abc");
        assert!(bar.contains("metric-row"));
        assert!(bar.contains("metric-label"));
        assert!(bar.contains("metric-bar-track"));
        assert!(bar.contains("metric-bar-fill"));
        assert!(bar.contains("metric-value"));
    }

    // ── Additional build_loss_curve_svg tests ──────────────

    #[test]
    fn loss_curve_svg_constant_loss() {
        // All loss values the same — should not panic (loss_range fallback to 1.0)
        let curves = vec![
            EpochLossPair { epoch: 0, loss: 0.5 },
            EpochLossPair { epoch: 10, loss: 0.5 },
            EpochLossPair { epoch: 20, loss: 0.5 },
        ];
        let svg = build_loss_curve_svg(&curves, "#14b8a6", "#C4A484");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<polyline"));
    }

    #[test]
    fn loss_curve_svg_many_epochs() {
        let curves: Vec<EpochLossPair> = (0..100)
            .map(|e| EpochLossPair {
                epoch: e,
                loss: 1.0 / (e as f64 + 1.0),
            })
            .collect();
        let svg = build_loss_curve_svg(&curves, "#14b8a6", "#C4A484");
        assert!(svg.contains("<svg"));
    }

    // ── Additional build_html tests ────────────────────────

    #[test]
    fn build_html_is_well_formed() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn build_html_no_recommendations() {
        let mut data = sample_report_data();
        data.recommendations.clear();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        // Should still render fine, just no recommendation items
        assert!(html.contains("Recommendations"));
    }

    #[test]
    fn build_html_empty_training_curves() {
        let mut data = sample_report_data();
        data.training_curves.clear();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("No training curve data available"));
    }

    #[test]
    fn build_html_with_custom_branding() {
        let data = sample_report_data();
        let branding = Branding {
            logo_path: None,
            company_name: "TestOrg".to_string(),
            primary_color: "#ff0000".to_string(),
            accent_color: "#00ff00".to_string(),
        };
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("TestOrg"));
        assert!(html.contains("#ff0000"));
        assert!(html.contains("#00ff00"));
    }

    #[test]
    fn build_html_contains_loss_value() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("0.023400"));
    }

    #[test]
    fn build_html_contains_epoch_count() {
        let data = sample_report_data();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("100"));
    }

    #[test]
    fn build_html_no_hyperparameters() {
        let mut data = sample_report_data();
        data.hyperparameters.clear();
        let branding = Branding::default();
        let html = build_html(&data, &branding).unwrap();
        assert!(html.contains("Hyperparameters"));
    }
}
