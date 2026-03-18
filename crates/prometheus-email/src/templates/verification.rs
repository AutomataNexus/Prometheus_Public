// ============================================================================
// File: verification.rs
// Description: Email verification template with code box and confirm button
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use super::layout;

pub fn render(username: &str, code: &str, verify_url: &str, expires_minutes: u32) -> String {
    let body = format!(
        r#"<h1 style="margin:0 0 8px;font-size:24px;font-weight:700;color:#111827;">Verify Your Email</h1>
<p style="margin:0 0 20px;font-size:15px;color:#6b7280;">One quick step to secure your account.</p>

<p style="margin:0 0 20px;font-size:15px;color:#374151;line-height:1.6;">
  Hi {username}, enter this verification code to confirm your email address:
</p>

{code_box}

<p style="margin:0 0 8px;font-size:14px;color:#6b7280;text-align:center;">
  This code expires in <strong>{expires_minutes} minutes</strong>.
</p>

<p style="margin:20px 0 8px;font-size:14px;color:#6b7280;text-align:center;">Or click the button below:</p>
{btn}

<p style="margin:20px 0 0;font-size:13px;color:#9ca3af;line-height:1.5;">
  If you didn't create a Prometheus account, you can safely ignore this email.
</p>"#,
        code_box = layout::code_box(code),
        btn = layout::button("Verify Email Address", verify_url),
    );

    layout::wrap(
        "Verify Your Email",
        &format!("Your Prometheus verification code is: {}", code),
        &body,
        "",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_contains_username() {
        let html = render("alice", "123456", "https://example.com/verify?code=123456", 30);
        assert!(html.contains("alice"));
    }

    #[test]
    fn render_contains_verification_code() {
        let html = render("user", "ABCDEF", "https://example.com/verify", 15);
        assert!(html.contains("ABCDEF"));
    }

    #[test]
    fn render_contains_verify_url() {
        let html = render("user", "123", "https://prom.example.com/verify?code=123", 10);
        assert!(html.contains("https://prom.example.com/verify?code=123"));
    }

    #[test]
    fn render_contains_expiry_time() {
        let html = render("user", "123", "https://example.com/verify", 45);
        assert!(html.contains("45 minutes"));
    }

    #[test]
    fn render_contains_verify_title() {
        let html = render("user", "123", "https://example.com/verify", 30);
        assert!(html.contains("Verify Your Email"));
    }

    #[test]
    fn render_preheader_contains_code() {
        let html = render("user", "XYZABC", "https://example.com/verify", 30);
        assert!(html.contains("Your Prometheus verification code is: XYZABC"));
    }

    #[test]
    fn render_contains_verify_button() {
        let html = render("user", "123", "https://example.com/verify", 30);
        assert!(html.contains("Verify Email Address"));
    }
}
