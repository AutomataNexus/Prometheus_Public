// ============================================================================
// File: config.rs
// Description: Server configuration struct deserialized from environment variables and config files
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub aegis_db_url: String,
    pub gradient_api_key: Option<String>,
    pub gradient_agent_id: Option<String>,
    /// Full URL to the DO GenAI chat completions endpoint
    /// e.g. https://agent-XXXXX.ondigitalocean.app/api/v1/chat/completions
    pub gradient_endpoint: Option<String>,
    pub data_dir: String,
    /// Stripe secret key (sk_live_... or sk_test_...) — from vault/env only
    pub stripe_secret_key: Option<String>,
    /// Stripe webhook signing secret (whsec_...) — from vault/env only
    pub stripe_webhook_secret: Option<String>,
    /// Stripe price IDs for subscription tiers — from vault/env only
    pub stripe_price_basic: Option<String>,
    pub stripe_price_pro: Option<String>,
    pub stripe_price_enterprise: Option<String>,
    /// Stripe meter ID for usage-based token billing
    #[allow(dead_code)]
    pub stripe_meter_id: Option<String>,
    /// Stripe price ID for token overage (metered)
    pub stripe_price_overage: Option<String>,
    /// Server-wide cap on concurrent training runs (all users combined).
    /// Default: number of CPU cores (minimum 2).
    pub max_concurrent_trainings: u32,
}

impl ServerConfig {
    /// Return the public-facing URL for this server, if configured.
    /// Falls back to `PROMETHEUS_PUBLIC_URL` environment variable.
    pub fn public_url(&self) -> Option<String> {
        std::env::var("PROMETHEUS_PUBLIC_URL").ok()
    }

    /// Whether Stripe billing is configured and available.
    pub fn stripe_enabled(&self) -> bool {
        self.stripe_secret_key.is_some()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("PROMETHEUS_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PROMETHEUS_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3030),
            aegis_db_url: std::env::var("AEGIS_DB_URL")
                .unwrap_or_else(|_| "http://localhost:9091".into()),
            gradient_api_key: std::env::var("DO_GENAI_ACCESS_KEY")
                .or_else(|_| std::env::var("GRADIENT_MODEL_ACCESS_KEY"))
                .ok(),
            gradient_agent_id: std::env::var("GRADIENT_AGENT_ID").ok(),
            gradient_endpoint: std::env::var("DO_GENAI_ENDPOINT").ok(),
            data_dir: std::env::var("PROMETHEUS_DATA_DIR")
                .unwrap_or_else(|_| "/tmp/prometheus-data".into()),
            // Stripe — all from vault/env, never hardcoded
            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").ok(),
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
            stripe_price_basic: std::env::var("STRIPE_PRICE_BASIC").ok(),
            stripe_price_pro: std::env::var("STRIPE_PRICE_PRO").ok(),
            stripe_price_enterprise: std::env::var("STRIPE_PRICE_ENTERPRISE").ok(),
            stripe_meter_id: std::env::var("STRIPE_METER_ID").ok(),
            stripe_price_overage: std::env::var("STRIPE_PRICE_OVERAGE").ok(),
            max_concurrent_trainings: std::env::var("PROMETHEUS_MAX_TRAININGS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(4).max(2)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F, R>(vars: &[(&str, Option<&str>)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for &(key, val) in vars {
            match val {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        let result = f();
        for &(key, _) in vars {
            std::env::remove_var(key);
        }
        result
    }

    #[test]
    fn default_host_without_env() {
        with_env(&[("PROMETHEUS_HOST", None)], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.host, "0.0.0.0");
        });
    }

    #[test]
    fn default_port_without_env() {
        with_env(&[("PROMETHEUS_PORT", None)], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.port, 3030);
        });
    }

    #[test]
    fn default_aegis_db_url_without_env() {
        with_env(&[("AEGIS_DB_URL", None)], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.aegis_db_url, "http://localhost:9091");
        });
    }

    #[test]
    fn default_data_dir_without_env() {
        with_env(&[("PROMETHEUS_DATA_DIR", None)], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.data_dir, "/tmp/prometheus-data");
        });
    }

    #[test]
    fn default_gradient_api_key_is_none_without_env() {
        with_env(&[
            ("GRADIENT_MODEL_ACCESS_KEY", None),
            ("DO_GENAI_ACCESS_KEY", None),
        ], || {
            let cfg = ServerConfig::default();
            assert!(cfg.gradient_api_key.is_none());
        });
    }

    #[test]
    fn default_gradient_agent_id_is_none_without_env() {
        with_env(&[("GRADIENT_AGENT_ID", None)], || {
            let cfg = ServerConfig::default();
            assert!(cfg.gradient_agent_id.is_none());
        });
    }

    #[test]
    fn port_from_env_var() {
        with_env(&[("PROMETHEUS_PORT", Some("8080"))], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.port, 8080);
        });
    }

    #[test]
    fn invalid_port_env_falls_back_to_default() {
        with_env(&[("PROMETHEUS_PORT", Some("not_a_number"))], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.port, 3030);
        });
    }

    #[test]
    fn host_from_env_var() {
        with_env(&[("PROMETHEUS_HOST", Some("127.0.0.1"))], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.host, "127.0.0.1");
        });
    }

    #[test]
    fn config_is_clone() {
        with_env(&[("PROMETHEUS_PORT", None)], || {
            let cfg = ServerConfig::default();
            let cfg2 = cfg.clone();
            assert_eq!(cfg2.port, 3030);
        });
    }

    #[test]
    fn public_url_from_env() {
        with_env(&[("PROMETHEUS_PUBLIC_URL", Some("https://prometheus.example.com"))], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.public_url(), Some("https://prometheus.example.com".to_string()));
        });
    }

    #[test]
    fn public_url_none_when_not_set() {
        with_env(&[("PROMETHEUS_PUBLIC_URL", None)], || {
            let cfg = ServerConfig::default();
            assert!(cfg.public_url().is_none());
        });
    }

    #[test]
    fn gradient_api_key_from_do_genai() {
        with_env(&[
            ("GRADIENT_MODEL_ACCESS_KEY", None),
            ("DO_GENAI_ACCESS_KEY", Some("do-key-123")),
        ], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.gradient_api_key, Some("do-key-123".to_string()));
        });
    }

    #[test]
    fn gradient_api_key_do_genai_takes_precedence() {
        with_env(&[
            ("DO_GENAI_ACCESS_KEY", Some("do-key")),
            ("GRADIENT_MODEL_ACCESS_KEY", Some("gradient-key")),
        ], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.gradient_api_key, Some("do-key".to_string()));
        });
    }

    #[test]
    fn gradient_api_key_falls_back_to_gradient_model() {
        with_env(&[
            ("DO_GENAI_ACCESS_KEY", None),
            ("GRADIENT_MODEL_ACCESS_KEY", Some("gradient-key")),
        ], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.gradient_api_key, Some("gradient-key".to_string()));
        });
    }

    #[test]
    fn gradient_endpoint_from_env() {
        with_env(&[("DO_GENAI_ENDPOINT", Some("https://agent-123.ondigitalocean.app/api/v1/chat/completions"))], || {
            let cfg = ServerConfig::default();
            assert_eq!(
                cfg.gradient_endpoint,
                Some("https://agent-123.ondigitalocean.app/api/v1/chat/completions".to_string())
            );
        });
    }

    #[test]
    fn gradient_endpoint_none_when_not_set() {
        with_env(&[("DO_GENAI_ENDPOINT", None)], || {
            let cfg = ServerConfig::default();
            assert!(cfg.gradient_endpoint.is_none());
        });
    }

    #[test]
    fn data_dir_from_env() {
        with_env(&[("PROMETHEUS_DATA_DIR", Some("/custom/data"))], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.data_dir, "/custom/data");
        });
    }

    #[test]
    fn aegis_db_url_from_env() {
        with_env(&[("AEGIS_DB_URL", Some("http://aegis:9999"))], || {
            let cfg = ServerConfig::default();
            assert_eq!(cfg.aegis_db_url, "http://aegis:9999");
        });
    }

    #[test]
    fn all_default_fields_are_populated() {
        with_env(&[
            ("PROMETHEUS_HOST", None),
            ("PROMETHEUS_PORT", None),
            ("AEGIS_DB_URL", None),
            ("PROMETHEUS_DATA_DIR", None),
            ("DO_GENAI_ACCESS_KEY", None),
            ("GRADIENT_MODEL_ACCESS_KEY", None),
        ], || {
            let cfg = ServerConfig::default();
            assert!(!cfg.host.is_empty());
            assert!(cfg.port > 0);
            assert!(!cfg.aegis_db_url.is_empty());
            assert!(!cfg.data_dir.is_empty());
        });
    }

    #[test]
    fn stripe_enabled_when_key_set() {
        with_env(&[("STRIPE_SECRET_KEY", Some("sk_test_123"))], || {
            let cfg = ServerConfig::default();
            assert!(cfg.stripe_enabled());
        });
    }

    #[test]
    fn stripe_disabled_without_key() {
        with_env(&[("STRIPE_SECRET_KEY", None)], || {
            let cfg = ServerConfig::default();
            assert!(!cfg.stripe_enabled());
        });
    }
}
