// ============================================================================
// File: sensor_poll.rs
// Description: Polling client that fetches and caches sensor data from NexusEdge daemon
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Sensor polling client for the NexusEdge hardware daemon.
//!
//! The NexusEdge daemon runs on each Raspberry Pi controller and exposes sensor
//! data over a local HTTP API on port 6100. This module provides a polling
//! client that periodically fetches the latest readings and caches them for
//! inference.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ── Data types ────────────────────────────────────────────────────────────────

/// A single snapshot of all sensor values at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    /// UTC timestamp when the reading was captured.
    pub timestamp: DateTime<Utc>,
    /// Sensor name -> value mapping.
    pub values: HashMap<String, f32>,
}

impl SensorReading {
    /// Create an empty reading at the current time.
    pub fn empty() -> Self {
        Self {
            timestamp: Utc::now(),
            values: HashMap::new(),
        }
    }

    /// Return the value of a named sensor, or `None` if absent.
    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<f32> {
        self.values.get(name).copied()
    }
}

/// Response shape returned by the NexusEdge `/sensors` endpoint.
#[derive(Debug, Deserialize)]
struct NexusSensorResponse {
    /// ISO 8601 timestamp string from the daemon.
    #[serde(default)]
    timestamp: Option<String>,
    /// Flat map of sensor name -> value.
    #[serde(default)]
    sensors: HashMap<String, f32>,
    /// Alternative field name used by some NexusEdge firmware versions.
    #[serde(default)]
    values: HashMap<String, f32>,
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors from sensor polling operations.
#[derive(Debug)]
pub enum PollError {
    /// HTTP request to NexusEdge failed.
    Http(String),
    /// Response body could not be parsed.
    Parse(String),
    /// The NexusEdge daemon returned a non-200 status.
    Status(i32),
}

impl std::fmt::Display for PollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PollError::Http(msg) => write!(f, "NexusEdge HTTP error: {}", msg),
            PollError::Parse(msg) => write!(f, "NexusEdge parse error: {}", msg),
            PollError::Status(code) => {
                write!(f, "NexusEdge returned status {}", code)
            }
        }
    }
}

impl std::error::Error for PollError {}

// ── One-shot poll ─────────────────────────────────────────────────────────────

/// Fetch the latest sensor readings from the NexusEdge daemon.
///
/// Makes a synchronous GET request to `{nexus_url}/sensors` with a 5-second
/// timeout.
pub fn poll_sensors(nexus_url: &str) -> Result<SensorReading, PollError> {
    let url = format!("{}/sensors", nexus_url.trim_end_matches('/'));

    tracing::debug!(url = %url, "polling NexusEdge sensors");

    let response = minreq::get(&url)
        .with_timeout(5)
        .send()
        .map_err(|e| PollError::Http(e.to_string()))?;

    if response.status_code != 200 {
        return Err(PollError::Status(response.status_code));
    }

    let body = response
        .as_str()
        .map_err(|e| PollError::Parse(e.to_string()))?;

    let parsed: NexusSensorResponse =
        serde_json::from_str(body).map_err(|e| PollError::Parse(e.to_string()))?;

    // Merge `sensors` and `values` fields (firmware compatibility).
    let mut values = parsed.sensors;
    for (k, v) in parsed.values {
        values.entry(k).or_insert(v);
    }

    // Parse timestamp from response, falling back to now.
    let timestamp = parsed
        .timestamp
        .as_deref()
        .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    tracing::debug!(
        sensor_count = values.len(),
        timestamp = %timestamp,
        "received sensor reading"
    );

    Ok(SensorReading { timestamp, values })
}

// ── Continuous poller ─────────────────────────────────────────────────────────

/// A background sensor poller that caches the latest reading.
///
/// The poller runs in a dedicated thread, fetching sensor data at the
/// configured interval and storing it behind an `Arc<Mutex<>>` for lock-free
/// reads from the HTTP handler.
pub struct SensorPoller {
    /// URL of the NexusEdge daemon.
    nexus_url: String,
    /// Poll interval.
    interval: Duration,
    /// Shared latest reading.
    latest: Arc<Mutex<SensorReading>>,
    /// Handle to the polling thread (if running).
    handle: Option<std::thread::JoinHandle<()>>,
    /// Signal to stop the polling thread.
    stop: Arc<std::sync::atomic::AtomicBool>,
}

impl SensorPoller {
    /// Create a new poller (does not start it yet).
    pub fn new(nexus_url: &str, interval_secs: u64) -> Self {
        Self {
            nexus_url: nexus_url.to_string(),
            interval: Duration::from_secs(interval_secs),
            latest: Arc::new(Mutex::new(SensorReading::empty())),
            handle: None,
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Get a clone-able handle to the latest reading.
    pub fn latest_handle(&self) -> Arc<Mutex<SensorReading>> {
        Arc::clone(&self.latest)
    }

    /// Start the background polling thread.
    pub fn start(&mut self) {
        if self.handle.is_some() {
            tracing::warn!("sensor poller already running");
            return;
        }

        let nexus_url = self.nexus_url.clone();
        let interval = self.interval;
        let latest = Arc::clone(&self.latest);
        let stop = Arc::clone(&self.stop);

        tracing::info!(
            nexus_url = %nexus_url,
            interval_ms = interval.as_millis() as u64,
            "starting sensor poller"
        );

        let handle = std::thread::Builder::new()
            .name("sensor-poller".into())
            .spawn(move || {
                poll_loop(&nexus_url, interval, &latest, &stop);
            })
            .expect("failed to spawn sensor poller thread");

        self.handle = Some(handle);
    }

    /// Signal the poller to stop and wait for the thread to exit.
    pub fn stop(&mut self) {
        self.stop
            .store(true, std::sync::atomic::Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Get the latest cached sensor reading.
    #[allow(dead_code)]
    pub fn latest_reading(&self) -> SensorReading {
        self.latest
            .lock()
            .expect("sensor reading lock poisoned")
            .clone()
    }
}

impl Drop for SensorPoller {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The actual polling loop that runs in a background thread.
fn poll_loop(
    nexus_url: &str,
    interval: Duration,
    latest: &Arc<Mutex<SensorReading>>,
    stop: &std::sync::atomic::AtomicBool,
) {
    let mut consecutive_failures: u32 = 0;
    const MAX_BACKOFF_SECS: u64 = 30;

    loop {
        if stop.load(std::sync::atomic::Ordering::Acquire) {
            tracing::info!("sensor poller shutting down");
            break;
        }

        match poll_sensors(nexus_url) {
            Ok(reading) => {
                if consecutive_failures > 0 {
                    tracing::info!(
                        previous_failures = consecutive_failures,
                        "sensor polling recovered"
                    );
                }
                consecutive_failures = 0;

                if let Ok(mut guard) = latest.lock() {
                    *guard = reading;
                }
            }
            Err(e) => {
                consecutive_failures += 1;
                tracing::warn!(
                    error = %e,
                    consecutive_failures,
                    "sensor poll failed"
                );
            }
        }

        // Apply exponential backoff on repeated failures, capped at MAX_BACKOFF_SECS.
        let sleep_duration = if consecutive_failures > 0 {
            let backoff_secs =
                (interval.as_secs() * 2u64.pow(consecutive_failures.min(5)))
                    .min(MAX_BACKOFF_SECS);
            Duration::from_secs(backoff_secs)
        } else {
            interval
        };

        // Sleep in small increments so we can respond to stop signals promptly.
        let deadline = std::time::Instant::now() + sleep_duration;
        while std::time::Instant::now() < deadline {
            if stop.load(std::sync::atomic::Ordering::Acquire) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_reading_empty() {
        let reading = SensorReading::empty();
        assert!(reading.values.is_empty());
        assert!(reading.get("nonexistent").is_none());
    }

    #[test]
    fn test_sensor_reading_get() {
        let mut values = HashMap::new();
        values.insert("temperature".into(), 42.5);
        values.insert("pressure".into(), 1013.25);
        let reading = SensorReading {
            timestamp: Utc::now(),
            values,
        };
        assert!((reading.get("temperature").unwrap() - 42.5).abs() < f32::EPSILON);
        assert!(reading.get("missing").is_none());
    }

    #[test]
    fn test_nexus_response_parsing() {
        let json = r#"{
            "timestamp": "2025-01-15T12:00:00Z",
            "sensors": {"temp": 22.5, "humidity": 65.0}
        }"#;
        let parsed: NexusSensorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.sensors.len(), 2);
        assert!(parsed.timestamp.is_some());
    }

    #[test]
    fn test_nexus_response_values_field() {
        // Some firmware versions use "values" instead of "sensors".
        let json = r#"{
            "values": {"vibration": 0.05, "rpm": 1750.0}
        }"#;
        let parsed: NexusSensorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.values.len(), 2);
        assert!(parsed.sensors.is_empty());
    }

    #[test]
    fn test_poller_creation() {
        let poller = SensorPoller::new("http://127.0.0.1:6100", 1);
        let reading = poller.latest_reading();
        assert!(reading.values.is_empty());
    }
}
