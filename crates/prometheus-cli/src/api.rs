// ============================================================================
// File: api.rs
// Description: HTTP client wrapper for authenticated Prometheus server API calls
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! HTTP client for the Prometheus server API.

use anyhow::{Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        }
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {t}"))
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header(AUTHORIZATION, auth);
        }
        let resp = req.send().await.context("Failed to connect to server")?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Server returned {status}: {text}");
        }
        resp.json().await.context("Failed to parse response")
    }

    pub async fn post(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url)
            .header(CONTENT_TYPE, "application/json")
            .json(&body);
        if let Some(auth) = self.auth_header() {
            req = req.header(AUTHORIZATION, auth);
        }
        let resp = req.send().await.context("Failed to connect to server")?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Server returned {status}: {text}");
        }
        resp.json().await.context("Failed to parse response")
    }

    pub async fn post_multipart(&self, path: &str, form: reqwest::multipart::Form) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).multipart(form);
        if let Some(auth) = self.auth_header() {
            req = req.header(AUTHORIZATION, auth);
        }
        let resp = req.send().await.context("Failed to connect to server")?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Server returned {status}: {text}");
        }
        resp.json().await.context("Failed to parse response")
    }

    pub async fn delete(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.delete(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header(AUTHORIZATION, auth);
        }
        let resp = req.send().await.context("Failed to connect to server")?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Server returned {status}: {text}");
        }
        resp.json().await.context("Failed to parse response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stores_base_url() {
        let client = ApiClient::new("http://localhost:3030", None);
        assert_eq!(client.base_url, "http://localhost:3030");
    }

    #[test]
    fn test_new_trims_trailing_slash() {
        let client = ApiClient::new("http://localhost:3030/", None);
        assert_eq!(client.base_url, "http://localhost:3030");
    }

    #[test]
    fn test_new_trims_multiple_trailing_slashes() {
        let client = ApiClient::new("http://example.com///", None);
        assert_eq!(client.base_url, "http://example.com");
    }

    #[test]
    fn test_new_no_trailing_slash_unchanged() {
        let client = ApiClient::new("https://api.example.com", None);
        assert_eq!(client.base_url, "https://api.example.com");
    }

    #[test]
    fn test_new_with_path_trims_only_trailing() {
        let client = ApiClient::new("http://host:8080/api/v1/", None);
        assert_eq!(client.base_url, "http://host:8080/api/v1");
    }

    #[test]
    fn test_token_accessor_none() {
        let client = ApiClient::new("http://localhost:3030", None);
        assert!(client.token().is_none());
    }

    #[test]
    fn test_token_accessor_some() {
        let client = ApiClient::new("http://localhost:3030", Some("my-token".into()));
        assert_eq!(client.token(), Some("my-token"));
    }

    #[test]
    fn test_auth_header_none_when_no_token() {
        let client = ApiClient::new("http://localhost:3030", None);
        assert!(client.auth_header().is_none());
    }

    #[test]
    fn test_auth_header_bearer_format() {
        let client = ApiClient::new("http://localhost:3030", Some("abc123".into()));
        let header = client.auth_header().unwrap();
        assert_eq!(header, "Bearer abc123");
    }

    #[test]
    fn test_auth_header_preserves_token_exactly() {
        let client = ApiClient::new("http://x", Some("tok-with-special/chars=".into()));
        let header = client.auth_header().unwrap();
        assert_eq!(header, "Bearer tok-with-special/chars=");
    }

    #[test]
    fn test_new_with_empty_token() {
        let client = ApiClient::new("http://localhost", Some("".into()));
        // Empty string is still Some
        assert_eq!(client.token(), Some(""));
        assert_eq!(client.auth_header(), Some("Bearer ".into()));
    }
}
