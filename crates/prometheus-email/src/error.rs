// ============================================================================
// File: error.rs
// Description: Error types for the email service covering API, template, and config errors
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error("Resend API error: {0}")]
    ResendApi(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resend_api_error_display() {
        let err = EmailError::ResendApi("401 Unauthorized".to_string());
        assert_eq!(err.to_string(), "Resend API error: 401 Unauthorized");
    }

    #[test]
    fn template_error_display() {
        let err = EmailError::Template("missing variable".to_string());
        assert_eq!(err.to_string(), "Template error: missing variable");
    }

    #[test]
    fn config_error_display() {
        let err = EmailError::Config("RESEND_API_KEY not set".to_string());
        assert_eq!(err.to_string(), "Configuration error: RESEND_API_KEY not set");
    }

    #[test]
    fn error_is_debug() {
        let err = EmailError::ResendApi("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("ResendApi"));
    }
}
