// ============================================================================
// File: inference.rs
// Description: AxonML model loader and forward-pass inference engine for edge devices
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Model inference engine for edge deployment.
//!
//! Loads a serialized `.axonml` model file and runs forward-pass inference on
//! sensor feature vectors. The model file format is a flat binary with a
//! metadata header describing the network architecture followed by concatenated
//! f32 weight data.
//!
//! ## File format (`.axonml`)
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │ Magic bytes: b"AXON" (4 bytes)                            │
//! │ Version: u32 LE (4 bytes)                                 │
//! │ Header length: u32 LE (4 bytes)                           │
//! │ Header JSON: UTF-8 (header_length bytes)                  │
//! │ Weights: f32 LE values (remainder of file)                │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! The header JSON contains a `ModelMetadata` describing layer dimensions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Read;

// ── Model metadata ───────────────────────────────────────────────────────────

/// Describes the architecture serialized inside an `.axonml` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Human-readable model name.
    pub name: String,
    /// Model version string.
    pub version: String,
    /// Ordered list of layer descriptors (input_dim, output_dim, activation).
    pub layers: Vec<LayerDescriptor>,
    /// Total number of f32 weight values expected.
    pub total_weights: usize,
    /// Optional description.
    #[serde(default)]
    pub description: String,
}

/// A single dense (fully-connected) layer descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDescriptor {
    /// Number of input features.
    pub input_dim: usize,
    /// Number of output features.
    pub output_dim: usize,
    /// Activation function name: "relu", "sigmoid", "tanh", "none".
    #[serde(default = "default_activation")]
    pub activation: String,
    /// Whether this layer has a bias vector.
    #[serde(default = "default_true")]
    pub has_bias: bool,
}

fn default_activation() -> String {
    "relu".into()
}

fn default_true() -> bool {
    true
}

impl LayerDescriptor {
    /// Number of weight parameters in this layer (weights + optional bias).
    #[allow(dead_code)]
    pub fn param_count(&self) -> usize {
        let weights = self.input_dim * self.output_dim;
        let bias = if self.has_bias { self.output_dim } else { 0 };
        weights + bias
    }
}

// ── Runtime layer ─────────────────────────────────────────────────────────────

/// A loaded dense layer ready for inference.
#[derive(Debug, Clone)]
struct DenseLayer {
    /// Weight matrix stored row-major: shape (output_dim, input_dim).
    weights: Vec<f32>,
    /// Optional bias vector: shape (output_dim,).
    bias: Option<Vec<f32>>,
    input_dim: usize,
    output_dim: usize,
    activation: Activation,
}

#[derive(Debug, Clone, Copy)]
enum Activation {
    None,
    Relu,
    Sigmoid,
    Tanh,
}

impl Activation {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "relu" => Activation::Relu,
            "sigmoid" => Activation::Sigmoid,
            "tanh" => Activation::Tanh,
            _ => Activation::None,
        }
    }

    #[inline]
    fn apply(self, x: f32) -> f32 {
        match self {
            Activation::None => x,
            Activation::Relu => x.max(0.0),
            Activation::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            Activation::Tanh => x.tanh(),
        }
    }
}

impl DenseLayer {
    /// Run a forward pass: output = activation(W * input + b).
    fn forward(&self, input: &[f32]) -> Vec<f32> {
        debug_assert_eq!(
            input.len(),
            self.input_dim,
            "layer input dimension mismatch: expected {}, got {}",
            self.input_dim,
            input.len(),
        );

        let mut output = Vec::with_capacity(self.output_dim);
        for o in 0..self.output_dim {
            let mut sum = 0.0_f32;
            let row_offset = o * self.input_dim;
            for i in 0..self.input_dim {
                sum += self.weights[row_offset + i] * input[i];
            }
            if let Some(ref bias) = self.bias {
                sum += bias[o];
            }
            output.push(self.activation.apply(sum));
        }
        output
    }
}

// ── Inference engine ──────────────────────────────────────────────────────────

/// Holds a loaded model and executes inference.
pub struct InferenceEngine {
    metadata: ModelMetadata,
    layers: Vec<DenseLayer>,
    /// Number of input features the model expects.
    input_dim: usize,
    /// Number of outputs the model produces.
    output_dim: usize,
    /// Monotonically increasing prediction counter.
    prediction_count: std::sync::atomic::AtomicU64,
}

impl std::fmt::Debug for InferenceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferenceEngine")
            .field("model", &self.metadata.name)
            .field("version", &self.metadata.version)
            .field("layers", &self.layers.len())
            .field("input_dim", &self.input_dim)
            .field("output_dim", &self.output_dim)
            .finish()
    }
}

/// The result of a single inference run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    /// Raw model output predictions.
    pub predictions: Vec<f32>,
    /// Anomaly score derived from predictions (0.0 = normal, 1.0 = critical).
    pub anomaly_score: f32,
    /// Equipment health score (1.0 = perfect, 0.0 = failing).
    pub health_score: f32,
    /// UTC timestamp of this prediction.
    pub timestamp: DateTime<Utc>,
    /// How many predictions have been run since engine initialization.
    pub prediction_number: u64,
}

/// Errors that can occur during model loading or inference.
#[derive(Debug)]
pub enum InferenceError {
    Io(std::io::Error),
    InvalidMagic,
    UnsupportedVersion(u32),
    InvalidHeader(String),
    WeightCountMismatch { expected: usize, actual: usize },
    DimensionMismatch { expected: usize, actual: usize },
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferenceError::Io(e) => write!(f, "I/O error: {}", e),
            InferenceError::InvalidMagic => {
                write!(f, "invalid model file: missing AXON magic bytes")
            }
            InferenceError::UnsupportedVersion(v) => {
                write!(f, "unsupported model version: {}", v)
            }
            InferenceError::InvalidHeader(msg) => {
                write!(f, "invalid model header: {}", msg)
            }
            InferenceError::WeightCountMismatch { expected, actual } => {
                write!(
                    f,
                    "weight count mismatch: header declares {} but file contains {}",
                    expected, actual
                )
            }
            InferenceError::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "input dimension mismatch: model expects {} features but got {}",
                    expected, actual
                )
            }
        }
    }
}

impl std::error::Error for InferenceError {}

impl From<std::io::Error> for InferenceError {
    fn from(e: std::io::Error) -> Self {
        InferenceError::Io(e)
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Load a model from an `.axonml` file on disk.
pub fn load_model(path: &str) -> Result<InferenceEngine, InferenceError> {
    tracing::info!(path = %path, "loading model");

    let data = std::fs::read(path)?;
    load_model_from_bytes(&data)
}

/// Load a model from raw bytes (useful for embedded / in-memory models).
pub fn load_model_from_bytes(data: &[u8]) -> Result<InferenceEngine, InferenceError> {
    // ── Parse header ──────────────────────────────────────────────────────

    if data.len() < 12 {
        return Err(InferenceError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "file too short for header",
        )));
    }

    // Magic bytes
    if &data[0..4] != b"AXON" {
        return Err(InferenceError::InvalidMagic);
    }

    // Version
    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if version == 0 || version > 2 {
        return Err(InferenceError::UnsupportedVersion(version));
    }

    // Header length
    let header_len =
        u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;

    if data.len() < 12 + header_len {
        return Err(InferenceError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "file truncated in header region",
        )));
    }

    let header_json = std::str::from_utf8(&data[12..12 + header_len])
        .map_err(|e| InferenceError::InvalidHeader(e.to_string()))?;

    let metadata: ModelMetadata = serde_json::from_str(header_json)
        .map_err(|e| InferenceError::InvalidHeader(e.to_string()))?;

    // ── Parse weights ─────────────────────────────────────────────────────

    let weight_bytes = &data[12 + header_len..];
    if weight_bytes.len() % 4 != 0 {
        return Err(InferenceError::WeightCountMismatch {
            expected: metadata.total_weights,
            actual: weight_bytes.len() / 4,
        });
    }

    let weight_count = weight_bytes.len() / 4;
    if weight_count != metadata.total_weights {
        return Err(InferenceError::WeightCountMismatch {
            expected: metadata.total_weights,
            actual: weight_count,
        });
    }

    let mut weights = Vec::with_capacity(weight_count);
    let mut cursor = std::io::Cursor::new(weight_bytes);
    let mut buf = [0u8; 4];
    for _ in 0..weight_count {
        cursor.read_exact(&mut buf)?;
        weights.push(f32::from_le_bytes(buf));
    }

    // ── Build layers ──────────────────────────────────────────────────────

    let mut layers = Vec::with_capacity(metadata.layers.len());
    let mut offset = 0;

    for desc in &metadata.layers {
        let w_count = desc.input_dim * desc.output_dim;
        let w = weights[offset..offset + w_count].to_vec();
        offset += w_count;

        let bias = if desc.has_bias {
            let b = weights[offset..offset + desc.output_dim].to_vec();
            offset += desc.output_dim;
            Some(b)
        } else {
            None
        };

        layers.push(DenseLayer {
            weights: w,
            bias,
            input_dim: desc.input_dim,
            output_dim: desc.output_dim,
            activation: Activation::from_str(&desc.activation),
        });
    }

    let input_dim = metadata
        .layers
        .first()
        .map(|l| l.input_dim)
        .unwrap_or(0);
    let output_dim = metadata
        .layers
        .last()
        .map(|l| l.output_dim)
        .unwrap_or(0);

    tracing::info!(
        name = %metadata.name,
        version = %metadata.version,
        layers = layers.len(),
        input_dim,
        output_dim,
        total_params = weight_count,
        "model loaded successfully"
    );

    Ok(InferenceEngine {
        metadata,
        layers,
        input_dim,
        output_dim,
        prediction_count: std::sync::atomic::AtomicU64::new(0),
    })
}

impl InferenceEngine {
    /// Run inference on a feature vector.
    ///
    /// The input length must match the model's expected input dimension.
    /// Returns a `PredictionResult` with raw outputs, anomaly score, and
    /// health score.
    pub fn predict(&self, input: &[f32]) -> Result<PredictionResult, InferenceError> {
        if input.len() != self.input_dim {
            return Err(InferenceError::DimensionMismatch {
                expected: self.input_dim,
                actual: input.len(),
            });
        }

        // Forward pass through all layers.
        let mut current = input.to_vec();
        for layer in &self.layers {
            current = layer.forward(&current);
        }

        // Derive anomaly and health scores from the output vector.
        let anomaly_score = compute_anomaly_score(&current);
        let health_score = 1.0 - anomaly_score;

        let count = self
            .prediction_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;

        Ok(PredictionResult {
            predictions: current,
            anomaly_score,
            health_score,
            timestamp: Utc::now(),
            prediction_number: count,
        })
    }

    /// Number of input features the model expects.
    pub fn input_dim(&self) -> usize {
        self.input_dim
    }

    /// Number of outputs the model produces.
    pub fn output_dim(&self) -> usize {
        self.output_dim
    }

    /// Model metadata.
    pub fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }

    /// Total number of predictions run so far.
    pub fn prediction_count(&self) -> u64 {
        self.prediction_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

// ── Scoring helpers ───────────────────────────────────────────────────────────

/// Compute an anomaly score from the raw model output.
///
/// Strategy: if the model has a single output, treat it as a direct anomaly
/// probability (clamped to [0, 1]). If the model has multiple outputs, use the
/// mean-squared magnitude as an anomaly indicator — higher magnitude outputs
/// signal greater deviation from normal operating conditions.
fn compute_anomaly_score(outputs: &[f32]) -> f32 {
    if outputs.is_empty() {
        return 0.0;
    }

    if outputs.len() == 1 {
        // Single-output model: direct anomaly probability.
        return outputs[0].clamp(0.0, 1.0);
    }

    // Multi-output model: root-mean-square as anomaly signal, clamped.
    let sum_sq: f32 = outputs.iter().map(|v| v * v).sum();
    let rms = (sum_sq / outputs.len() as f32).sqrt();
    rms.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tiny test model file in memory.
    fn build_test_model(input_dim: usize, hidden_dim: usize, output_dim: usize) -> Vec<u8> {
        let layers = vec![
            LayerDescriptor {
                input_dim,
                output_dim: hidden_dim,
                activation: "relu".into(),
                has_bias: true,
            },
            LayerDescriptor {
                input_dim: hidden_dim,
                output_dim,
                activation: "sigmoid".into(),
                has_bias: true,
            },
        ];

        let total_weights: usize = layers.iter().map(|l| l.param_count()).sum();

        let metadata = ModelMetadata {
            name: "test-model".into(),
            version: "0.1.0".into(),
            layers,
            total_weights,
            description: "unit test model".into(),
        };

        let header = serde_json::to_string(&metadata).unwrap();
        let header_bytes = header.as_bytes();

        let mut data = Vec::new();
        data.extend_from_slice(b"AXON");
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(header_bytes);

        // Fill weights with small values.
        for i in 0..total_weights {
            let w = (i as f32) * 0.01;
            data.extend_from_slice(&w.to_le_bytes());
        }

        data
    }

    #[test]
    fn test_load_and_predict() {
        let model_data = build_test_model(4, 8, 1);
        let engine = load_model_from_bytes(&model_data).unwrap();
        assert_eq!(engine.input_dim(), 4);
        assert_eq!(engine.output_dim(), 1);

        let input = vec![0.5, 0.3, 0.7, 0.1];
        let result = engine.predict(&input).unwrap();
        assert_eq!(result.predictions.len(), 1);
        assert!(result.anomaly_score >= 0.0 && result.anomaly_score <= 1.0);
        assert!(result.health_score >= 0.0 && result.health_score <= 1.0);
        assert_eq!(result.prediction_number, 1);
    }

    #[test]
    fn test_dimension_mismatch() {
        let model_data = build_test_model(4, 8, 1);
        let engine = load_model_from_bytes(&model_data).unwrap();

        let bad_input = vec![0.5, 0.3]; // wrong size
        assert!(engine.predict(&bad_input).is_err());
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = build_test_model(2, 4, 1);
        data[0] = b'X'; // corrupt magic
        assert!(load_model_from_bytes(&data).is_err());
    }

    #[test]
    fn test_anomaly_score_single_output() {
        assert!((compute_anomaly_score(&[0.3]) - 0.3).abs() < f32::EPSILON);
        assert!((compute_anomaly_score(&[1.5]) - 1.0).abs() < f32::EPSILON);
        assert!((compute_anomaly_score(&[-0.2]) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_prediction_counter() {
        let model_data = build_test_model(2, 4, 1);
        let engine = load_model_from_bytes(&model_data).unwrap();
        let input = vec![0.5, 0.3];
        let _ = engine.predict(&input).unwrap();
        let _ = engine.predict(&input).unwrap();
        assert_eq!(engine.prediction_count(), 2);
    }
}
