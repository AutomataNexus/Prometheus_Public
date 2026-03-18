// ============================================================================
// File: config.rs
// Description: Per-unit edge daemon configuration for model, sensors, and normalization
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Per-unit configuration for the Prometheus edge inference daemon.
//!
//! Each Raspberry Pi controller has its own configuration specifying which model
//! to load, how to reach the NexusEdge hardware daemon, which sensor features to
//! use, and how to normalize inputs before inference.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level edge daemon configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    /// Filesystem path to the `.axonml` model file.
    pub model_path: String,

    /// Base URL of the NexusEdge hardware daemon (e.g. `http://127.0.0.1:6100`).
    pub nexus_url: String,

    /// How often (in seconds) to poll sensors from NexusEdge.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,

    /// How often (in seconds) to run inference on the latest sensor data.
    #[serde(default = "default_inference_interval")]
    pub inference_interval_secs: u64,

    /// HTTP port for the edge daemon API.
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// Ordered list of sensor feature names to extract from each reading.
    /// These are fed to the model in this exact order.
    pub features: Vec<String>,

    /// Per-feature normalization ranges: `feature_name -> (min, max)`.
    /// Input values are scaled to [0, 1] using `(value - min) / (max - min)`.
    #[serde(default)]
    pub normalization: HashMap<String, NormRange>,

    /// Optional human-readable unit identifier (e.g. "reactor-7-pump-3").
    #[serde(default)]
    pub unit_id: String,

    /// Anomaly score threshold above which a reading is flagged.
    #[serde(default = "default_anomaly_threshold")]
    pub anomaly_threshold: f32,
}

/// Min/max range used to normalize a single feature to [0, 1].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormRange {
    pub min: f32,
    pub max: f32,
}

impl NormRange {
    /// Normalize a raw value into [0, 1], clamping to bounds.
    pub fn normalize(&self, value: f32) -> f32 {
        let range = self.max - self.min;
        if range.abs() < f32::EPSILON {
            return 0.0;
        }
        ((value - self.min) / range).clamp(0.0, 1.0)
    }
}

// ── Defaults ──────────────────────────────────────────────────────────────────

fn default_poll_interval() -> u64 {
    1
}

fn default_inference_interval() -> u64 {
    30
}

fn default_http_port() -> u16 {
    6200
}

fn default_anomaly_threshold() -> f32 {
    0.75
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            model_path: String::from("model.axonml"),
            nexus_url: String::from("http://127.0.0.1:6100"),
            poll_interval_secs: default_poll_interval(),
            inference_interval_secs: default_inference_interval(),
            http_port: default_http_port(),
            features: Vec::new(),
            normalization: HashMap::new(),
            unit_id: String::new(),
            anomaly_threshold: default_anomaly_threshold(),
        }
    }
}

impl EdgeConfig {
    /// Load configuration from a file. Supports both TOML (`.toml`) and JSON
    /// (`.json`) based on the file extension. Falls back to JSON if the
    /// extension is unrecognized.
    pub fn load(path: &str) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(path.to_string(), e))?;

        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json");

        match ext {
            "toml" => Self::from_toml(&contents),
            _ => Self::from_json(&contents),
        }
    }

    /// Deserialize from a TOML string.
    ///
    /// We do a lightweight TOML parse without pulling in the full `toml` crate:
    /// the config is simple enough to round-trip through our own mini-parser,
    /// but for robustness we fall back to JSON-style serde.
    fn from_toml(contents: &str) -> Result<Self, ConfigError> {
        // Minimal TOML parsing: convert to JSON via line-by-line key=value,
        // handling strings, numbers, booleans, and arrays/tables.
        // For production correctness we embed a tiny TOML-to-JSON converter.
        let json = toml_to_json(contents)?;
        serde_json::from_str(&json).map_err(ConfigError::Json)
    }

    /// Deserialize from a JSON string.
    fn from_json(contents: &str) -> Result<Self, ConfigError> {
        serde_json::from_str(contents).map_err(ConfigError::Json)
    }

    /// Extract the ordered feature vector from a sensor reading's value map,
    /// applying normalization where configured.
    pub fn extract_features(&self, values: &HashMap<String, f32>) -> Vec<f32> {
        self.features
            .iter()
            .map(|name| {
                let raw = values.get(name).copied().unwrap_or(0.0);
                match self.normalization.get(name) {
                    Some(range) => range.normalize(raw),
                    None => raw,
                }
            })
            .collect()
    }
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors that can occur during configuration loading.
#[derive(Debug)]
pub enum ConfigError {
    Io(String, std::io::Error),
    Json(serde_json::Error),
    Toml(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(path, e) => write!(f, "failed to read config file '{}': {}", path, e),
            ConfigError::Json(e) => write!(f, "invalid config JSON: {}", e),
            ConfigError::Toml(msg) => write!(f, "invalid config TOML: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

// ── Minimal TOML-to-JSON converter ────────────────────────────────────────────
//
// Handles the subset of TOML used by edge configs: top-level keys, inline
// tables (`[section]`), string/number/bool values, arrays of strings/numbers.
// This avoids adding a `toml` crate dependency on resource-constrained Pi
// devices.

fn toml_to_json(toml: &str) -> Result<String, ConfigError> {
    use std::fmt::Write;

    let mut out = String::from("{\n");
    let mut current_table: Option<String> = None;
    let mut table_entries: Vec<String> = Vec::new();
    let mut top_entries: Vec<String> = Vec::new();
    let mut sub_tables: Vec<String> = Vec::new();

    for line in toml.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Table header: `[section]`
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Flush previous table.
            if let Some(ref table_name) = current_table {
                let body = table_entries.join(",\n");
                sub_tables.push(format!("  \"{}\": {{{}}}", table_name, body));
                table_entries.clear();
            }
            let name = trimmed.trim_matches(|c| c == '[' || c == ']').trim();
            current_table = Some(name.to_string());
            continue;
        }

        // Key = value
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim().trim_matches('"');
            let value = value.trim();
            let json_value = toml_value_to_json(value)?;
            let entry = format!("  \"{}\": {}", key, json_value);

            if current_table.is_some() {
                table_entries.push(entry);
            } else {
                top_entries.push(entry);
            }
        }
    }

    // Flush last table.
    if let Some(ref table_name) = current_table {
        let body = table_entries.join(",\n");
        sub_tables.push(format!("  \"{}\": {{{}}}", table_name, body));
    }

    let mut all_entries = top_entries;
    all_entries.extend(sub_tables);
    let _ = write!(out, "{}", all_entries.join(",\n"));
    out.push_str("\n}");

    Ok(out)
}

fn toml_value_to_json(value: &str) -> Result<String, ConfigError> {
    let value = value.trim();

    // Quoted string
    if value.starts_with('"') && value.ends_with('"') {
        return Ok(value.to_string());
    }

    // Array
    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        let elements: Vec<String> = inner
            .split(',')
            .map(|e| {
                let e = e.trim();
                if e.starts_with('"') && e.ends_with('"') {
                    e.to_string()
                } else {
                    e.to_string()
                }
            })
            .filter(|e| !e.is_empty())
            .collect();
        return Ok(format!("[{}]", elements.join(", ")));
    }

    // Boolean
    if value == "true" || value == "false" {
        return Ok(value.to_string());
    }

    // Number (integer or float)
    if value.parse::<f64>().is_ok() {
        return Ok(value.to_string());
    }

    Err(ConfigError::Toml(format!("unsupported TOML value: {}", value)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_norm_range() {
        let range = NormRange { min: 0.0, max: 100.0 };
        assert!((range.normalize(50.0) - 0.5).abs() < f32::EPSILON);
        assert!((range.normalize(-10.0) - 0.0).abs() < f32::EPSILON);
        assert!((range.normalize(200.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_default_config() {
        let cfg = EdgeConfig::default();
        assert_eq!(cfg.poll_interval_secs, 1);
        assert_eq!(cfg.inference_interval_secs, 30);
        assert_eq!(cfg.http_port, 6200);
        assert!((cfg.anomaly_threshold - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_json_round_trip() {
        let cfg = EdgeConfig {
            model_path: "test.axonml".into(),
            nexus_url: "http://localhost:6100".into(),
            features: vec!["temp".into(), "pressure".into()],
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let loaded: EdgeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.features, cfg.features);
    }

    #[test]
    fn test_extract_features_with_normalization() {
        let mut norm = HashMap::new();
        norm.insert("temp".into(), NormRange { min: 0.0, max: 100.0 });
        let cfg = EdgeConfig {
            features: vec!["temp".into(), "pressure".into()],
            normalization: norm,
            ..Default::default()
        };
        let mut values = HashMap::new();
        values.insert("temp".into(), 50.0);
        values.insert("pressure".into(), 3.5);
        let feats = cfg.extract_features(&values);
        assert!((feats[0] - 0.5).abs() < f32::EPSILON);
        assert!((feats[1] - 3.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toml_to_json() {
        let toml = r#"
model_path = "model.axonml"
nexus_url = "http://127.0.0.1:6100"
poll_interval_secs = 2
features = ["temp", "vibration"]
"#;
        let json = toml_to_json(toml).unwrap();
        let cfg: EdgeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.model_path, "model.axonml");
        assert_eq!(cfg.poll_interval_secs, 2);
        assert_eq!(cfg.features, vec!["temp", "vibration"]);
    }
}
