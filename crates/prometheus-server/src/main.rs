// ============================================================================
// File: main.rs
// Description: Application entry point — initializes tracing, loads config, and starts the Axum HTTP server
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

mod api;
mod auth;
mod config;
mod error;
mod router;
mod state;
mod ws;

use config::ServerConfig;
use state::AppState;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,prometheus_server=debug")),
        )
        .init();

    let config = ServerConfig::default();
    let addr = format!("{}:{}", config.host, config.port);

    tracing::info!("Starting Prometheus server");
    tracing::info!("  Listening on: {addr}");
    tracing::info!("  Aegis-DB URL: {}", config.aegis_db_url);
    tracing::info!(
        "  DO GenAI: {}",
        if config.gradient_endpoint.is_some() && config.gradient_api_key.is_some() {
            format!("configured ({})", config.gradient_endpoint.as_deref().unwrap_or(""))
        } else {
            "not configured (PrometheusForge will use local inference)".to_string()
        }
    );

    // Create data directory
    let _ = tokio::fs::create_dir_all(&config.data_dir).await;
    let _ = tokio::fs::create_dir_all(format!("{}/datasets", config.data_dir)).await;
    let _ = tokio::fs::create_dir_all(format!("{}/models", config.data_dir)).await;
    let _ = tokio::fs::create_dir_all(format!("{}/deployments", config.data_dir)).await;

    // Initialize Aegis-DB collections
    let state = AppState::new(config);
    initialize_aegis_db(&state).await;

    // Initialize Shield security engine
    let shield = std::sync::Arc::new(prometheus_shield::Shield::new(
        prometheus_shield::ShieldConfig::default(),
    ));
    tracing::info!("Shield security engine initialized (block_threshold={:.1})", shield.config.block_threshold);

    // Initialize Email service (optional — runs without RESEND_API_KEY)
    let email_service = match prometheus_email::EmailService::from_env() {
        Ok(svc) => {
            tracing::info!("Email service initialized (from: {})", svc.config().from);
            Some(std::sync::Arc::new(svc))
        }
        Err(e) => {
            tracing::warn!("Email service unavailable: {e} — email features disabled");
            None
        }
    };

    // Spawn background data lifecycle task (compress/delete inactive datasets)
    tokio::spawn(api::data_lifecycle::run_lifecycle_sweep(state.clone()));
    tracing::info!("Data lifecycle manager started (compress after {}h, free retention {}d)",
        24, 30);

    let app = router::create_router(state, shield, email_service);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    tracing::info!("Prometheus is ready at http://{addr}");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

async fn initialize_aegis_db(state: &AppState) {
    let collections = [
        "datasets",
        "models",
        "training_plans",
        "deployments",
        "evaluations",
        "agent_history",
        "cli_sessions",
        "subscriptions",
        "user_status",
        "email_verifications",
        "password_resets",
        "mfa_secrets",
        "user_preferences",
        "push_tokens",
        "ingestion_keys",
    ];

    for collection in &collections {
        let _ = state
            .aegis_request(
                reqwest::Method::POST,
                "/api/v1/documents/collections",
                Some(serde_json::json!({ "name": collection })),
            )
            .await;
    }

    // Create SQL tables
    let sql_statements = [
        "CREATE TABLE IF NOT EXISTS prometheus_users (id INT PRIMARY KEY, username TEXT UNIQUE NOT NULL, email TEXT, role TEXT DEFAULT 'operator', created_at TEXT)",
        "CREATE TABLE IF NOT EXISTS prometheus_audit (id INT PRIMARY KEY, user_id INT, action TEXT, resource TEXT, details TEXT, timestamp TEXT)",
    ];

    for sql in &sql_statements {
        let _ = state
            .aegis_request(
                reqwest::Method::POST,
                "/api/v1/query",
                Some(serde_json::json!({ "sql": sql })),
            )
            .await;
    }

    tracing::info!("Aegis-DB schema initialized");

    // Seed admin user if it doesn't exist
    seed_admin_user(state).await;
}

async fn seed_admin_user(state: &AppState) {
    // Check if admin user already exists
    let resp = state
        .http_client
        .get(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .send()
        .await;

    let admin_exists = resp
        .ok()
        .and_then(|r| {
            if r.status().is_success() {
                Some(r)
            } else {
                None
            }
        });

    // Try to parse users list and check for "DevOps"
    let already_exists = if let Some(resp) = admin_exists {
        let users: Vec<serde_json::Value> = resp.json().await.unwrap_or_default();
        users.iter().any(|u| {
            u.get("username").and_then(|v| v.as_str()) == Some("DevOps")
        })
    } else {
        false
    };

    if already_exists {
        tracing::info!("Admin user 'DevOps' already exists");
        return;
    }

    // Create admin user
    let create_resp = state
        .http_client
        .post(format!("{}/api/v1/admin/users", state.config.aegis_db_url))
        .json(&serde_json::json!({
            "username": std::env::var("PROMETHEUS_ADMIN_USER").unwrap_or_else(|_| "admin".into()),
            "email": std::env::var("PROMETHEUS_ADMIN_EMAIL").unwrap_or_else(|_| "admin@localhost".into()),
            "password": std::env::var("PROMETHEUS_ADMIN_PASS").unwrap_or_else(|_| "changeme123".into()),
            "role": "admin",
        }))
        .send()
        .await;

    match create_resp {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let user_id = body
                .get("user")
                .and_then(|u| u.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("DevOps")
                .to_string();

            let now = chrono::Utc::now();

            // Mark as verified and approved
            let status = serde_json::json!({
                "id": user_id,
                "username": "DevOps",
                "email": "devops@automatanexus.com",
                "email_verified": true,
                "account_approved": true,
                "created_at": now.to_rfc3339(),
                "verified_at": now.to_rfc3339(),
                "approved_at": now.to_rfc3339(),
            });
            let _ = state.aegis_create_doc("user_status", status).await;

            // Create enterprise subscription
            let sub = serde_json::json!({
                "id": user_id,
                "user_id": user_id,
                "tier": "enterprise",
                "stripe_customer_id": null,
                "stripe_subscription_id": null,
                "tokens_used": 0,
                "tokens_limit": 500_000u64,
                "token_balance": 500_000u64,
                "tokens_used_this_period": 0,
                "current_period_start": now.to_rfc3339(),
                "current_period_end": (now + chrono::Duration::days(365)).to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339(),
            });
            let _ = state.aegis_create_doc("subscriptions", sub).await;

            tracing::info!("Seeded admin user 'DevOps' (devops@automatanexus.com)");
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!("Failed to seed admin user (HTTP {}): {}", status, body);
        }
        Err(e) => {
            tracing::warn!("Failed to seed admin user: {e}");
        }
    }
}
