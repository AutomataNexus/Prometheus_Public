// ============================================================================
// File: router.rs
// Description: Axum router setup with all API routes, middleware, CORS, and static file serving
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post, put, delete},
    middleware,
    Router,
};
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    services::ServeDir,
};
use crate::state::AppState;
use std::sync::Arc;
use axum::Extension;
use crate::api;
use crate::auth;
use crate::ws;

pub fn create_router(state: AppState, shield: Arc<prometheus_shield::Shield>, email_service: Option<Arc<prometheus_email::EmailService>>) -> Router {
    // Public auth routes (no middleware)
    let auth_routes = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/session", get(auth::get_session))
        .route("/auth/me", get(auth::get_me))
        .route("/auth/cli/init", post(api::cli_auth::cli_auth_init))
        .route("/auth/cli/poll", get(api::cli_auth::cli_auth_poll))
        .route("/auth/cli/verify", post(api::cli_auth::cli_auth_verify))
        // Public user lifecycle
        .route("/auth/signup", post(api::users::signup))
        .route("/auth/verify-email", post(api::users::verify_email))
        .route("/auth/resend-verification", post(api::users::resend_verification))
        .route("/auth/forgot-password", post(api::users::forgot_password))
        .route("/auth/reset-password", post(api::users::reset_password))
        // MFA validation (during login, before full auth)
        .route("/auth/mfa/validate", post(api::mfa::validate))
        // Stripe webhook (public, verified via signature)
        .route("/billing/webhook", post(api::billing::webhook))
        .with_state(state.clone());

    // Protected API routes (require valid Aegis-DB bearer token)
    let protected_routes = Router::new()
        // Datasets
        .route("/datasets", get(api::datasets::list_datasets).post(api::datasets::upload_dataset))
        .route("/datasets/connect", post(api::datasets::connect_source))
        .route("/datasets/catalog", get(api::datasets::list_catalog))
        .route("/datasets/catalog/import", post(api::datasets::import_catalog_dataset))
        .route("/datasets/:id", get(api::datasets::get_dataset).delete(api::datasets::delete_dataset))
        .route("/datasets/:id/preview", get(api::datasets::get_dataset_preview))
        .route("/datasets/:id/status", post(api::datasets::toggle_dataset_status))
        .route("/datasets/:id/validate", post(api::datasets::validate_dataset))
        .route("/datasets/:id/unlock", post(api::datasets::unlock_dataset))
        .route("/datasets/:id/recommend", get(api::datasets::recommend_models))
        // Training
        .route("/training", get(api::training::list_training_runs))
        .route("/training/start", post(api::training::start_training))
        .route("/training/:id", get(api::training::get_training_run))
        .route("/training/:id/stop", post(api::training::stop_training))
        .route("/training/queue", get(api::training::get_queue_status))
        .route("/training/clear", post(api::training::clear_completed_training))
        // Models
        .route("/models", get(api::models::list_models))
        .route("/models/:id", get(api::models::get_model).delete(api::models::delete_model).put(api::models::rename_model))
        .route("/models/:id/download", get(api::models::download_model_format))
        .route("/models/:id/convert", post(api::models::convert_model))
        .route("/models/:id/compare", post(api::models::compare_models))
        .route("/models/quantize", post(api::models::quantize_model))
        // Deployment
        .route("/deployments", get(api::deployment::list_deployments).post(api::deployment::create_deployment))
        .route("/deployments/targets", get(api::deployment::list_targets).post(api::deployment::add_target))
        .route("/deployments/targets/:id", delete(api::deployment::delete_target))
        .route("/deployments/:id", get(api::deployment::get_deployment))
        .route("/deployments/:id/binary", get(api::deployment::download_binary))
        // Agent
        .route("/agent/chat", post(api::agents::chat))
        .route("/agent/analyze", post(api::agents::analyze))
        .route("/agent/history", get(api::agents::get_history))
        // Evaluation
        .route("/evaluations", get(api::evaluation::list_evaluations))
        .route("/evaluations/:id", get(api::evaluation::get_evaluation))
        .route("/evaluations/:id/gradient", post(api::evaluation::run_gradient_eval))
        // Email
        .route("/email/welcome", post(api::email::send_welcome))
        .route("/email/verification", post(api::email::send_verification))
        .route("/email/password-reset", post(api::email::send_password_reset))
        .route("/email/support/confirm", post(api::email::send_support_confirmation))
        .route("/email/support/response", post(api::email::send_support_response))
        .route("/email/security-alert", post(api::email::send_security_alert))
        .route("/email/daily-report", post(api::email::send_daily_report))
        // Service accounts (admin only)
        .route("/service-accounts", get(api::service_accounts::list_service_accounts).post(api::service_accounts::create_service_account))
        .route("/service-accounts/:username", delete(api::service_accounts::delete_service_account))
        // Billing & Subscriptions
        .route("/billing/subscription", get(api::billing::get_subscription))
        .route("/billing/checkout", post(api::billing::create_checkout))
        .route("/billing/portal", post(api::billing::create_portal))
        .route("/billing/usage", get(api::billing::get_usage))
        // User Profile & Preferences
        .route("/profile", get(api::profile::get_profile))
        .route("/profile/preferences", get(api::profile::get_preferences).put(api::profile::update_preferences))
        .route("/profile/token-balance", get(api::profile::get_token_balance))
        // MFA (authenticated)
        .route("/mfa/setup", post(api::mfa::setup))
        .route("/mfa/verify", post(api::mfa::verify))
        .route("/mfa/disable", post(api::mfa::disable))
        // Change password (authenticated)
        .route("/auth/change-password", put(api::users::change_password))
        // Admin user management
        .route("/admin/users", get(api::users::admin_list_users).post(api::users::admin_create_user))
        .route("/admin/users/:username", get(api::users::admin_get_user).put(api::users::admin_update_user).delete(api::users::admin_delete_user))
        .route("/admin/users/:username/approve", post(api::users::admin_approve_user))
        // Saved connections (encrypted credential vault)
        .route("/connections", get(api::datasets::list_connections).post(api::datasets::save_connection))
        .route("/connections/:id", delete(api::datasets::delete_connection))
        .route("/connections/:id/use", post(api::datasets::use_connection))
        // Ingestion keys
        .route("/ingestion-keys", get(api::ingestion_keys::list_keys).post(api::ingestion_keys::create_key))
        .route("/ingestion-keys/:id", delete(api::ingestion_keys::delete_key))
        // Push notifications
        .route("/push/register", post(api::push::register_token))
        // System
        .route("/system/metrics", get(api::health::system_metrics))
        // Apply auth middleware to all protected routes
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::middleware::auth_middleware,
        ))
        .with_state(state.clone());

    Router::new()
        // Health check (no auth)
        .route("/health", get(api::health::health))
        // Public auth routes
        .nest("/api/v1", auth_routes)
        // Protected API routes
        .nest("/api/v1", protected_routes)
        // WebSocket routes (auth checked in handler)
        .route("/ws/training/:id", get(ws::training_ws))
        // Static file serving for WASM UI
        .nest_service("/assets", ServeDir::new("assets"))
        // Leptos SPA fallback
        .fallback_service(ServeDir::new("dist").fallback(
            tower_http::services::ServeFile::new("dist/index.html"),
        ))
        .layer(DefaultBodyLimit::max(500 * 1024 * 1024)) // 500 MB upload limit
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn(prometheus_shield::shield_middleware))
        .layer(Extension(shield))
        .layer({
            let email_ext = email_service.unwrap_or_else(|| {
                // Default email config when RESEND_API_KEY is not set
                Arc::new(prometheus_email::EmailService::new(
                    prometheus_email::EmailConfig {
                        resend_api_key: String::new(),
                        from: "noreply@example.com".into(),
                        reply_to: "noreply@example.com".into(),
                        base_url: "http://localhost".into(),
                        company_name: "Automata Controls".into(),
                        support_email: "support@example.com".into(),
                        security_recipients: vec![],
                    },
                ))
            });
            Extension(email_ext)
        })
        .with_state(state)
}
