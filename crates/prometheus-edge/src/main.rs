// ============================================================================
// File: main.rs
// Description: Edge inference daemon HTTP server for Raspberry Pi controllers
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Prometheus Edge Inference Daemon
//!
//! A lightweight HTTP server that runs on Raspberry Pi controllers, polling
//! sensors from the NexusEdge hardware daemon and executing model inference at
//! configurable intervals. Designed for resource-constrained ARM devices.
//!
//! ## Usage
//!
//! ```text
//! prometheus-edge --config /etc/prometheus/edge.json
//! prometheus-edge -c /etc/prometheus/edge.toml
//! ```
//!
//! ## HTTP API (port 6200 by default)
//!
//! - `GET  /health`   — Liveness check
//! - `GET  /predict`  — Run inference on latest cached sensor data
//! - `POST /predict`  — Run inference on provided JSON sensor data
//! - `GET  /metrics`  — Current model and runtime metrics

mod config;
mod inference;
mod sensor_poll;

use config::EdgeConfig;
use inference::{InferenceEngine, PredictionResult};
use sensor_poll::{SensorPoller, SensorReading};

use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

// ── Constants ─────────────────────────────────────────────────────────────────

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_CONFIG_PATH: &str = "/etc/prometheus/edge.json";

// ── Shared application state ──────────────────────────────────────────────────

/// State shared between the HTTP server, the inference loop, and the sensor
/// poller via `Arc`.
struct AppState {
    engine: InferenceEngine,
    config: EdgeConfig,
    latest_reading: Arc<Mutex<SensorReading>>,
    latest_prediction: RwLock<Option<PredictionResult>>,
    start_time: Instant,
}

// ── CLI argument parsing ──────────────────────────────────────────────────────

struct CliArgs {
    config_path: String,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut config_path = DEFAULT_CONFIG_PATH.to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                i += 1;
                if i < args.len() {
                    config_path = args[i].clone();
                } else {
                    eprintln!("error: --config requires a file path argument");
                    std::process::exit(1);
                }
            }
            "--version" | "-V" => {
                println!("prometheus-edge {}", VERSION);
                std::process::exit(0);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                eprintln!("error: unknown argument '{}'", other);
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    CliArgs { config_path }
}

fn print_usage() {
    eprintln!(
        "Prometheus Edge Inference Daemon v{}\n\
         \n\
         USAGE:\n\
         \x20   prometheus-edge [OPTIONS]\n\
         \n\
         OPTIONS:\n\
         \x20   -c, --config <PATH>  Path to configuration file [default: {}]\n\
         \x20   -V, --version        Print version information\n\
         \x20   -h, --help           Print this help message",
        VERSION, DEFAULT_CONFIG_PATH,
    );
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    // Initialize structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .compact()
        .init();

    tracing::info!(version = VERSION, "starting prometheus-edge");

    // Parse CLI arguments.
    let args = parse_args();

    // Load configuration.
    let config = match EdgeConfig::load(&args.config_path) {
        Ok(cfg) => {
            tracing::info!(
                config_path = %args.config_path,
                model_path = %cfg.model_path,
                nexus_url = %cfg.nexus_url,
                features = ?cfg.features,
                "configuration loaded"
            );
            cfg
        }
        Err(e) => {
            tracing::error!(error = %e, path = %args.config_path, "failed to load configuration");
            eprintln!("fatal: {}", e);
            std::process::exit(1);
        }
    };

    // Load the inference model.
    let engine = match inference::load_model(&config.model_path) {
        Ok(eng) => {
            tracing::info!(
                model = %eng.metadata().name,
                input_dim = eng.input_dim(),
                output_dim = eng.output_dim(),
                "model loaded"
            );
            eng
        }
        Err(e) => {
            tracing::error!(error = %e, path = %config.model_path, "failed to load model");
            eprintln!("fatal: {}", e);
            std::process::exit(1);
        }
    };

    // Start the sensor poller.
    let mut poller = SensorPoller::new(&config.nexus_url, config.poll_interval_secs);
    let latest_reading = poller.latest_handle();
    poller.start();

    // Build shared state.
    let state = Arc::new(AppState {
        engine,
        config: config.clone(),
        latest_reading,
        latest_prediction: RwLock::new(None),
        start_time: Instant::now(),
    });

    // Spawn the periodic inference loop.
    let inference_state = Arc::clone(&state);
    let inference_handle = std::thread::Builder::new()
        .name("inference-loop".into())
        .spawn(move || {
            inference_loop(inference_state);
        })
        .expect("failed to spawn inference loop thread");

    // Start the HTTP server on the configured port.
    let bind_addr = format!("0.0.0.0:{}", config.http_port);
    tracing::info!(addr = %bind_addr, "starting HTTP server");

    let server = match tiny_http::Server::http(&bind_addr) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, addr = %bind_addr, "failed to bind HTTP server");
            eprintln!("fatal: failed to bind to {}: {}", bind_addr, e);
            std::process::exit(1);
        }
    };

    tracing::info!(addr = %bind_addr, "prometheus-edge is ready");

    // HTTP request loop — runs on the main thread.
    for request in server.incoming_requests() {
        let state = Arc::clone(&state);
        // Handle each request in a short-lived thread to avoid blocking the
        // accept loop (tiny_http is synchronous).
        std::thread::spawn(move || {
            handle_request(request, &state);
        });
    }

    // Cleanup (unreachable in normal operation, but keeps the compiler happy).
    drop(inference_handle);
    drop(poller);
}

// ── Periodic inference loop ───────────────────────────────────────────────────

fn inference_loop(state: Arc<AppState>) {
    let interval = Duration::from_secs(state.config.inference_interval_secs);

    tracing::info!(
        interval_secs = state.config.inference_interval_secs,
        "inference loop started"
    );

    loop {
        std::thread::sleep(interval);

        // Grab the latest sensor reading.
        let reading = state
            .latest_reading
            .lock()
            .expect("sensor reading lock poisoned")
            .clone();

        if reading.values.is_empty() {
            tracing::debug!("skipping inference: no sensor data available");
            continue;
        }

        // Extract and normalize features.
        let features = state.config.extract_features(&reading.values);

        if features.len() != state.engine.input_dim() {
            tracing::warn!(
                expected = state.engine.input_dim(),
                got = features.len(),
                "feature vector size does not match model input dimension"
            );
            continue;
        }

        match state.engine.predict(&features) {
            Ok(result) => {
                if result.anomaly_score > state.config.anomaly_threshold {
                    tracing::warn!(
                        anomaly_score = result.anomaly_score,
                        threshold = state.config.anomaly_threshold,
                        "anomaly detected"
                    );
                } else {
                    tracing::info!(
                        anomaly_score = result.anomaly_score,
                        health_score = result.health_score,
                        prediction_number = result.prediction_number,
                        "inference complete"
                    );
                }

                if let Ok(mut guard) = state.latest_prediction.write() {
                    *guard = Some(result);
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "inference failed");
            }
        }
    }
}

// ── HTTP request handling ─────────────────────────────────────────────────────

fn handle_request(request: tiny_http::Request, state: &AppState) {
    let method = request.method().to_string();
    let path = request.url().to_string();

    // Strip query string for routing.
    let route = path.split('?').next().unwrap_or(&path);

    tracing::debug!(method = %method, path = %route, "incoming request");

    let result = match (method.as_str(), route) {
        ("GET", "/health") => handle_health(request, state),
        ("GET", "/predict") => handle_get_predict(request, state),
        ("POST", "/predict") => handle_post_predict(request, state),
        ("GET", "/metrics") => handle_metrics(request, state),
        _ => respond_json(
            request,
            404,
            &ErrorResponse {
                error: "not_found".into(),
                message: format!("no handler for {} {}", method, route),
            },
        ),
    };

    if let Err(e) = result {
        tracing::error!(error = %e, "failed to send HTTP response");
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

fn handle_health(
    request: tiny_http::Request,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = HealthResponse {
        status: "ok".into(),
        version: VERSION.into(),
        unit_id: state.config.unit_id.clone(),
        uptime_secs: state.start_time.elapsed().as_secs(),
        model: state.engine.metadata().name.clone(),
        model_version: state.engine.metadata().version.clone(),
        timestamp: Utc::now(),
    };
    respond_json(request, 200, &response)
}

fn handle_get_predict(
    request: tiny_http::Request,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    // Grab latest sensor data and run inference on demand.
    let reading = state
        .latest_reading
        .lock()
        .expect("sensor reading lock poisoned")
        .clone();

    if reading.values.is_empty() {
        return respond_json(
            request,
            503,
            &ErrorResponse {
                error: "no_data".into(),
                message: "no sensor data available yet".into(),
            },
        );
    }

    let features = state.config.extract_features(&reading.values);

    match state.engine.predict(&features) {
        Ok(result) => {
            let response = PredictResponse {
                prediction: result,
                sensor_timestamp: reading.timestamp,
                features_used: state.config.features.clone(),
            };
            respond_json(request, 200, &response)
        }
        Err(e) => respond_json(
            request,
            500,
            &ErrorResponse {
                error: "inference_error".into(),
                message: e.to_string(),
            },
        ),
    }
}

fn handle_post_predict(
    mut request: tiny_http::Request,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the request body.
    let mut body = String::new();
    request.as_reader().read_to_string(&mut body)?;

    if body.is_empty() {
        return respond_json(
            request,
            400,
            &ErrorResponse {
                error: "empty_body".into(),
                message: "request body must be a JSON object with sensor values".into(),
            },
        );
    }

    // Try parsing as a raw feature vector first, then as a named-value map.
    let features: Vec<f32> = if let Ok(raw) = serde_json::from_str::<Vec<f32>>(&body) {
        raw
    } else if let Ok(map) = serde_json::from_str::<HashMap<String, f32>>(&body) {
        state.config.extract_features(&map)
    } else if let Ok(wrapper) = serde_json::from_str::<PostPredictRequest>(&body) {
        if !wrapper.values.is_empty() {
            state.config.extract_features(&wrapper.values)
        } else {
            wrapper.features
        }
    } else {
        return respond_json(
            request,
            400,
            &ErrorResponse {
                error: "invalid_json".into(),
                message: "expected a JSON array of floats, an object with sensor names, \
                          or {\"values\": {...}} / {\"features\": [...]}"
                    .into(),
            },
        );
    };

    if features.is_empty() {
        return respond_json(
            request,
            400,
            &ErrorResponse {
                error: "empty_features".into(),
                message: "no features could be extracted from the request".into(),
            },
        );
    }

    match state.engine.predict(&features) {
        Ok(result) => {
            let response = PredictResponse {
                prediction: result,
                sensor_timestamp: Utc::now(),
                features_used: state.config.features.clone(),
            };
            respond_json(request, 200, &response)
        }
        Err(e) => respond_json(
            request,
            422,
            &ErrorResponse {
                error: "inference_error".into(),
                message: e.to_string(),
            },
        ),
    }
}

fn handle_metrics(
    request: tiny_http::Request,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    let latest_prediction = state
        .latest_prediction
        .read()
        .expect("prediction lock poisoned")
        .clone();

    let reading = state
        .latest_reading
        .lock()
        .expect("sensor reading lock poisoned")
        .clone();

    let response = MetricsResponse {
        unit_id: state.config.unit_id.clone(),
        model_name: state.engine.metadata().name.clone(),
        model_version: state.engine.metadata().version.clone(),
        input_dim: state.engine.input_dim(),
        output_dim: state.engine.output_dim(),
        total_predictions: state.engine.prediction_count(),
        uptime_secs: state.start_time.elapsed().as_secs(),
        latest_anomaly_score: latest_prediction
            .as_ref()
            .map(|p| p.anomaly_score),
        latest_health_score: latest_prediction
            .as_ref()
            .map(|p| p.health_score),
        latest_prediction_time: latest_prediction
            .as_ref()
            .map(|p| p.timestamp),
        sensor_count: reading.values.len(),
        last_sensor_poll: reading.timestamp,
        anomaly_threshold: state.config.anomaly_threshold,
        timestamp: Utc::now(),
    };

    respond_json(request, 200, &response)
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    unit_id: String,
    uptime_secs: u64,
    model: String,
    model_version: String,
    timestamp: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
struct PredictResponse {
    prediction: PredictionResult,
    sensor_timestamp: chrono::DateTime<Utc>,
    features_used: Vec<String>,
}

#[derive(Serialize)]
struct MetricsResponse {
    unit_id: String,
    model_name: String,
    model_version: String,
    input_dim: usize,
    output_dim: usize,
    total_predictions: u64,
    uptime_secs: u64,
    latest_anomaly_score: Option<f32>,
    latest_health_score: Option<f32>,
    latest_prediction_time: Option<chrono::DateTime<Utc>>,
    sensor_count: usize,
    last_sensor_poll: chrono::DateTime<Utc>,
    anomaly_threshold: f32,
    timestamp: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

#[derive(serde::Deserialize)]
struct PostPredictRequest {
    #[serde(default)]
    values: HashMap<String, f32>,
    #[serde(default)]
    features: Vec<f32>,
}

// ── Response helpers ──────────────────────────────────────────────────────────

fn respond_json<T: Serialize>(
    request: tiny_http::Request,
    status_code: i32,
    body: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(body)?;
    let response = tiny_http::Response::from_string(json)
        .with_status_code(tiny_http::StatusCode(status_code as u16))
        .with_header(
            tiny_http::Header::from_bytes(
                b"Content-Type" as &[u8],
                b"application/json" as &[u8],
            )
            .expect("valid header"),
        )
        .with_header(
            tiny_http::Header::from_bytes(
                b"X-Prometheus-Edge-Version" as &[u8],
                VERSION.as_bytes(),
            )
            .expect("valid header"),
        );
    request.respond(response)?;
    Ok(())
}
