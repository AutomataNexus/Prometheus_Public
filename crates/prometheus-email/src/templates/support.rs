// ============================================================================
// File: support.rs
// Description: Support ticket confirmation and internal notification email templates
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
use super::layout;

/// Confirmation sent to the user when they submit a support inquiry.
pub fn render_confirmation(
    username: &str,
    ticket_id: &str,
    subject: &str,
    message_preview: &str,
) -> String {
    let preview = if message_preview.len() > 200 {
        format!("{}...", &message_preview[..200])
    } else {
        message_preview.to_string()
    };

    let body = format!(
        r#"<h1 style="margin:0 0 8px;font-size:24px;font-weight:700;color:#111827;">We Got Your Message</h1>
<p style="margin:0 0 20px;font-size:15px;color:#6b7280;">Our team will respond within 24 hours.</p>

<p style="margin:0 0 20px;font-size:15px;color:#374151;line-height:1.6;">
  Hi {username}, thanks for reaching out. Here's a summary of your inquiry:
</p>

<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="margin:20px 0;background-color:#FFFDF7;border:1px solid #E8D4C4;border-radius:8px;">
<tr><td style="padding:16px 20px;border-bottom:1px solid #E8D4C4;">
  <p style="margin:0 0 2px;font-size:12px;color:#6b7280;text-transform:uppercase;letter-spacing:0.5px;">Ticket</p>
  <p style="margin:0;font-size:15px;font-weight:600;color:#14b8a6;font-family:monospace;">{ticket_id}</p>
</td></tr>
<tr><td style="padding:16px 20px;border-bottom:1px solid #E8D4C4;">
  <p style="margin:0 0 2px;font-size:12px;color:#6b7280;text-transform:uppercase;letter-spacing:0.5px;">Subject</p>
  <p style="margin:0;font-size:15px;color:#111827;font-weight:600;">{subject}</p>
</td></tr>
<tr><td style="padding:16px 20px;">
  <p style="margin:0 0 2px;font-size:12px;color:#6b7280;text-transform:uppercase;letter-spacing:0.5px;">Message</p>
  <p style="margin:0;font-size:14px;color:#374151;line-height:1.5;white-space:pre-wrap;">{preview}</p>
</td></tr>
</table>

<p style="margin:0;font-size:13px;color:#6b7280;line-height:1.5;">
  You can reply to this email to add additional context. Reference your ticket ID <strong>{ticket_id}</strong> in any follow-up communication.
</p>"#,
    );

    layout::wrap(
        &format!("Support Ticket: {}", ticket_id),
        &format!("We received your support request: {}", subject),
        &body,
        "",
    )
}

/// Response sent to the user when support replies.
pub fn render_response(
    username: &str,
    ticket_id: &str,
    subject: &str,
    response_body: &str,
    responder_name: &str,
) -> String {
    let body = format!(
        r#"<h1 style="margin:0 0 8px;font-size:24px;font-weight:700;color:#111827;">Response to Your Inquiry</h1>
<p style="margin:0 0 20px;font-size:15px;color:#6b7280;">Ticket: {ticket_id} &middot; {subject}</p>

<p style="margin:0 0 12px;font-size:15px;color:#374151;">Hi {username},</p>

<div style="margin:16px 0;padding:20px;background-color:#FFFDF7;border-left:3px solid #14b8a6;border-radius:0 8px 8px 0;">
  <p style="margin:0;font-size:14px;color:#374151;line-height:1.6;white-space:pre-wrap;">{response_body}</p>
  <p style="margin:12px 0 0;font-size:13px;color:#6b7280;">&mdash; {responder_name}, Automata Controls Support</p>
</div>

<p style="margin:20px 0 0;font-size:13px;color:#6b7280;">
  Reply to this email to continue the conversation.
</p>"#,
    );

    layout::wrap(
        &format!("Re: {}", subject),
        &format!("{} replied to your support ticket {}", responder_name, ticket_id),
        &body,
        "",
    )
}
