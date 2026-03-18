// ============================================================================
// File: middleware.rs
// Description: Bearer token authentication middleware validating sessions against Aegis-DB
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};
use crate::state::AppState;

/// Auth middleware — validates bearer token against Aegis-DB session endpoint.
///
/// Extracts the token from the Authorization header and calls
/// GET /api/v1/auth/session on Aegis-DB. If valid, the user info
/// is inserted into request extensions for downstream handlers.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth_header[7..].to_string();

    // Check if this is an ingestion key (prom_xxx...)
    if token.starts_with("prom_") {
        if let Some((user_id, username, role)) = crate::api::ingestion_keys::validate_ingestion_key(&state, &token).await {
            request.extensions_mut().insert(AuthUser {
                token: token.clone(),
                user_id,
                username,
                role,
            });
            return Ok(next.run(request).await);
        }
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Validate token against Aegis-DB via /api/v1/auth/me (reads Bearer header)
    let resp = state
        .http_client
        .get(format!("{}/api/v1/auth/me", state.config.aegis_db_url))
        .header("Authorization", format!("Bearer {}", &token))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !resp.status().is_success() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let user: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Aegis-DB returns null for invalid sessions
    if user.is_null() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Extract user info and insert into extensions
    let role = user
        .get("role")
        .and_then(|r| r.as_str())
        .unwrap_or("viewer")
        .to_string();
    request.extensions_mut().insert(AuthUser {
        token: token.clone(),
        user_id: user.get("id").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
        username: user.get("username").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
        role,
    });

    Ok(next.run(request).await)
}

/// Authenticated user info extracted by the middleware.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub token: String,
    #[allow(dead_code)] // populated from Aegis-DB auth; available for handler use
    pub user_id: String,
    #[allow(dead_code)] // populated from Aegis-DB auth; available for handler use
    pub username: String,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    #[allow(dead_code)] // authorization helper for future handler use
    pub fn can_write(&self) -> bool {
        self.role == "admin" || self.role == "operator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auth_user(role: &str) -> AuthUser {
        AuthUser {
            token: "test-token-abc".to_string(),
            user_id: "u-42".to_string(),
            username: "testuser".to_string(),
            role: role.to_string(),
        }
    }

    #[test]
    fn is_admin_with_admin_role() {
        let user = make_auth_user("admin");
        assert!(user.is_admin());
    }

    #[test]
    fn is_admin_with_operator_role() {
        let user = make_auth_user("operator");
        assert!(!user.is_admin());
    }

    #[test]
    fn is_admin_with_viewer_role() {
        let user = make_auth_user("viewer");
        assert!(!user.is_admin());
    }

    #[test]
    fn can_write_with_admin_role() {
        let user = make_auth_user("admin");
        assert!(user.can_write());
    }

    #[test]
    fn can_write_with_operator_role() {
        let user = make_auth_user("operator");
        assert!(user.can_write());
    }

    #[test]
    fn cannot_write_with_viewer_role() {
        let user = make_auth_user("viewer");
        assert!(!user.can_write());
    }

    #[test]
    fn cannot_write_with_unknown_role() {
        let user = make_auth_user("guest");
        assert!(!user.can_write());
    }

    #[test]
    fn is_admin_case_sensitive() {
        let user = make_auth_user("Admin");
        assert!(!user.is_admin());
    }

    #[test]
    fn auth_user_clone() {
        let user = make_auth_user("admin");
        let user2 = user.clone();
        assert_eq!(user2.token, "test-token-abc");
        assert_eq!(user2.user_id, "u-42");
        assert_eq!(user2.username, "testuser");
        assert_eq!(user2.role, "admin");
    }

    #[test]
    fn auth_user_debug_format() {
        let user = make_auth_user("viewer");
        let debug = format!("{:?}", user);
        assert!(debug.contains("AuthUser"));
        assert!(debug.contains("viewer"));
    }
}
