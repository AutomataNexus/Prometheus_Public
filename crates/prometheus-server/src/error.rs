// ============================================================================
// File: error.rs
// Description: Unified error types and HTTP error response mapping for the API
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Aegis-DB error: {0}")]
    AegisDb(String),

    #[error("Training error: {0}")]
    #[allow(dead_code)] // error variant for training pipeline failures
    Training(String),

    #[error("Email error: {0}")]
    Email(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limited".into()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::AegisDb(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::Training(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::Email(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::Anyhow(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = json!({
            "error": message,
            "status": status.as_u16(),
        });

        (status, axum::Json(body)).into_response()
    }
}


impl From<prometheus_shield::ShieldError> for AppError {
    fn from(e: prometheus_shield::ShieldError) -> Self {
        match e {
            prometheus_shield::ShieldError::SqlInjectionDetected(_) => AppError::Forbidden("Request blocked by security policy".into()),
            prometheus_shield::ShieldError::SsrfBlocked(_) => AppError::Forbidden("Request blocked by security policy".into()),
            prometheus_shield::ShieldError::RateLimitExceeded { .. } => AppError::RateLimited,
            prometheus_shield::ShieldError::ThreatScoreExceeded(_) => AppError::Forbidden("Request blocked by security policy".into()),
            prometheus_shield::ShieldError::MaliciousInput(msg) => AppError::BadRequest(msg),
            prometheus_shield::ShieldError::PathTraversal(_) => AppError::Forbidden("Request blocked by security policy".into()),
            prometheus_shield::ShieldError::InvalidConnectionString(msg) => AppError::BadRequest(msg),
            prometheus_shield::ShieldError::QuarantineFailed(msg) => AppError::BadRequest(msg),
            prometheus_shield::ShieldError::EmailViolation(msg) => AppError::BadRequest(msg),
            prometheus_shield::ShieldError::EmailBombing(_) => AppError::RateLimited,
        }
    }
}


impl From<prometheus_email::EmailError> for AppError {
    fn from(e: prometheus_email::EmailError) -> Self {
        AppError::Email(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn not_found_status_code() {
        let err = AppError::NotFound("missing".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn unauthorized_status_code() {
        let err = AppError::Unauthorized("bad token".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn forbidden_status_code() {
        let err = AppError::Forbidden("no access".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn bad_request_status_code() {
        let err = AppError::BadRequest("invalid input".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn rate_limited_status_code() {
        let err = AppError::RateLimited;
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn internal_status_code() {
        let err = AppError::Internal("crash".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn aegis_db_status_code() {
        let err = AppError::AegisDb("connection refused".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn training_error_status_code() {
        let err = AppError::Training("diverged".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn anyhow_error_status_code() {
        let err = AppError::Anyhow(anyhow::anyhow!("something broke"));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn display_not_found() {
        let err = AppError::NotFound("widget".into());
        assert_eq!(err.to_string(), "Not found: widget");
    }

    #[test]
    fn display_unauthorized() {
        let err = AppError::Unauthorized("expired".into());
        assert_eq!(err.to_string(), "Unauthorized: expired");
    }

    #[test]
    fn display_forbidden() {
        let err = AppError::Forbidden("denied".into());
        assert_eq!(err.to_string(), "Forbidden: denied");
    }

    #[test]
    fn display_bad_request() {
        let err = AppError::BadRequest("missing field".into());
        assert_eq!(err.to_string(), "Bad request: missing field");
    }

    #[test]
    fn display_rate_limited() {
        let err = AppError::RateLimited;
        assert_eq!(err.to_string(), "Rate limited");
    }

    #[test]
    fn display_internal() {
        let err = AppError::Internal("oops".into());
        assert_eq!(err.to_string(), "Internal error: oops");
    }

    #[test]
    fn display_aegis_db() {
        let err = AppError::AegisDb("timeout".into());
        assert_eq!(err.to_string(), "Aegis-DB error: timeout");
    }

    #[test]
    fn display_training() {
        let err = AppError::Training("nan loss".into());
        assert_eq!(err.to_string(), "Training error: nan loss");
    }

    #[test]
    fn app_result_ok() {
        let result: AppResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn app_result_err() {
        let result: AppResult<i32> = Err(AppError::NotFound("x".into()));
        assert!(result.is_err());
    }

    #[test]
    fn anyhow_conversion() {
        let anyhow_err: anyhow::Error = anyhow::anyhow!("something");
        let app_err: AppError = anyhow_err.into();
        assert!(matches!(app_err, AppError::Anyhow(_)));
        assert!(app_err.to_string().contains("something"));
    }
}
