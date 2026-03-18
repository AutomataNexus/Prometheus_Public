// ============================================================================
// File: models.rs
// Description: Authentication data types — User, Role, LoginRequest/Response, and AuthUser
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Operator,
    Viewer,
}

#[allow(dead_code)] // authorization helpers for handler use
impl Role {
    pub fn can_write(&self) -> bool {
        matches!(self, Role::Admin | Role::Operator)
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, Role::Admin)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_approved: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub valid: bool,
    pub user: Option<User>,
}

#[allow(dead_code)] // API request struct for future change-password endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn role_serialize_admin() {
        let role = Role::Admin;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, r#""admin""#);
    }

    #[test]
    fn role_serialize_operator() {
        let role = Role::Operator;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, r#""operator""#);
    }

    #[test]
    fn role_serialize_viewer() {
        let role = Role::Viewer;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, r#""viewer""#);
    }

    #[test]
    fn role_deserialize_admin() {
        let role: Role = serde_json::from_str(r#""admin""#).unwrap();
        assert_eq!(role, Role::Admin);
    }

    #[test]
    fn role_deserialize_operator() {
        let role: Role = serde_json::from_str(r#""operator""#).unwrap();
        assert_eq!(role, Role::Operator);
    }

    #[test]
    fn role_deserialize_viewer() {
        let role: Role = serde_json::from_str(r#""viewer""#).unwrap();
        assert_eq!(role, Role::Viewer);
    }

    #[test]
    fn role_can_write_admin() {
        assert!(Role::Admin.can_write());
    }

    #[test]
    fn role_can_write_operator() {
        assert!(Role::Operator.can_write());
    }

    #[test]
    fn role_cannot_write_viewer() {
        assert!(!Role::Viewer.can_write());
    }

    #[test]
    fn role_is_admin_true() {
        assert!(Role::Admin.is_admin());
    }

    #[test]
    fn role_is_admin_false_operator() {
        assert!(!Role::Operator.is_admin());
    }

    #[test]
    fn role_is_admin_false_viewer() {
        assert!(!Role::Viewer.is_admin());
    }

    #[test]
    fn login_request_roundtrip() {
        let req = LoginRequest {
            username: "alice".to_string(),
            password: "secret123".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: LoginRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.username, "alice");
        assert_eq!(parsed.password, "secret123");
    }

    #[test]
    fn session_info_valid_with_user() {
        let info = SessionInfo {
            valid: true,
            user: Some(User {
                id: "1".to_string(),
                username: "bob".to_string(),
                email: Some("bob@example.com".to_string()),
                role: Role::Operator,
            }),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: SessionInfo = serde_json::from_str(&json).unwrap();
        assert!(parsed.valid);
        assert!(parsed.user.is_some());
        let user = parsed.user.unwrap();
        assert_eq!(user.username, "bob");
        assert_eq!(user.role, Role::Operator);
    }

    #[test]
    fn session_info_invalid_no_user() {
        let info = SessionInfo {
            valid: false,
            user: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: SessionInfo = serde_json::from_str(&json).unwrap();
        assert!(!parsed.valid);
        assert!(parsed.user.is_none());
    }

    #[test]
    fn user_email_optional() {
        let user = User {
            id: "2".to_string(),
            username: "carol".to_string(),
            email: None,
            role: Role::Viewer,
        };
        let json = serde_json::to_string(&user).unwrap();
        let parsed: User = serde_json::from_str(&json).unwrap();
        assert!(parsed.email.is_none());
    }

    #[test]
    fn change_password_request_roundtrip() {
        let req = ChangePasswordRequest {
            current_password: "old".to_string(),
            new_password: "new".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: ChangePasswordRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.current_password, "old");
        assert_eq!(parsed.new_password, "new");
    }
}
