// ============================================================================
// File: config.rs
// Description: CLI configuration management stored at ~/.prometheus/config.toml
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! CLI configuration stored at ~/.prometheus/config.toml

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_url: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

fn default_data_dir() -> String {
    config_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/tmp/prometheus-cli".into())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:3030".into(),
            data_dir: default_data_dir(),
        }
    }
}

fn config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".prometheus"))
}

impl Config {
    pub fn config_path() -> Option<PathBuf> {
        config_dir().map(|d| d.join("config.toml"))
    }

    pub fn credentials_path() -> Option<PathBuf> {
        config_dir().map(|d| d.join("credentials"))
    }

    pub fn load() -> Result<Self> {
        let Some(path) = Self::config_path() else {
            return Ok(Self::default());
        };
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::config_path() else {
            anyhow::bail!("Cannot determine config directory");
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn load_token(&self) -> Option<String> {
        Self::credentials_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn save_token(token: &str) -> Result<()> {
        let Some(path) = Self::credentials_path() else {
            anyhow::bail!("Cannot determine credentials path");
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, token)?;
        // Set restrictive permissions on credentials file
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    pub fn clear_token() -> Result<()> {
        if let Some(path) = Self::credentials_path() {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config_values() {
        let cfg = Config::default();
        assert_eq!(cfg.server_url, "http://localhost:3030");
        // data_dir should be set to something (either ~/.prometheus or /tmp fallback)
        assert!(!cfg.data_dir.is_empty());
    }

    #[test]
    fn test_config_path_is_under_home() {
        // config_path should return Some on systems with a home dir
        if let Some(path) = Config::config_path() {
            assert!(path.to_string_lossy().contains(".prometheus"));
            assert!(path.to_string_lossy().ends_with("config.toml"));
        }
    }

    #[test]
    fn test_credentials_path_is_under_home() {
        if let Some(path) = Config::credentials_path() {
            assert!(path.to_string_lossy().contains(".prometheus"));
            assert!(path.to_string_lossy().ends_with("credentials"));
        }
    }

    #[test]
    fn test_config_serialize_deserialize_roundtrip() {
        let cfg = Config {
            server_url: "https://example.com:8080".into(),
            data_dir: "/tmp/test-data".into(),
        };
        let serialized = toml::to_string_pretty(&cfg).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.server_url, "https://example.com:8080");
        assert_eq!(deserialized.data_dir, "/tmp/test-data");
    }

    #[test]
    fn test_config_deserialize_with_defaults() {
        // If data_dir is missing, it should use the default
        let toml_str = r#"server_url = "http://myhost:3000""#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.server_url, "http://myhost:3000");
        assert!(!cfg.data_dir.is_empty());
    }

    #[test]
    fn test_config_save_and_load() {
        // Use a temporary directory to avoid touching real config
        let tmp = std::env::temp_dir().join("prometheus-cli-test-config");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config_file = tmp.join("config.toml");
        let cfg = Config {
            server_url: "http://test-server:9999".into(),
            data_dir: "/tmp/test".into(),
        };
        let content = toml::to_string_pretty(&cfg).unwrap();
        fs::write(&config_file, &content).unwrap();

        let loaded_content = fs::read_to_string(&config_file).unwrap();
        let loaded: Config = toml::from_str(&loaded_content).unwrap();
        assert_eq!(loaded.server_url, "http://test-server:9999");
        assert_eq!(loaded.data_dir, "/tmp/test");

        let _ = fs::remove_dir_all(&tmp);
    }

    // Token tests use isolated temp files to avoid parallel test conflicts
    fn token_roundtrip_in_tempdir(token_content: &str) -> Option<String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static CTR: AtomicU64 = AtomicU64::new(0);
        let id = CTR.fetch_add(1, Ordering::Relaxed);
        let tmp = std::env::temp_dir().join(format!("prom-tok-{}-{}", std::process::id(), id));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let cred_path = tmp.join("credentials");
        fs::write(&cred_path, token_content).unwrap();
        let loaded = fs::read_to_string(&cred_path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let _ = fs::remove_dir_all(&tmp);
        loaded
    }

    #[test]
    fn test_token_save_load_cycle() {
        let token = "test-token-cycle";
        let loaded = token_roundtrip_in_tempdir(token);
        assert_eq!(loaded, Some("test-token-cycle".into()));
    }

    #[test]
    #[cfg(unix)]
    fn test_credentials_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = std::env::temp_dir().join(format!("prom-perm-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let cred_path = tmp.join("credentials");
        fs::write(&cred_path, "perm-test").unwrap();
        fs::set_permissions(&cred_path, fs::Permissions::from_mode(0o600)).unwrap();
        let mode = fs::metadata(&cred_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "Credentials file should have mode 0600, got {:o}", mode);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_token_trims_whitespace() {
        let loaded = token_roundtrip_in_tempdir("  my-trimmed-token  \n");
        assert_eq!(loaded, Some("my-trimmed-token".into()));
    }

    #[test]
    fn test_load_token_returns_none_for_empty() {
        let loaded = token_roundtrip_in_tempdir("   \n");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_clear_token_no_error_when_missing() {
        // clear_token should not error if file doesn't exist
        Config::clear_token().unwrap();
    }
}
