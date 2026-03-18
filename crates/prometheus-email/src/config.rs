// ============================================================================
// File: config.rs
// Description: Email service configuration with Resend API credentials and branding
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
/// Email service configuration.
#[derive(Debug, Clone)]
pub struct EmailConfig {
    /// Resend API key (re_xxxxx).
    pub resend_api_key: String,
    /// From address with display name.
    pub from: String,
    /// Reply-to address for support emails.
    pub reply_to: String,
    /// Base URL for links in emails (e.g., "https://prometheus.automatanexus.com").
    pub base_url: String,
    /// Company name for branding.
    pub company_name: String,
    /// Support email displayed in footer.
    pub support_email: String,
    /// Security team recipients for alerts and daily reports.
    pub security_recipients: Vec<String>,
}

impl EmailConfig {
    pub fn from_env() -> Result<Self, crate::error::EmailError> {
        let api_key = std::env::var("RESEND_API_KEY")
            .map_err(|_| crate::error::EmailError::Config("RESEND_API_KEY not set".into()))?;

        Ok(Self {
            resend_api_key: api_key,
            from: std::env::var("EMAIL_FROM")
                .unwrap_or_else(|_| "Prometheus <noreply@automatanexus.com>".into()),
            reply_to: std::env::var("EMAIL_REPLY_TO")
                .unwrap_or_else(|_| "support@automatanexus.com".into()),
            base_url: std::env::var("PROMETHEUS_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3030".into()),
            company_name: "Automata Controls".into(),
            support_email: std::env::var("SUPPORT_EMAIL")
                .unwrap_or_else(|_| "support@automatanexus.com".into()),
            security_recipients: std::env::var("SECURITY_EMAIL_RECIPIENTS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize all tests that touch env vars to prevent parallel race conditions.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Helper: run a closure with env vars set, restoring them afterward.
    fn with_env<F, R>(vars: &[(&str, Option<&str>)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Set/remove vars
        for &(key, val) in vars {
            match val {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        let result = f();
        // Clean up all vars
        for &(key, _) in vars {
            std::env::remove_var(key);
        }
        result
    }

    #[test]
    fn from_env_fails_without_api_key() {
        with_env(&[("RESEND_API_KEY", None)], || {
            let result = EmailConfig::from_env();
            assert!(result.is_err());
        });
    }

    #[test]
    fn from_env_succeeds_with_api_key() {
        with_env(&[
            ("RESEND_API_KEY", Some("re_test_123")),
            ("EMAIL_FROM", None),
            ("EMAIL_REPLY_TO", None),
            ("PROMETHEUS_BASE_URL", None),
            ("SUPPORT_EMAIL", None),
            ("SECURITY_EMAIL_RECIPIENTS", None),
        ], || {
            let config = EmailConfig::from_env().unwrap();
            assert_eq!(config.resend_api_key, "re_test_123");
        });
    }

    #[test]
    fn from_env_default_from_address() {
        with_env(&[("RESEND_API_KEY", Some("re_test_456")), ("EMAIL_FROM", None)], || {
            let config = EmailConfig::from_env().unwrap();
            assert_eq!(config.from, "Prometheus <noreply@automatanexus.com>");
        });
    }

    #[test]
    fn from_env_custom_from_address() {
        with_env(&[
            ("RESEND_API_KEY", Some("re_test_789")),
            ("EMAIL_FROM", Some("Custom <custom@example.com>")),
        ], || {
            let config = EmailConfig::from_env().unwrap();
            assert_eq!(config.from, "Custom <custom@example.com>");
        });
    }

    #[test]
    fn from_env_default_reply_to() {
        with_env(&[("RESEND_API_KEY", Some("re_test_reply")), ("EMAIL_REPLY_TO", None)], || {
            let config = EmailConfig::from_env().unwrap();
            assert_eq!(config.reply_to, "support@automatanexus.com");
        });
    }

    #[test]
    fn from_env_default_base_url() {
        with_env(&[("RESEND_API_KEY", Some("re_test_base")), ("PROMETHEUS_BASE_URL", None)], || {
            let config = EmailConfig::from_env().unwrap();
            assert_eq!(config.base_url, "http://localhost:3030");
        });
    }

    #[test]
    fn from_env_company_name_is_automata_controls() {
        with_env(&[("RESEND_API_KEY", Some("re_test_company"))], || {
            let config = EmailConfig::from_env().unwrap();
            assert_eq!(config.company_name, "Automata Controls");
        });
    }

    #[test]
    fn from_env_security_recipients_parsed() {
        with_env(&[
            ("RESEND_API_KEY", Some("re_test_sec")),
            ("SECURITY_EMAIL_RECIPIENTS", Some("alice@test.com, bob@test.com, carol@test.com")),
        ], || {
            let config = EmailConfig::from_env().unwrap();
            assert_eq!(config.security_recipients.len(), 3);
            assert_eq!(config.security_recipients[0], "alice@test.com");
            assert_eq!(config.security_recipients[1], "bob@test.com");
            assert_eq!(config.security_recipients[2], "carol@test.com");
        });
    }

    #[test]
    fn from_env_empty_security_recipients() {
        with_env(&[
            ("RESEND_API_KEY", Some("re_test_empty_sec")),
            ("SECURITY_EMAIL_RECIPIENTS", None),
        ], || {
            let config = EmailConfig::from_env().unwrap();
            assert!(config.security_recipients.is_empty());
        });
    }

    #[test]
    fn from_env_security_recipients_filters_empty() {
        with_env(&[
            ("RESEND_API_KEY", Some("re_test_filter")),
            ("SECURITY_EMAIL_RECIPIENTS", Some("a@b.com,,, ,c@d.com")),
        ], || {
            let config = EmailConfig::from_env().unwrap();
            assert_eq!(config.security_recipients.len(), 2);
        });
    }

    #[test]
    fn config_is_clone() {
        let config = EmailConfig {
            resend_api_key: "re_test_clone".into(),
            from: "test@test.com".into(),
            reply_to: "reply@test.com".into(),
            base_url: "http://localhost".into(),
            company_name: "Test".into(),
            support_email: "support@test.com".into(),
            security_recipients: vec!["a@b.com".into()],
        };
        let config2 = config.clone();
        assert_eq!(config2.resend_api_key, config.resend_api_key);
        assert_eq!(config2.from, config.from);
    }
}
