// ============================================================================
// File: welcome.rs
// Description: Welcome email template sent after successful account creation
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use super::layout;

pub fn render(username: &str, login_url: &str) -> String {
    let body = format!(
        r#"<h1 style="margin:0 0 8px;font-size:24px;font-weight:700;color:#111827;">Welcome to Prometheus</h1>
<p style="margin:0 0 20px;font-size:15px;color:#6b7280;">Your account has been created successfully.</p>

<p style="margin:0 0 6px;font-size:15px;color:#111827;">Hello <strong>{username}</strong>,</p>
<p style="margin:0 0 20px;font-size:15px;color:#374151;line-height:1.6;">
  You now have access to the Prometheus AI edge ML platform. Train predictive maintenance models,
  deploy to edge controllers, and monitor your building equipment — all from one place.
</p>

<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="margin:20px 0;background-color:#FFFDF7;border-radius:8px;border:1px solid #E8D4C4;">
<tr><td style="padding:16px 20px;">
  <p style="margin:0 0 4px;font-size:12px;color:#6b7280;text-transform:uppercase;letter-spacing:0.5px;">Your account</p>
  <p style="margin:0;font-size:15px;color:#111827;font-weight:600;">{username}</p>
</td></tr>
</table>

{btn}

<p style="margin:20px 0 0;font-size:13px;color:#6b7280;line-height:1.5;">
  Need help getting started? Visit our documentation or reach out to support.
</p>"#,
        btn = layout::button("Sign In to Prometheus", login_url),
    );

    layout::wrap(
        "Welcome to Prometheus",
        &format!("Welcome, {}! Your Prometheus account is ready.", username),
        &body,
        "",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_contains_username() {
        let html = render("alice", "https://example.com/login");
        assert!(html.contains("alice"));
    }

    #[test]
    fn render_contains_login_url() {
        let html = render("bob", "https://prometheus.example.com/login");
        assert!(html.contains("https://prometheus.example.com/login"));
    }

    #[test]
    fn render_contains_welcome_title() {
        let html = render("user", "https://example.com/login");
        assert!(html.contains("Welcome to Prometheus"));
    }

    #[test]
    fn render_contains_sign_in_button() {
        let html = render("user", "https://example.com/login");
        assert!(html.contains("Sign In to Prometheus"));
    }

    #[test]
    fn render_is_valid_html() {
        let html = render("user", "https://example.com/login");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn render_preheader_contains_username() {
        let html = render("charlie", "https://example.com/login");
        assert!(html.contains("Welcome, charlie!"));
    }
}
