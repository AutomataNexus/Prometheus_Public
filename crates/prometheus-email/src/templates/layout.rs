// ============================================================================
// File: layout.rs
// Description: NexusEdge branded HTML email layout wrapper with responsive table design
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
/// Wrap email body content in the branded NexusEdge layout.
/// Uses table-based layout for maximum email client compatibility.
pub fn wrap(title: &str, preheader: &str, body_html: &str, footer_extra: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="X-UA-Compatible" content="IE=edge">
<title>{title}</title>
<style>
  body, table, td {{ margin:0; padding:0; }}
  img {{ border:0; display:block; }}
  a {{ color:#14b8a6; text-decoration:none; }}
  a:hover {{ text-decoration:underline; }}
  @media only screen and (max-width:620px) {{
    .container {{ width:100% !important; padding:16px !important; }}
    .btn {{ width:100% !important; }}
  }}
</style>
<!--[if mso]><style>body,table,td{{font-family:Arial,sans-serif !important;}}</style><![endif]-->
</head>
<body style="margin:0;padding:0;background-color:#FFFDF7;font-family:'Inter',-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;">
<!-- Preheader (hidden preview text) -->
<div style="display:none;max-height:0;overflow:hidden;mso-hide:all;">{preheader}</div>

<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#FFFDF7;">
<tr><td align="center" style="padding:32px 16px;">

<!-- Container -->
<table role="presentation" class="container" width="580" cellpadding="0" cellspacing="0" style="background-color:#FAF8F5;border:1px solid #E8D4C4;border-radius:12px;overflow:hidden;">

<!-- Header -->
<tr>
<td style="background:linear-gradient(135deg,#14b8a6 0%,#0d9488 100%);padding:28px 40px;">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
  <tr>
    <td style="font-size:22px;font-weight:700;color:#ffffff;letter-spacing:-0.5px;">
      Prometheus
    </td>
    <td align="right" style="font-size:12px;color:rgba(255,255,255,0.8);text-transform:uppercase;letter-spacing:1px;">
      AI-Forged Edge Intelligence
    </td>
  </tr>
  </table>
</td>
</tr>

<!-- Body -->
<tr>
<td style="padding:36px 40px 24px;">
  {body_html}
</td>
</tr>

<!-- Footer -->
<tr>
<td style="padding:0 40px 32px;">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
  <tr><td style="border-top:1px solid #E8D4C4;padding-top:20px;">
    {footer_extra}
    <p style="margin:8px 0 0;font-size:12px;color:#6b7280;line-height:1.5;">
      Automata Controls &middot; Building Intelligence Platform<br>
      This is an automated message from Prometheus. Do not reply directly.
    </p>
  </td></tr>
  </table>
</td>
</tr>

</table>
<!-- /Container -->

</td></tr>
</table>
</body>
</html>"##
    )
}

/// Render a primary CTA button.
pub fn button(text: &str, url: &str) -> String {
    format!(
        r#"<table role="presentation" cellpadding="0" cellspacing="0" style="margin:24px 0;">
<tr><td align="center" style="background-color:#14b8a6;border-radius:8px;">
  <a href="{url}" target="_blank" class="btn" style="display:inline-block;padding:14px 32px;font-size:15px;font-weight:600;color:#ffffff;text-decoration:none;border-radius:8px;">
    {text}
  </a>
</td></tr>
</table>"#
    )
}

/// Render a secondary/ghost button.
pub fn button_ghost(text: &str, url: &str) -> String {
    format!(
        r#"<table role="presentation" cellpadding="0" cellspacing="0" style="margin:16px 0;">
<tr><td align="center" style="border:2px solid #E8D4C4;border-radius:8px;">
  <a href="{url}" target="_blank" style="display:inline-block;padding:12px 28px;font-size:14px;font-weight:600;color:#111827;text-decoration:none;">
    {text}
  </a>
</td></tr>
</table>"#
    )
}

/// Render a code/token display box.
pub fn code_box(code: &str) -> String {
    format!(
        r#"<div style="margin:24px 0;padding:20px;background-color:#FFFDF7;border:2px dashed #E8D4C4;border-radius:8px;text-align:center;">
  <span style="font-family:'JetBrains Mono',Consolas,monospace;font-size:32px;font-weight:700;color:#111827;letter-spacing:6px;">{code}</span>
</div>"#
    )
}

/// Render a metric row for reports (label + value).
pub fn metric_row(label: &str, value: &str, color: &str) -> String {
    format!(
        r#"<tr>
  <td style="padding:10px 12px;font-size:14px;color:#6b7280;border-bottom:1px solid #E8D4C4;">{label}</td>
  <td align="right" style="padding:10px 12px;font-size:14px;font-weight:600;color:{color};border-bottom:1px solid #E8D4C4;">{value}</td>
</tr>"#
    )
}

/// Render a section heading inside the email body.
pub fn heading(text: &str) -> String {
    format!(
        r#"<h2 style="margin:28px 0 12px;font-size:16px;font-weight:600;color:#111827;text-transform:uppercase;letter-spacing:0.5px;border-bottom:2px solid #14b8a6;padding-bottom:8px;display:inline-block;">{text}</h2>"#
    )
}

/// Render a warning/alert banner.
pub fn alert_banner(text: &str) -> String {
    format!(
        r#"<div style="margin:16px 0;padding:14px 18px;background-color:#FEF2F2;border-left:4px solid #EF4444;border-radius:0 8px 8px 0;font-size:14px;color:#991B1B;">
  <strong style="display:block;margin-bottom:4px;">Security Alert</strong>
  {text}
</div>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── wrap() tests ───────────────────────────────────────

    #[test]
    fn wrap_contains_title() {
        let html = wrap("Test Title", "preview", "<p>body</p>", "");
        assert!(html.contains("<title>Test Title</title>"));
    }

    #[test]
    fn wrap_contains_preheader() {
        let html = wrap("Title", "This is the preview text", "<p>body</p>", "");
        assert!(html.contains("This is the preview text"));
    }

    #[test]
    fn wrap_contains_body_html() {
        let html = wrap("Title", "pre", "<p>Hello World</p>", "");
        assert!(html.contains("<p>Hello World</p>"));
    }

    #[test]
    fn wrap_contains_footer_extra() {
        let html = wrap("Title", "pre", "<p>body</p>", "<p>Extra footer</p>");
        assert!(html.contains("<p>Extra footer</p>"));
    }

    #[test]
    fn wrap_contains_prometheus_branding() {
        let html = wrap("Title", "pre", "<p>body</p>", "");
        assert!(html.contains("Prometheus"));
        assert!(html.contains("Automata Controls"));
    }

    #[test]
    fn wrap_is_valid_html() {
        let html = wrap("Title", "pre", "<p>body</p>", "");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("</body>"));
    }

    #[test]
    fn wrap_contains_responsive_meta() {
        let html = wrap("Title", "pre", "<p>body</p>", "");
        assert!(html.contains("viewport"));
    }

    // ── button() tests ─────────────────────────────────────

    #[test]
    fn button_contains_text() {
        let btn = button("Click Me", "https://example.com");
        assert!(btn.contains("Click Me"));
    }

    #[test]
    fn button_contains_url() {
        let btn = button("Click", "https://example.com/action");
        assert!(btn.contains("https://example.com/action"));
    }

    #[test]
    fn button_has_anchor_tag() {
        let btn = button("Text", "https://url.com");
        assert!(btn.contains("<a href="));
    }

    #[test]
    fn button_has_primary_color() {
        let btn = button("Text", "https://url.com");
        assert!(btn.contains("#14b8a6"));
    }

    // ── button_ghost() tests ───────────────────────────────

    #[test]
    fn button_ghost_contains_text() {
        let btn = button_ghost("View More", "https://example.com");
        assert!(btn.contains("View More"));
    }

    #[test]
    fn button_ghost_contains_url() {
        let btn = button_ghost("View", "https://example.com/more");
        assert!(btn.contains("https://example.com/more"));
    }

    #[test]
    fn button_ghost_has_border_style() {
        let btn = button_ghost("Text", "https://url.com");
        assert!(btn.contains("border:2px solid"));
    }

    // ── code_box() tests ───────────────────────────────────

    #[test]
    fn code_box_contains_code() {
        let box_html = code_box("ABC123");
        assert!(box_html.contains("ABC123"));
    }

    #[test]
    fn code_box_has_monospace_font() {
        let box_html = code_box("123456");
        assert!(box_html.contains("monospace"));
    }

    #[test]
    fn code_box_has_letter_spacing() {
        let box_html = code_box("ABCDEF");
        assert!(box_html.contains("letter-spacing"));
    }

    // ── metric_row() tests ─────────────────────────────────

    #[test]
    fn metric_row_contains_label() {
        let row = metric_row("Requests", "1000", "#111");
        assert!(row.contains("Requests"));
    }

    #[test]
    fn metric_row_contains_value() {
        let row = metric_row("Requests", "1000", "#111");
        assert!(row.contains("1000"));
    }

    #[test]
    fn metric_row_contains_color() {
        let row = metric_row("Metric", "42", "#ff0000");
        assert!(row.contains("#ff0000"));
    }

    #[test]
    fn metric_row_is_table_row() {
        let row = metric_row("Label", "Value", "#000");
        assert!(row.contains("<tr>"));
        assert!(row.contains("</tr>"));
    }

    // ── heading() tests ────────────────────────────────────

    #[test]
    fn heading_contains_text() {
        let h = heading("Overview");
        assert!(h.contains("Overview"));
    }

    #[test]
    fn heading_is_h2() {
        let h = heading("Title");
        assert!(h.contains("<h2"));
        assert!(h.contains("</h2>"));
    }

    #[test]
    fn heading_has_primary_color_border() {
        let h = heading("Test");
        assert!(h.contains("#14b8a6"));
    }

    // ── alert_banner() tests ───────────────────────────────

    #[test]
    fn alert_banner_contains_text() {
        let banner = alert_banner("Something went wrong");
        assert!(banner.contains("Something went wrong"));
    }

    #[test]
    fn alert_banner_contains_security_alert_label() {
        let banner = alert_banner("issue");
        assert!(banner.contains("Security Alert"));
    }

    #[test]
    fn alert_banner_has_red_border() {
        let banner = alert_banner("test");
        assert!(banner.contains("#EF4444"));
    }
}
