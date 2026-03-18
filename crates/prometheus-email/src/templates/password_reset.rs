// ============================================================================
// File: password_reset.rs
// Description: Password reset email template with expiring reset link
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use super::layout;

pub fn render(username: &str, reset_url: &str, expires_minutes: u32) -> String {
    let body = format!(
        r#"<h1 style="margin:0 0 8px;font-size:24px;font-weight:700;color:#111827;">Reset Your Password</h1>
<p style="margin:0 0 20px;font-size:15px;color:#6b7280;">We received a password reset request for your account.</p>

<p style="margin:0 0 20px;font-size:15px;color:#374151;line-height:1.6;">
  Hi {username}, click the button below to choose a new password. This link expires in <strong>{expires_minutes} minutes</strong>.
</p>

{btn}

<p style="margin:20px 0 12px;font-size:13px;color:#6b7280;">If the button doesn't work, copy and paste this URL into your browser:</p>
<p style="margin:0 0 20px;padding:12px;background-color:#FFFDF7;border:1px solid #E8D4C4;border-radius:6px;font-size:12px;color:#14b8a6;word-break:break-all;font-family:monospace;">
  {reset_url}
</p>

<p style="margin:0;font-size:13px;color:#9ca3af;line-height:1.5;">
  If you did not request a password reset, please ignore this email. Your password will remain unchanged.
  If you're concerned about unauthorized access, contact support immediately.
</p>"#,
        btn = layout::button("Reset Password", reset_url),
    );

    layout::wrap(
        "Reset Your Password",
        "A password reset was requested for your Prometheus account.",
        &body,
        "",
    )
}
