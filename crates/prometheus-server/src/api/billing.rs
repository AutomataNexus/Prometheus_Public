// ============================================================================
// File: billing.rs
// Description: Stripe billing, subscription management, and webhook handling
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

//! Stripe billing and subscription management.
//!
//! All Stripe keys and price IDs come from environment/vault — never hardcoded.
//! Subscription state is stored in Aegis-DB `subscriptions` collection.

use axum::{extract::State, http::HeaderMap, Extension, Json};
use serde::{Deserialize, Serialize};
use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Subscription tiers — stored in Aegis-DB, enforced server-side.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionTier {
    Free,
    Basic,
    Pro,
    Enterprise,
}

impl SubscriptionTier {
    pub fn monthly_token_limit(&self) -> u64 {
        match self {
            SubscriptionTier::Free => 5_000,
            SubscriptionTier::Basic => 10_000,
            SubscriptionTier::Pro => 50_000,
            SubscriptionTier::Enterprise => 1_000_000,
        }
    }

    pub fn max_concurrent_trainings(&self) -> u32 {
        match self {
            SubscriptionTier::Free => 1,
            SubscriptionTier::Basic => 2,
            SubscriptionTier::Pro => 5,
            SubscriptionTier::Enterprise => 20,
        }
    }

    pub fn max_datasets(&self) -> u32 {
        match self {
            SubscriptionTier::Free => 3,
            SubscriptionTier::Basic => 10,
            SubscriptionTier::Pro => 50,
            SubscriptionTier::Enterprise => 500,
        }
    }

    pub fn max_models(&self) -> u32 {
        match self {
            SubscriptionTier::Free => 2,
            SubscriptionTier::Basic => 5,
            SubscriptionTier::Pro => 25,
            SubscriptionTier::Enterprise => 200,
        }
    }

    pub fn max_dataset_size_bytes(&self) -> u64 {
        match self {
            SubscriptionTier::Free => 25 * 1024 * 1024,         // 25 MB
            SubscriptionTier::Basic => 50 * 1024 * 1024,        // 50 MB
            SubscriptionTier::Pro => 250 * 1024 * 1024,         // 250 MB
            SubscriptionTier::Enterprise => 50 * 1024 * 1024 * 1024, // 50 GB
        }
    }

    pub fn max_deployments(&self) -> u32 {
        match self {
            SubscriptionTier::Free => 1,
            SubscriptionTier::Basic => 3,
            SubscriptionTier::Pro => 10,
            SubscriptionTier::Enterprise => 100,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SubscriptionTier::Free => "Free",
            SubscriptionTier::Basic => "Basic",
            SubscriptionTier::Pro => "Pro",
            SubscriptionTier::Enterprise => "Enterprise",
        }
    }

    /// Whether this tier supports usage-based overage billing.
    pub fn allows_overage(&self) -> bool {
        !matches!(self, SubscriptionTier::Free)
    }
}

impl Default for SubscriptionTier {
    fn default() -> Self {
        SubscriptionTier::Free
    }
}

/// Subscription record stored in Aegis-DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub user_id: String,
    pub tier: SubscriptionTier,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub current_period_start: Option<String>,
    pub current_period_end: Option<String>,
    pub token_balance: u64,
    pub tokens_used_this_period: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutRequest {
    pub tier: SubscriptionTier,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub checkout_url: String,
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct PortalResponse {
    pub portal_url: String,
}

#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub tier: SubscriptionTier,
    pub token_balance: u64,
    pub tokens_used: u64,
    pub tokens_limit: u64,
    pub percentage_used: f64,
    pub unlimited: bool,
    pub max_concurrent_trainings: u32,
    pub max_datasets: u32,
    pub max_models: u32,
    pub max_deployments: u32,
    pub max_dataset_size_bytes: u64,
}

/// Get the current user's subscription.
pub async fn get_subscription(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Subscription>> {

    match load_subscription(&state, &auth.user_id).await {
        Ok(sub) => Ok(Json(sub)),
        Err(_) => {
            // Create default free subscription
            let sub = create_default_subscription(&state, &auth.user_id).await?;
            Ok(Json(sub))
        }
    }
}

/// Get token usage stats.
pub async fn get_usage(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<UsageResponse>> {

    // Admins get unlimited everything — no subscription lookup needed
    if auth.is_admin() {
        return Ok(Json(UsageResponse {
            tier: SubscriptionTier::Enterprise,
            token_balance: u64::MAX,
            tokens_used: 0,
            tokens_limit: u64::MAX,
            percentage_used: 0.0,
            unlimited: true,
            max_concurrent_trainings: u32::MAX,
            max_datasets: u32::MAX,
            max_models: u32::MAX,
            max_deployments: u32::MAX,
            max_dataset_size_bytes: u64::MAX,
        }));
    }

    let sub = match load_subscription(&state, &auth.user_id).await {
        Ok(s) => s,
        Err(_) => create_default_subscription(&state, &auth.user_id).await?,
    };

    let limit = sub.tier.monthly_token_limit();
    let percentage = if limit == u64::MAX {
        0.0
    } else if limit == 0 {
        100.0
    } else {
        (sub.tokens_used_this_period as f64 / limit as f64) * 100.0
    };

    Ok(Json(UsageResponse {
        tier: sub.tier.clone(),
        token_balance: sub.token_balance,
        tokens_used: sub.tokens_used_this_period,
        tokens_limit: limit,
        percentage_used: percentage,
        unlimited: false,
        max_concurrent_trainings: sub.tier.max_concurrent_trainings(),
        max_datasets: sub.tier.max_datasets(),
        max_models: sub.tier.max_models(),
        max_deployments: sub.tier.max_deployments(),
        max_dataset_size_bytes: sub.tier.max_dataset_size_bytes(),
    }))
}

/// Create a Stripe checkout session for upgrading subscription.
pub async fn create_checkout(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CheckoutRequest>,
) -> AppResult<Json<CheckoutResponse>> {

    let stripe_key = state.config.stripe_secret_key.as_ref()
        .ok_or_else(|| AppError::BadRequest("Stripe billing not configured".into()))?;

    let price_id = match req.tier {
        SubscriptionTier::Basic => state.config.stripe_price_basic.as_ref()
            .ok_or_else(|| AppError::BadRequest("Basic tier price not configured".into()))?,
        SubscriptionTier::Pro => state.config.stripe_price_pro.as_ref()
            .ok_or_else(|| AppError::BadRequest("Pro tier price not configured".into()))?,
        SubscriptionTier::Enterprise => state.config.stripe_price_enterprise.as_ref()
            .ok_or_else(|| AppError::BadRequest("Enterprise tier price not configured".into()))?,
        SubscriptionTier::Free => return Err(AppError::BadRequest("Cannot checkout for free tier".into())),
    };

    let base_url = state.config.public_url()
        .unwrap_or_else(|| format!("http://{}:{}", state.config.host, state.config.port));

    let success_url = req.success_url
        .unwrap_or_else(|| format!("{base_url}/billing?success=true"));
    let cancel_url = req.cancel_url
        .unwrap_or_else(|| format!("{base_url}/billing?canceled=true"));

    // Ensure user has a Stripe customer ID
    let sub = match load_subscription(&state, &auth.user_id).await {
        Ok(s) => s,
        Err(_) => create_default_subscription(&state, &auth.user_id).await?,
    };

    let customer_id = if let Some(cid) = &sub.stripe_customer_id {
        cid.clone()
    } else {
        // Create Stripe customer
        let customer = create_stripe_customer(&state, stripe_key, &auth.user_id, &auth.username).await?;
        // Update subscription with customer ID
        update_subscription_field(&state, &auth.user_id, "stripe_customer_id", &customer).await?;
        customer
    };

    // Build checkout line items — base subscription + optional metered overage
    let mut form_params: Vec<(&str, String)> = vec![
        ("mode", "subscription".into()),
        ("customer", customer_id.clone()),
        ("line_items[0][price]", price_id.clone()),
        ("line_items[0][quantity]", "1".into()),
        ("success_url", success_url.clone()),
        ("cancel_url", cancel_url.clone()),
        ("metadata[user_id]", auth.user_id.clone()),
        ("metadata[tier]", format!("{:?}", req.tier).to_lowercase()),
    ];

    // Add metered overage price as a second line item (usage-based billing)
    if let Some(overage_price) = &state.config.stripe_price_overage {
        if req.tier.allows_overage() {
            form_params.push(("line_items[1][price]", overage_price.clone()));
        }
    }

    let form_refs: Vec<(&str, &str)> = form_params.iter().map(|(k, v)| (*k, v.as_str())).collect();

    // Create checkout session via Stripe API
    let resp = state.http_client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .header("Authorization", format!("Bearer {stripe_key}"))
        .form(&form_refs)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Stripe request failed: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("Stripe checkout failed: {text}")));
    }

    let session: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("Stripe response parse error: {e}")))?;

    let checkout_url = session.get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("No URL in Stripe response".into()))?
        .to_string();

    let session_id = session.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(Json(CheckoutResponse { checkout_url, session_id }))
}

/// Create a Stripe customer portal session for managing subscription.
pub async fn create_portal(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<PortalResponse>> {

    let stripe_key = state.config.stripe_secret_key.as_ref()
        .ok_or_else(|| AppError::BadRequest("Stripe billing not configured".into()))?;

    let sub = load_subscription(&state, &auth.user_id).await
        .map_err(|_| AppError::BadRequest("No subscription found".into()))?;

    let customer_id = sub.stripe_customer_id
        .ok_or_else(|| AppError::BadRequest("No Stripe customer associated".into()))?;

    let base_url = state.config.public_url()
        .unwrap_or_else(|| format!("http://{}:{}", state.config.host, state.config.port));

    let resp = state.http_client
        .post("https://api.stripe.com/v1/billing_portal/sessions")
        .header("Authorization", format!("Bearer {stripe_key}"))
        .form(&[
            ("customer", customer_id.as_str()),
            ("return_url", &format!("{base_url}/billing")),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Stripe portal request failed: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("Stripe portal failed: {text}")));
    }

    let session: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("Stripe response parse error: {e}")))?;

    let portal_url = session.get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("No URL in Stripe portal response".into()))?
        .to_string();

    Ok(Json(PortalResponse { portal_url }))
}

/// Stripe webhook handler — processes subscription events.
/// This endpoint is PUBLIC (no auth middleware) but verified via Stripe signature.
pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> AppResult<Json<serde_json::Value>> {
    let webhook_secret = state.config.stripe_webhook_secret.as_ref()
        .ok_or_else(|| AppError::BadRequest("Webhook not configured".into()))?;

    // Verify Stripe signature
    let signature = headers.get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Stripe signature".into()))?;

    verify_stripe_signature(&body, signature, webhook_secret)?;

    let event: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AppError::BadRequest(format!("Invalid webhook payload: {e}")))?;

    let event_type = event.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    tracing::info!("Stripe webhook: {event_type}");

    match event_type {
        "checkout.session.completed" => {
            if let Some(session) = event.get("data").and_then(|d| d.get("object")) {
                handle_checkout_completed(&state, session).await?;
            }
        }
        "customer.subscription.updated" => {
            if let Some(subscription) = event.get("data").and_then(|d| d.get("object")) {
                handle_subscription_updated(&state, subscription).await?;
            }
        }
        "customer.subscription.deleted" => {
            if let Some(subscription) = event.get("data").and_then(|d| d.get("object")) {
                handle_subscription_deleted(&state, subscription).await?;
            }
        }
        "invoice.payment_succeeded" => {
            if let Some(invoice) = event.get("data").and_then(|d| d.get("object")) {
                handle_payment_succeeded(&state, invoice).await?;
            }
        }
        _ => {
            tracing::debug!("Unhandled Stripe event: {event_type}");
        }
    }

    Ok(Json(serde_json::json!({ "received": true })))
}

// ── Internal helpers ──────────────────────────────────────

async fn load_subscription(state: &AppState, user_id: &str) -> Result<Subscription, AppError> {
    let doc = state.aegis_get_doc("subscriptions", user_id).await?;
    serde_json::from_value(doc)
        .map_err(|e| AppError::Internal(format!("Subscription parse error: {e}")))
}

async fn create_default_subscription(state: &AppState, user_id: &str) -> Result<Subscription, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    let sub = Subscription {
        id: user_id.to_string(),
        user_id: user_id.to_string(),
        tier: SubscriptionTier::Free,
        stripe_customer_id: None,
        stripe_subscription_id: None,
        current_period_start: Some(now.clone()),
        current_period_end: None,
        token_balance: 1_000,
        tokens_used_this_period: 0,
        created_at: now.clone(),
        updated_at: now,
    };

    let doc = serde_json::to_value(&sub)
        .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;
    state.aegis_create_doc("subscriptions", doc).await?;

    Ok(sub)
}

async fn create_stripe_customer(
    state: &AppState,
    stripe_key: &str,
    user_id: &str,
    username: &str,
) -> Result<String, AppError> {
    let resp = state.http_client
        .post("https://api.stripe.com/v1/customers")
        .header("Authorization", format!("Bearer {stripe_key}"))
        .form(&[
            ("name", username),
            ("metadata[user_id]", user_id),
            ("metadata[source]", "prometheus"),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Stripe customer creation failed: {e}")))?;

    let customer: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("Stripe response parse error: {e}")))?;

    customer.get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AppError::Internal("No customer ID in Stripe response".into()))
}

async fn update_subscription_field(
    state: &AppState,
    user_id: &str,
    field: &str,
    value: &str,
) -> Result<(), AppError> {
    let mut sub = load_subscription(state, user_id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    sub.updated_at = now;

    match field {
        "stripe_customer_id" => sub.stripe_customer_id = Some(value.to_string()),
        "stripe_subscription_id" => sub.stripe_subscription_id = Some(value.to_string()),
        _ => {}
    }

    let doc = serde_json::to_value(&sub)
        .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;

    // Delete and recreate (Aegis-DB upsert pattern)
    let _ = state.aegis_delete_doc("subscriptions", user_id).await;
    state.aegis_create_doc("subscriptions", doc).await?;
    Ok(())
}

fn verify_stripe_signature(payload: &str, signature: &str, secret: &str) -> Result<(), AppError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Parse signature header: t=timestamp,v1=signature
    let mut timestamp = "";
    let mut sig_v1 = "";

    for part in signature.split(',') {
        let part = part.trim();
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = t;
        } else if let Some(v) = part.strip_prefix("v1=") {
            sig_v1 = v;
        }
    }

    if timestamp.is_empty() || sig_v1.is_empty() {
        return Err(AppError::Unauthorized("Invalid Stripe signature format".into()));
    }

    // Compute expected signature
    let signed_payload = format!("{timestamp}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::Internal("HMAC key error".into()))?;
    mac.update(signed_payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if expected != sig_v1 {
        return Err(AppError::Unauthorized("Invalid Stripe webhook signature".into()));
    }

    Ok(())
}

async fn handle_checkout_completed(state: &AppState, session: &serde_json::Value) -> Result<(), AppError> {
    let user_id = session.pointer("/metadata/user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("No user_id in checkout metadata".into()))?;

    let tier_str = session.pointer("/metadata/tier")
        .and_then(|v| v.as_str())
        .unwrap_or("pro");

    let tier = match tier_str {
        "basic" => SubscriptionTier::Basic,
        "enterprise" => SubscriptionTier::Enterprise,
        _ => SubscriptionTier::Pro,
    };

    let stripe_sub_id = session.get("subscription")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let stripe_customer_id = session.get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let now = chrono::Utc::now().to_rfc3339();
    let sub = Subscription {
        id: user_id.to_string(),
        user_id: user_id.to_string(),
        tier: tier.clone(),
        stripe_customer_id: Some(stripe_customer_id.to_string()),
        stripe_subscription_id: Some(stripe_sub_id.to_string()),
        current_period_start: Some(now.clone()),
        current_period_end: None,
        token_balance: tier.monthly_token_limit(),
        tokens_used_this_period: 0,
        created_at: now.clone(),
        updated_at: now,
    };

    let doc = serde_json::to_value(&sub)
        .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;

    let _ = state.aegis_delete_doc("subscriptions", user_id).await;
    state.aegis_create_doc("subscriptions", doc).await?;

    tracing::info!("Subscription upgraded: user={user_id} tier={}", tier.display_name());
    Ok(())
}

async fn handle_subscription_updated(state: &AppState, subscription: &serde_json::Value) -> Result<(), AppError> {
    let customer_id = subscription.get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Find user by stripe customer ID — list all subscriptions and match
    let all_subs = state.aegis_list_docs("subscriptions").await?;
    let user_sub = all_subs.iter().find(|doc| {
        doc.get("stripe_customer_id")
            .and_then(|v| v.as_str())
            == Some(customer_id)
    });

    if let Some(doc) = user_sub {
        let user_id = doc.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
        let status = subscription.get("status").and_then(|v| v.as_str()).unwrap_or("active");

        if status == "active" {
            tracing::info!("Subscription renewed for customer {customer_id}");
        } else {
            tracing::warn!("Subscription status changed to {status} for user {user_id}");
        }
    }

    Ok(())
}

async fn handle_subscription_deleted(state: &AppState, subscription: &serde_json::Value) -> Result<(), AppError> {
    let customer_id = subscription.get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let all_subs = state.aegis_list_docs("subscriptions").await?;
    let user_sub = all_subs.iter().find(|doc| {
        doc.get("stripe_customer_id")
            .and_then(|v| v.as_str())
            == Some(customer_id)
    });

    if let Some(doc) = user_sub {
        if let Some(user_id) = doc.get("user_id").and_then(|v| v.as_str()) {
            // Downgrade to free
            let now = chrono::Utc::now().to_rfc3339();
            let sub = Subscription {
                id: user_id.to_string(),
                user_id: user_id.to_string(),
                tier: SubscriptionTier::Free,
                stripe_customer_id: Some(customer_id.to_string()),
                stripe_subscription_id: None,
                current_period_start: Some(now.clone()),
                current_period_end: None,
                token_balance: SubscriptionTier::Free.monthly_token_limit(),
                tokens_used_this_period: 0,
                created_at: doc.get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&now)
                    .to_string(),
                updated_at: now,
            };

            let val = serde_json::to_value(&sub)
                .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;
            let _ = state.aegis_delete_doc("subscriptions", user_id).await;
            state.aegis_create_doc("subscriptions", val).await?;
            tracing::info!("Subscription canceled, downgraded to Free: user={user_id}");
        }
    }

    Ok(())
}

async fn handle_payment_succeeded(state: &AppState, invoice: &serde_json::Value) -> Result<(), AppError> {
    let customer_id = invoice.get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let all_subs = state.aegis_list_docs("subscriptions").await?;
    let user_sub = all_subs.iter().find(|doc| {
        doc.get("stripe_customer_id")
            .and_then(|v| v.as_str())
            == Some(customer_id)
    });

    if let Some(doc) = user_sub {
        if let Some(user_id) = doc.get("user_id").and_then(|v| v.as_str()) {
            // Reset token usage for new billing period
            let mut sub: Subscription = serde_json::from_value(doc.clone())
                .map_err(|e| AppError::Internal(format!("Parse error: {e}")))?;

            sub.tokens_used_this_period = 0;
            sub.token_balance = sub.tier.monthly_token_limit();
            sub.updated_at = chrono::Utc::now().to_rfc3339();

            let val = serde_json::to_value(&sub)
                .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;
            let _ = state.aegis_delete_doc("subscriptions", user_id).await;
            state.aegis_create_doc("subscriptions", val).await?;
            tracing::info!("Token balance reset for new period: user={user_id}");
        }
    }

    Ok(())
}

/// Load a user's subscription tier (returns Free if none exists).
pub async fn get_user_tier(state: &AppState, user_id: &str) -> SubscriptionTier {
    load_subscription(state, user_id)
        .await
        .map(|s| s.tier)
        .unwrap_or(SubscriptionTier::Free)
}

/// Check a resource count against a tier limit.
pub async fn enforce_limit(
    state: &AppState,
    user_id: &str,
    collection: &str,
    owner_field: &str,
    max_fn: fn(&SubscriptionTier) -> u32,
    resource_name: &str,
) -> Result<(), AppError> {
    let tier = get_user_tier(state, user_id).await;
    let limit = max_fn(&tier);
    if limit == u32::MAX {
        return Ok(());
    }

    let docs = state.aegis_list_docs(collection).await.unwrap_or_default();
    let count = docs.iter().filter(|d| {
        d.get(owner_field).and_then(|v| v.as_str()) == Some(user_id)
    }).count() as u32;

    if count >= limit {
        return Err(AppError::Forbidden(format!(
            "{} limit reached ({}/{}). Upgrade your plan for more.",
            resource_name, count, limit
        )));
    }

    Ok(())
}

/// Deduct tokens from a user's balance. Called by training endpoints.
/// If the user exceeds their plan limit and has overage billing enabled,
/// the overage is reported to the Stripe meter for usage-based billing.
pub async fn deduct_tokens(state: &AppState, user_id: &str, amount: u64) -> Result<(), AppError> {
    let mut sub = match load_subscription(state, user_id).await {
        Ok(s) => s,
        Err(_) => create_default_subscription(state, user_id).await?,
    };

    let limit = sub.tier.monthly_token_limit();

    // Check if this would exceed the limit
    if limit != u64::MAX && sub.tokens_used_this_period + amount > limit {
        if sub.tier.allows_overage() && sub.stripe_customer_id.is_some() {
            // Paid tier with overage billing — allow it but report to Stripe meter
            let overage = (sub.tokens_used_this_period + amount).saturating_sub(limit);
            report_usage_to_meter(state, &sub, overage).await;
        } else {
            return Err(AppError::Forbidden(format!(
                "Token limit exceeded. Used: {}, Limit: {}, Requested: {}. Upgrade your plan for more.",
                sub.tokens_used_this_period, limit, amount
            )));
        }
    }

    sub.tokens_used_this_period += amount;
    if sub.token_balance >= amount {
        sub.token_balance -= amount;
    } else {
        sub.token_balance = 0;
    }
    sub.updated_at = chrono::Utc::now().to_rfc3339();

    let doc = serde_json::to_value(&sub)
        .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;
    let _ = state.aegis_delete_doc("subscriptions", user_id).await;
    state.aegis_create_doc("subscriptions", doc).await?;

    Ok(())
}

/// Report token usage to Stripe Billing Meter for usage-based overage billing.
async fn report_usage_to_meter(state: &AppState, sub: &Subscription, token_count: u64) {
    let Some(stripe_key) = &state.config.stripe_secret_key else { return };
    let Some(customer_id) = &sub.stripe_customer_id else { return };

    // Report usage event to Stripe meter
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let result = state.http_client
        .post("https://api.stripe.com/v1/billing/meter_events")
        .header("Authorization", format!("Bearer {stripe_key}"))
        .form(&[
            ("event_name", "training_token_usage"),
            ("payload[stripe_customer_id]", customer_id.as_str()),
            ("payload[value]", &token_count.to_string()),
            ("timestamp", &timestamp),
        ])
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!("Reported {token_count} overage tokens to Stripe meter for user {}", sub.user_id);
        }
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!("Stripe meter event failed: {text}");
        }
        Err(e) => {
            tracing::warn!("Stripe meter event request error: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_tier_defaults() {
        let tier = SubscriptionTier::Free;
        assert_eq!(tier.monthly_token_limit(), 1_000);
        assert_eq!(tier.max_concurrent_trainings(), 1);
        assert_eq!(tier.max_datasets(), 3);
        assert_eq!(tier.max_models(), 2);
        assert_eq!(tier.max_deployments(), 1);
        assert_eq!(tier.max_dataset_size_bytes(), 50 * 1024 * 1024);
        assert_eq!(tier.display_name(), "Free");
        assert!(!tier.allows_overage());
    }

    #[test]
    fn basic_tier_limits() {
        let tier = SubscriptionTier::Basic;
        assert_eq!(tier.monthly_token_limit(), 10_000);
        assert_eq!(tier.max_concurrent_trainings(), 2);
        assert_eq!(tier.max_datasets(), 10);
        assert_eq!(tier.max_models(), 5);
        assert_eq!(tier.max_deployments(), 3);
        assert_eq!(tier.max_dataset_size_bytes(), 100 * 1024 * 1024);
        assert_eq!(tier.display_name(), "Basic");
        assert!(tier.allows_overage());
    }

    #[test]
    fn pro_tier_limits() {
        let tier = SubscriptionTier::Pro;
        assert_eq!(tier.monthly_token_limit(), 50_000);
        assert_eq!(tier.max_concurrent_trainings(), 5);
        assert_eq!(tier.max_datasets(), 50);
        assert_eq!(tier.max_models(), 25);
        assert_eq!(tier.max_deployments(), 10);
        assert_eq!(tier.max_dataset_size_bytes(), 500 * 1024 * 1024);
        assert_eq!(tier.display_name(), "Pro");
        assert!(tier.allows_overage());
    }

    #[test]
    fn enterprise_tier_limits() {
        let tier = SubscriptionTier::Enterprise;
        assert_eq!(tier.monthly_token_limit(), 500_000);
        assert_eq!(tier.max_concurrent_trainings(), 20);
        assert_eq!(tier.max_datasets(), 500);
        assert_eq!(tier.max_models(), 200);
        assert_eq!(tier.max_deployments(), 100);
        assert_eq!(tier.max_dataset_size_bytes(), 10 * 1024 * 1024 * 1024);
        assert_eq!(tier.display_name(), "Enterprise");
        assert!(tier.allows_overage());
    }











    #[test]
    fn default_tier_is_free() {
        let tier = SubscriptionTier::default();
        assert_eq!(tier, SubscriptionTier::Free);
    }

    #[test]
    fn tier_serialize_roundtrip() {
        let tiers = vec![SubscriptionTier::Free, SubscriptionTier::Basic, SubscriptionTier::Pro, SubscriptionTier::Enterprise];
        for tier in tiers {
            let json = serde_json::to_string(&tier).unwrap();
            let parsed: SubscriptionTier = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, tier);
        }
    }

    #[test]
    fn subscription_serialize_roundtrip() {
        let sub = Subscription {
            id: "user-1".into(),
            user_id: "user-1".into(),
            tier: SubscriptionTier::Pro,
            stripe_customer_id: Some("cus_test".into()),
            stripe_subscription_id: Some("sub_test".into()),
            current_period_start: Some("2026-01-01T00:00:00Z".into()),
            current_period_end: Some("2026-02-01T00:00:00Z".into()),
            token_balance: 45_000,
            tokens_used_this_period: 5_000,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-15T00:00:00Z".into(),
        };
        let json = serde_json::to_value(&sub).unwrap();
        let parsed: Subscription = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.tier, SubscriptionTier::Pro);
        assert_eq!(parsed.token_balance, 45_000);
        assert_eq!(parsed.tokens_used_this_period, 5_000);
    }

    #[test]
    fn usage_response_percentage() {
        let resp = UsageResponse {
            tier: SubscriptionTier::Pro,
            token_balance: 25_000,
            tokens_used: 25_000,
            tokens_limit: 50_000,
            percentage_used: 50.0,
            unlimited: false,
            max_concurrent_trainings: 5,
            max_datasets: 50,
            max_models: 25,
            max_deployments: 10,
            max_dataset_size_bytes: 500 * 1024 * 1024,
        };
        assert!((resp.percentage_used - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn checkout_request_deserialize() {
        let json = r#"{"tier":"pro"}"#;
        let req: CheckoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.tier, SubscriptionTier::Pro);
        assert!(req.success_url.is_none());
        assert!(req.cancel_url.is_none());
    }

    #[test]
    fn verify_stripe_signature_valid() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let secret = "whsec_test_secret";
        let payload = r#"{"type":"test"}"#;
        let timestamp = "1234567890";

        let signed = format!("{timestamp}.{payload}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());

        let header = format!("t={timestamp},v1={sig}");
        assert!(verify_stripe_signature(payload, &header, secret).is_ok());
    }

    #[test]
    fn verify_stripe_signature_invalid() {
        let result = verify_stripe_signature(
            r#"{"type":"test"}"#,
            "t=123,v1=invalid_signature",
            "whsec_test",
        );
        assert!(result.is_err());
    }

    #[test]
    fn verify_stripe_signature_missing_parts() {
        let result = verify_stripe_signature("payload", "invalid", "secret");
        assert!(result.is_err());
    }
}
