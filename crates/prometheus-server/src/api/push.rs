// ============================================================================
// File: push.rs
// Description: Expo push notification token registration and event-driven notification delivery
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

//! Push notification token registration and delivery.
//!
//! Stores Expo push tokens in Aegis-DB `push_tokens` collection.
//! Tokens are registered per-user per-device. Notifications are sent
//! via the Expo Push API for training events, security alerts, and
//! account lifecycle events.

use axum::{extract::State, Extension, Json};
use serde::{Deserialize, Serialize};
use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RegisterTokenRequest {
    pub token: String,
    pub platform: String,
    pub device_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PushToken {
    id: String,
    user_id: String,
    token: String,
    platform: String,
    device_name: String,
    registered_at: String,
}

/// Notification type — determines channel and data payload on the client.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)] // variants used as the notification system grows
pub enum NotificationType {
    TrainingComplete,
    TrainingFailed,
    TrainingEpochMilestone,
    DeploymentReady,
    SecurityAlert,
    AccountVerified,
    SubscriptionChanged,
    TrainingQueued,
    TrainingStarted,
}

impl NotificationType {
    fn channel_id(&self) -> &'static str {
        match self {
            NotificationType::TrainingComplete
            | NotificationType::TrainingFailed
            | NotificationType::TrainingEpochMilestone
            | NotificationType::TrainingQueued
            | NotificationType::TrainingStarted => "training",
            NotificationType::DeploymentReady => "training",
            NotificationType::SecurityAlert => "alerts",
            NotificationType::AccountVerified
            | NotificationType::SubscriptionChanged => "default",
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            NotificationType::TrainingComplete => "training_complete",
            NotificationType::TrainingFailed => "training_failed",
            NotificationType::TrainingEpochMilestone => "training_epoch_milestone",
            NotificationType::DeploymentReady => "deployment_ready",
            NotificationType::SecurityAlert => "security_alert",
            NotificationType::AccountVerified => "account_verified",
            NotificationType::SubscriptionChanged => "subscription_changed",
            NotificationType::TrainingQueued => "training_queued",
            NotificationType::TrainingStarted => "training_started",
        }
    }
}

/// Register an Expo push token for the authenticated user.
pub async fn register_token(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<RegisterTokenRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now().to_rfc3339();
    let token_id = format!("{}_{}", auth.user_id, sha256_short(&req.token));

    let push_token = PushToken {
        id: token_id.clone(),
        user_id: auth.user_id.clone(),
        token: req.token,
        platform: req.platform,
        device_name: req.device_name,
        registered_at: now,
    };

    let doc = serde_json::to_value(&push_token)
        .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;

    // Upsert — delete existing for this device, then create
    let _ = state.aegis_delete_doc("push_tokens", &token_id).await;
    state.aegis_create_doc("push_tokens", doc).await?;

    Ok(Json(serde_json::json!({ "registered": true })))
}

/// Send a typed push notification with optional data payload.
pub async fn notify_user_typed(
    state: &AppState,
    user_id: &str,
    title: &str,
    body: &str,
    notification_type: &NotificationType,
    data: Option<serde_json::Value>,
) -> Result<(), AppError> {
    send_push(state, user_id, title, body, notification_type, data).await
}

async fn send_push(
    state: &AppState,
    user_id: &str,
    title: &str,
    body: &str,
    notification_type: &NotificationType,
    data: Option<serde_json::Value>,
) -> Result<(), AppError> {
    let tokens = state.aegis_list_docs("push_tokens").await?;
    let user_tokens: Vec<&serde_json::Value> = tokens
        .iter()
        .filter(|t| t.get("user_id").and_then(|v| v.as_str()) == Some(user_id))
        .collect();

    if user_tokens.is_empty() {
        return Ok(());
    }

    let channel_id = notification_type.channel_id();

    let messages: Vec<serde_json::Value> = user_tokens
        .iter()
        .filter_map(|t| {
            t.get("token").and_then(|v| v.as_str()).map(|token| {
                let mut msg = serde_json::json!({
                    "to": token,
                    "title": title,
                    "body": body,
                    "sound": "default",
                    "channelId": channel_id,
                    "data": {
                        "type": notification_type.as_str(),
                    },
                });
                // Merge extra data into the data field
                if let Some(ref extra) = data {
                    if let (Some(msg_data), Some(extra_obj)) = (
                        msg.get_mut("data").and_then(|d| d.as_object_mut()),
                        extra.as_object(),
                    ) {
                        for (k, v) in extra_obj {
                            msg_data.insert(k.clone(), v.clone());
                        }
                    }
                }
                msg
            })
        })
        .collect();

    // Send via Expo Push API
    let resp = state
        .http_client
        .post("https://exp.host/--/api/v2/push/send")
        .json(&messages)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Push send failed: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        tracing::warn!("Push notification failed: {text}");
    }

    Ok(())
}

fn sha256_short(input: &str) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(input.as_bytes());
    hex::encode(&hash[..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_token_request_deserialize() {
        let json = r#"{"token":"ExponentPushToken[xxx]","platform":"android","device_name":"Pixel 7"}"#;
        let req: RegisterTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.token, "ExponentPushToken[xxx]");
        assert_eq!(req.platform, "android");
        assert_eq!(req.device_name, "Pixel 7");
    }

    #[test]
    fn push_token_serialize_roundtrip() {
        let pt = PushToken {
            id: "user-1_abc".into(),
            user_id: "user-1".into(),
            token: "ExponentPushToken[test]".into(),
            platform: "ios".into(),
            device_name: "iPhone 15".into(),
            registered_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_value(&pt).unwrap();
        let parsed: PushToken = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.user_id, "user-1");
        assert_eq!(parsed.platform, "ios");
    }

    #[test]
    fn sha256_short_deterministic() {
        let a = sha256_short("test-token");
        let b = sha256_short("test-token");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16); // 8 bytes = 16 hex chars
    }

    #[test]
    fn sha256_short_different_inputs() {
        let a = sha256_short("token-a");
        let b = sha256_short("token-b");
        assert_ne!(a, b);
    }

    #[test]
    fn notification_type_channel_training() {
        assert_eq!(NotificationType::TrainingComplete.channel_id(), "training");
        assert_eq!(NotificationType::TrainingFailed.channel_id(), "training");
        assert_eq!(NotificationType::TrainingEpochMilestone.channel_id(), "training");
    }

    #[test]
    fn notification_type_channel_alerts() {
        assert_eq!(NotificationType::SecurityAlert.channel_id(), "alerts");
    }

    #[test]
    fn notification_type_channel_default() {
        assert_eq!(NotificationType::AccountVerified.channel_id(), "default");
        assert_eq!(NotificationType::SubscriptionChanged.channel_id(), "default");
    }

    #[test]
    fn notification_type_as_str() {
        assert_eq!(NotificationType::TrainingComplete.as_str(), "training_complete");
        assert_eq!(NotificationType::TrainingFailed.as_str(), "training_failed");
        assert_eq!(NotificationType::DeploymentReady.as_str(), "deployment_ready");
    }
}
