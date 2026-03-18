# prometheus-email

Transactional email service for the Prometheus platform. Sends branded HTML emails through the Resend API with a table-based responsive layout optimized for email client compatibility.

## Email Types

| Method | Template | Trigger |
|--------|----------|---------|
| `send_welcome` | Account welcome with sign-in button | User registration |
| `send_verification` | Verification code box + confirm button | Email verification |
| `send_password_reset` | Reset link with expiry notice | Password reset request |
| `send_support_confirmation` | Ticket summary with ID, subject, message preview | Support inquiry submitted |
| `send_support_response` | Quoted response with responder attribution | Support agent reply |
| `send_security_alert` | Severity-badged alert with threat details table | Shield threat detection |
| `send_daily_report` | Dashboard-style metrics: requests, blocks, threats, IPs, audit chain | Daily cron |

## Architecture

```
EmailService
   ├── EmailConfig (from env vars)
   ├── ResendClient (reqwest-based HTTP client)
   └── Templates
        ├── layout.rs     -- shared HTML wrapper, buttons, code boxes, metric rows
        ├── welcome.rs
        ├── verification.rs
        ├── password_reset.rs
        ├── support.rs     -- confirmation + response templates
        ├── security_alert.rs
        └── daily_report.rs
```

All templates use the shared `layout::wrap()` function which provides:
- Responsive table-based structure (max 580px container)
- Gradient header with Prometheus branding
- Hidden preheader text for email preview
- Consistent footer with company attribution
- MSO conditional comments for Outlook compatibility

## Security Alert Severity Levels

| Level | Color | Badge BG | Includes Alert Banner |
|-------|-------|----------|-----------------------|
| Critical | `#DC2626` | `#FEF2F2` | Yes |
| High | `#EA580C` | `#FFF7ED` | Yes |
| Medium | `#D97706` | `#FFFBEB` | No |
| Low | `#2563EB` | `#EFF6FF` | No |

## Daily Report Sections

The daily security report email includes:
- **Overview** -- total requests, blocked count with percentage, unique IPs, active bans, uptime
- **Threat Breakdown** -- categorized threat counts with color coding
- **Top Blocked IPs** -- IP addresses ranked by block count
- **Platform Status** -- active training runs, deployed models, audit chain integrity

## Configuration

All configuration is via environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RESEND_API_KEY` | Yes | -- | Resend API key (`re_xxxxx`) |
| `EMAIL_FROM` | No | `Prometheus <noreply@automatanexus.com>` | Sender address |
| `EMAIL_REPLY_TO` | No | `support@automatanexus.com` | Reply-to address |
| `PROMETHEUS_BASE_URL` | No | `http://localhost:3030` | Base URL for email links |
| `SUPPORT_EMAIL` | No | `support@automatanexus.com` | Footer support contact |
| `SECURITY_EMAIL_RECIPIENTS` | No | -- | Comma-separated list for alerts and daily reports |

## Usage

```rust
use prometheus_email::EmailService;

let svc = EmailService::from_env()?;

// Account lifecycle
svc.send_welcome("user@example.com", "alice").await?;
svc.send_verification("user@example.com", "alice", "ABC123", 30).await?;
svc.send_password_reset("user@example.com", "alice", "token123", 60).await?;

// Security
svc.send_security_alert(&alert).await?;
svc.send_daily_report(&report).await?;
```

## Dependencies

- `reqwest` -- HTTP client for Resend API
- `serde` / `serde_json` -- request/response serialization
- `chrono` -- timestamp formatting in daily reports
- `tracing` -- structured logging of sent messages
- `thiserror` -- error type definitions
