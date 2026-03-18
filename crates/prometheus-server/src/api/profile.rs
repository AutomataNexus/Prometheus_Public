// ============================================================================
// File: profile.rs
// Description: User profile and preferences management with Aegis-DB persistence
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

//! User profile and preferences management.
//!
//! Profile data and preferences are stored in Aegis-DB `user_preferences` collection.
//! Accessible from the profile/preferences tab in the UI header dropdown.

use axum::{extract::State, Extension, Json};
use serde::{Deserialize, Serialize};
use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// User profile response — combines auth info with preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: String,
    pub preferences: UserPreferences,
    pub subscription_tier: String,
    pub mfa_enabled: bool,
    pub token_balance: u64,
    pub tokens_used: u64,
}

/// User-configurable preferences stored in Aegis-DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub id: String,
    pub user_id: String,
    pub theme: String,
    pub notifications_enabled: bool,
    pub email_notifications: bool,
    pub training_auto_stop: bool,
    pub default_architecture: Option<String>,
    pub mfa_enabled: bool,
    pub timezone: String,
    pub created_at: String,
    pub updated_at: String,
}

impl UserPreferences {
    fn default_for(user_id: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: user_id.to_string(),
            user_id: user_id.to_string(),
            theme: "nexusedge-dark".into(),
            notifications_enabled: true,
            email_notifications: true,
            training_auto_stop: false,
            default_architecture: None,
            mfa_enabled: false,
            timezone: "UTC".into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Partial update request for preferences.
#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub theme: Option<String>,
    pub notifications_enabled: Option<bool>,
    pub email_notifications: Option<bool>,
    pub training_auto_stop: Option<bool>,
    pub default_architecture: Option<String>,
    pub timezone: Option<String>,
}

/// Get the current user's full profile.
pub async fn get_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<UserProfile>> {

    let prefs = load_or_create_preferences(&state, &auth.user_id).await?;

    // Load subscription info
    let (tier, balance, used) = match state.aegis_get_doc("subscriptions", &auth.user_id).await {
        Ok(doc) => {
            let tier = doc.get("tier").and_then(|v| v.as_str()).unwrap_or("free").to_string();
            let balance = doc.get("token_balance").and_then(|v| v.as_u64()).unwrap_or(1000);
            let used = doc.get("tokens_used_this_period").and_then(|v| v.as_u64()).unwrap_or(0);
            (tier, balance, used)
        }
        Err(_) => ("free".to_string(), 1000, 0),
    };

    Ok(Json(UserProfile {
        user_id: auth.user_id.clone(),
        username: auth.username.clone(),
        email: None, // Would come from Aegis-DB auth
        role: auth.role.clone(),
        preferences: prefs,
        subscription_tier: tier,
        mfa_enabled: crate::api::mfa::check_mfa_required(&state, &auth.user_id).await,
        token_balance: balance,
        tokens_used: used,
    }))
}

/// Update user preferences.
pub async fn update_preferences(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<UpdatePreferencesRequest>,
) -> AppResult<Json<UserPreferences>> {

    let mut prefs = load_or_create_preferences(&state, &auth.user_id).await?;

    // Apply partial updates
    if let Some(theme) = req.theme {
        prefs.theme = theme;
    }
    if let Some(notifications) = req.notifications_enabled {
        prefs.notifications_enabled = notifications;
    }
    if let Some(email_notifs) = req.email_notifications {
        prefs.email_notifications = email_notifs;
    }
    if let Some(auto_stop) = req.training_auto_stop {
        prefs.training_auto_stop = auto_stop;
    }
    if let Some(arch) = req.default_architecture {
        prefs.default_architecture = Some(arch);
    }
    if let Some(tz) = req.timezone {
        prefs.timezone = tz;
    }

    prefs.updated_at = chrono::Utc::now().to_rfc3339();

    let doc = serde_json::to_value(&prefs)
        .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;

    let _ = state.aegis_delete_doc("user_preferences", &auth.user_id).await;
    state.aegis_create_doc("user_preferences", doc).await?;

    Ok(Json(prefs))
}

/// Get just the preferences (lightweight endpoint).
pub async fn get_preferences(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<UserPreferences>> {

    let prefs = load_or_create_preferences(&state, &auth.user_id).await?;
    Ok(Json(prefs))
}

/// Get the user's token balance.
pub async fn get_token_balance(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<serde_json::Value>> {

    let (tier, balance, used, limit) = match state.aegis_get_doc("subscriptions", &auth.user_id).await {
        Ok(doc) => {
            let tier_str = doc.get("tier").and_then(|v| v.as_str()).unwrap_or("free");
            let tier: crate::api::billing::SubscriptionTier = serde_json::from_value(
                serde_json::json!(tier_str)
            ).unwrap_or_default();
            let balance = doc.get("token_balance").and_then(|v| v.as_u64()).unwrap_or(1000);
            let used = doc.get("tokens_used_this_period").and_then(|v| v.as_u64()).unwrap_or(0);
            (tier_str.to_string(), balance, used, tier.monthly_token_limit())
        }
        Err(_) => ("free".to_string(), 1000u64, 0u64, 1000u64),
    };

    Ok(Json(serde_json::json!({
        "tier": tier,
        "token_balance": balance,
        "tokens_used": used,
        "tokens_limit": limit,
        "unlimited": limit == u64::MAX,
    })))
}

// ── Internal helpers ──────────────────────────────────────

async fn load_or_create_preferences(state: &AppState, user_id: &str) -> Result<UserPreferences, AppError> {
    match state.aegis_get_doc("user_preferences", user_id).await {
        Ok(doc) => serde_json::from_value(doc)
            .map_err(|e| AppError::Internal(format!("Preferences parse error: {e}"))),
        Err(_) => {
            let prefs = UserPreferences::default_for(user_id);
            let doc = serde_json::to_value(&prefs)
                .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;
            state.aegis_create_doc("user_preferences", doc).await?;
            Ok(prefs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preferences_values() {
        let prefs = UserPreferences::default_for("user-1");
        assert_eq!(prefs.user_id, "user-1");
        assert_eq!(prefs.theme, "nexusedge-dark");
        assert!(prefs.notifications_enabled);
        assert!(prefs.email_notifications);
        assert!(!prefs.training_auto_stop);
        assert!(prefs.default_architecture.is_none());
        assert!(!prefs.mfa_enabled);
        assert_eq!(prefs.timezone, "UTC");
    }

    #[test]
    fn preferences_serialize_roundtrip() {
        let prefs = UserPreferences {
            id: "user-1".into(),
            user_id: "user-1".into(),
            theme: "nexusedge-light".into(),
            notifications_enabled: false,
            email_notifications: true,
            training_auto_stop: true,
            default_architecture: Some("bert".into()),
            mfa_enabled: true,
            timezone: "America/New_York".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-15T00:00:00Z".into(),
        };
        let json = serde_json::to_value(&prefs).unwrap();
        let parsed: UserPreferences = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.theme, "nexusedge-light");
        assert!(!parsed.notifications_enabled);
        assert!(parsed.mfa_enabled);
        assert_eq!(parsed.default_architecture, Some("bert".into()));
    }

    #[test]
    fn user_profile_serialize() {
        let profile = UserProfile {
            user_id: "u-1".into(),
            username: "alice".into(),
            email: Some("alice@test.com".into()),
            role: "admin".into(),
            preferences: UserPreferences::default_for("u-1"),
            subscription_tier: "pro".into(),
            mfa_enabled: false,
            token_balance: 45000,
            tokens_used: 5000,
        };
        let json = serde_json::to_value(&profile).unwrap();
        assert_eq!(json["username"], "alice");
        assert_eq!(json["subscription_tier"], "pro");
        assert_eq!(json["token_balance"], 45000);
    }

    #[test]
    fn update_request_partial() {
        let json = r#"{"theme":"light"}"#;
        let req: UpdatePreferencesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.theme, Some("light".into()));
        assert!(req.notifications_enabled.is_none());
        assert!(req.timezone.is_none());
    }

    #[test]
    fn update_request_full() {
        let json = r#"{
            "theme": "dark",
            "notifications_enabled": false,
            "email_notifications": false,
            "training_auto_stop": true,
            "default_architecture": "gpt2",
            "timezone": "Europe/London"
        }"#;
        let req: UpdatePreferencesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.theme, Some("dark".into()));
        assert_eq!(req.notifications_enabled, Some(false));
        assert_eq!(req.email_notifications, Some(false));
        assert_eq!(req.training_auto_stop, Some(true));
        assert_eq!(req.default_architecture, Some("gpt2".into()));
        assert_eq!(req.timezone, Some("Europe/London".into()));
    }

    #[test]
    fn preferences_id_matches_user_id() {
        let prefs = UserPreferences::default_for("user-42");
        assert_eq!(prefs.id, "user-42");
        assert_eq!(prefs.user_id, "user-42");
    }
}
