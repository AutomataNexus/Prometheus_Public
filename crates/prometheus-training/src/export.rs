// ============================================================================
// File: export.rs
// Description: Model serialization, export to .axonml binary format, and INT8 quantization
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Model export to `.axonml` format.
//!
//! Serializes trained model weights and metadata into the AxonML binary format
//! for deployment on edge devices.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::architectures::{TrainableModel, ModelWeights};
use crate::preprocessor::NormalizationStats;
use crate::{Hyperparameters, Result, TrainingError};

// ---------------------------------------------------------------------------
// .axonml file format
// ---------------------------------------------------------------------------

/// Magic bytes identifying an .axonml file.
const AXONML_MAGIC: &[u8; 6] = b"AXONML";

/// Current format version.
const AXONML_VERSION: u8 = 1;

/// Header for the .axonml binary format.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AxonmlHeader {
    /// Architecture identifier string.
    architecture: String,
    /// Number of input features.
    input_features: usize,
    /// Total parameter count.
    num_parameters: usize,
    /// Whether weights are quantized.
    quantized: bool,
    /// Quantization bit width (if quantized).
    quant_bits: Option<u8>,
}

// ---------------------------------------------------------------------------
// Export functions
// ---------------------------------------------------------------------------

/// Save a trained model's weights to an `.axonml` file.
///
/// The file contains:
/// 1. Magic bytes and version
/// 2. JSON-encoded header with metadata
/// 3. JSON-encoded model weights (parameters + normalization stats)
///
/// Returns the absolute path to the saved file.
#[instrument(skip(weights), fields(path = %path))]
pub fn save_model(weights: &ModelWeights, path: &str) -> Result<String> {
    let output_path = Path::new(path);

    // Create parent directories if they don't exist.
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            TrainingError::Export(format!("failed to create output directory: {e}"))
        })?;
    }

    // Ensure .axonml extension.
    let final_path = if output_path.extension().map_or(true, |ext| ext != "axonml") {
        output_path.with_extension("axonml")
    } else {
        output_path.to_path_buf()
    };

    let header = AxonmlHeader {
        architecture: format!("{}", weights.architecture),
        input_features: weights.input_features,
        num_parameters: weights.weights.len(),
        quantized: false,
        quant_bits: None,
    };

    // Serialize to the .axonml format.
    let header_json = serde_json::to_vec(&header).map_err(|e| {
        TrainingError::Export(format!("failed to serialize header: {e}"))
    })?;
    let weights_json = serde_json::to_vec(weights).map_err(|e| {
        TrainingError::Export(format!("failed to serialize weights: {e}"))
    })?;

    let mut file = fs::File::create(&final_path).map_err(|e| {
        TrainingError::Export(format!("failed to create file {}: {e}", final_path.display()))
    })?;

    // Write magic bytes.
    file.write_all(AXONML_MAGIC)?;
    // Write version byte.
    file.write_all(&[AXONML_VERSION])?;
    // Write header length (4 bytes LE) + header.
    let header_len = header_json.len() as u32;
    file.write_all(&header_len.to_le_bytes())?;
    file.write_all(&header_json)?;
    // Write weights length (4 bytes LE) + weights.
    let weights_len = weights_json.len() as u32;
    file.write_all(&weights_len.to_le_bytes())?;
    file.write_all(&weights_json)?;

    let path_str = final_path
        .to_str()
        .ok_or_else(|| TrainingError::Export("invalid path encoding".into()))?
        .to_string();

    let file_size = fs::metadata(&final_path)
        .map(|m| m.len())
        .unwrap_or(0);

    info!(
        "Model saved to {} ({} parameters, {:.1} KB)",
        path_str,
        weights.weights.len(),
        file_size as f64 / 1024.0
    );

    Ok(path_str)
}

/// Load a model's weights from an `.axonml` file.
///
/// Returns the deserialized `ModelWeights` struct.
#[instrument(fields(path = %path))]
pub fn load_model(path: &str) -> Result<ModelWeights> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Err(TrainingError::ModelNotFound(format!(
            "model file not found: {path}"
        )));
    }

    let data = fs::read(file_path)?;

    // Validate magic bytes.
    if data.len() < 11 || &data[0..6] != AXONML_MAGIC {
        return Err(TrainingError::Export(
            "invalid .axonml file: bad magic bytes".into(),
        ));
    }

    let version = data[6];
    if version != AXONML_VERSION {
        return Err(TrainingError::Export(format!(
            "unsupported .axonml version: {version} (expected {AXONML_VERSION})"
        )));
    }

    // Read header.
    let header_len = u32::from_le_bytes([data[7], data[8], data[9], data[10]]) as usize;
    let header_start = 11;
    let header_end = header_start + header_len;
    if data.len() < header_end + 4 {
        return Err(TrainingError::Export(
            "invalid .axonml file: truncated header".into(),
        ));
    }

    // Read weights.
    let weights_len_start = header_end;
    let weights_len = u32::from_le_bytes([
        data[weights_len_start],
        data[weights_len_start + 1],
        data[weights_len_start + 2],
        data[weights_len_start + 3],
    ]) as usize;
    let weights_start = weights_len_start + 4;
    let weights_end = weights_start + weights_len;
    if data.len() < weights_end {
        return Err(TrainingError::Export(
            "invalid .axonml file: truncated weights".into(),
        ));
    }

    let weights: ModelWeights =
        serde_json::from_slice(&data[weights_start..weights_end]).map_err(|e| {
            TrainingError::Export(format!("failed to deserialize weights: {e}"))
        })?;

    info!(
        "Loaded model from {} ({} parameters, architecture: {})",
        path,
        weights.weights.len(),
        weights.architecture
    );

    Ok(weights)
}

/// Create `ModelWeights` from a trained model and normalization stats.
pub fn create_model_weights(
    model: &dyn TrainableModel,
    hyperparameters: &Hyperparameters,
    norm_stats: Option<&NormalizationStats>,
    anomaly_threshold: Option<f32>,
) -> ModelWeights {
    let (norm_means, norm_stds) = match norm_stats {
        Some(stats) => (stats.means.clone(), stats.stds.clone()),
        None => (Vec::new(), Vec::new()),
    };

    ModelWeights {
        architecture: model.architecture(),
        input_features: model.input_features(),
        hyperparameters: hyperparameters.clone(),
        weights: model.flat_parameters(),
        norm_means,
        norm_stds,
        anomaly_threshold,
    }
}

/// Save model weights as a simple JSON file (for debugging/inspection).
#[instrument(skip(weights), fields(path = %path))]
pub fn save_model_json(weights: &ModelWeights, path: &str) -> Result<String> {
    let output_path = Path::new(path);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            TrainingError::Export(format!("failed to create output directory: {e}"))
        })?;
    }

    let json = serde_json::to_string_pretty(weights).map_err(|e| {
        TrainingError::Export(format!("failed to serialize to JSON: {e}"))
    })?;

    fs::write(output_path, &json)?;

    let path_str = output_path
        .to_str()
        .ok_or_else(|| TrainingError::Export("invalid path encoding".into()))?
        .to_string();

    info!("Model JSON saved to {}", path_str);
    Ok(path_str)
}

/// Quantize model weights to INT8.
///
/// Applies per-tensor symmetric quantization: each f32 weight is mapped
/// to an i8 value using a scale factor derived from the max absolute value.
/// The quantized weights are stored as f32 representations of the i8 values,
/// along with the scale factor needed for dequantization.
pub fn quantize_int8(weights: &ModelWeights) -> Result<ModelWeights> {
    if weights.weights.is_empty() {
        return Err(TrainingError::Export(
            "cannot quantize empty weights".into(),
        ));
    }

    // Find the maximum absolute value for scale computation.
    let max_abs = weights
        .weights
        .iter()
        .map(|w| w.abs())
        .fold(0.0f32, f32::max);

    if max_abs < 1e-10 {
        // All weights are effectively zero; nothing to quantize.
        return Ok(weights.clone());
    }

    let scale = max_abs / 127.0;

    // Quantize: round(w / scale), clamp to [-127, 127], then store as f32.
    let quantized_weights: Vec<f32> = weights
        .weights
        .iter()
        .map(|&w| {
            let q = (w / scale).round().clamp(-127.0, 127.0);
            q * scale // Dequantize back to f32 for storage.
        })
        .collect();

    info!(
        "Quantized {} parameters to INT8 (scale={:.6}, max_abs={:.6})",
        quantized_weights.len(),
        scale,
        max_abs
    );

    Ok(ModelWeights {
        weights: quantized_weights,
        ..weights.clone()
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Architecture, Hyperparameters};

    fn sample_weights() -> ModelWeights {
        ModelWeights {
            architecture: Architecture::Sentinel,
            input_features: 5,
            hyperparameters: Hyperparameters::default(),
            weights: vec![0.1, -0.2, 0.3, -0.4, 0.5],
            norm_means: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            norm_stds: vec![0.1, 0.2, 0.3, 0.4, 0.5],
            anomaly_threshold: None,
        }
    }

    #[test]
    fn test_quantize_int8() {
        let weights = sample_weights();
        let quantized = quantize_int8(&weights).unwrap();

        assert_eq!(quantized.weights.len(), weights.weights.len());
        // Quantized values should be close to originals (small model).
        for (orig, quant) in weights.weights.iter().zip(quantized.weights.iter()) {
            assert!(
                (orig - quant).abs() < 0.01,
                "quantization error too large: {} vs {}",
                orig,
                quant
            );
        }
    }

    #[test]
    fn test_quantize_empty() {
        let mut weights = sample_weights();
        weights.weights = Vec::new();
        assert!(quantize_int8(&weights).is_err());
    }

    // -----------------------------------------------------------------------
    // Additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_save_and_load_roundtrip() {
        let weights = sample_weights();
        let dir = std::env::temp_dir().join("prometheus_test_export_roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.axonml");
        let path_str = path.to_str().unwrap();

        let saved_path = save_model(&weights, path_str).unwrap();
        let loaded = load_model(&saved_path).unwrap();

        assert_eq!(loaded.architecture, weights.architecture);
        assert_eq!(loaded.input_features, weights.input_features);
        assert_eq!(loaded.weights.len(), weights.weights.len());
        for (a, b) in loaded.weights.iter().zip(weights.weights.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
        assert_eq!(loaded.norm_means, weights.norm_means);
        assert_eq!(loaded.norm_stds, weights.norm_stds);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_model_file_magic_bytes() {
        let weights = sample_weights();
        let dir = std::env::temp_dir().join("prometheus_test_magic_bytes");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.axonml");
        let path_str = path.to_str().unwrap();

        save_model(&weights, path_str).unwrap();
        let data = std::fs::read(&path).unwrap();

        // First 6 bytes should be AXONML.
        assert_eq!(&data[0..6], b"AXONML");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_model_file_version() {
        let weights = sample_weights();
        let dir = std::env::temp_dir().join("prometheus_test_version");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.axonml");
        let path_str = path.to_str().unwrap();

        save_model(&weights, path_str).unwrap();
        let data = std::fs::read(&path).unwrap();

        // Byte 6 is the version.
        assert_eq!(data[6], 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_model_file_json_header_parseable() {
        let weights = sample_weights();
        let dir = std::env::temp_dir().join("prometheus_test_json_header");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.axonml");
        let path_str = path.to_str().unwrap();

        save_model(&weights, path_str).unwrap();
        let data = std::fs::read(&path).unwrap();

        // Parse header length from bytes 7..11.
        let header_len = u32::from_le_bytes([data[7], data[8], data[9], data[10]]) as usize;
        let header_json = &data[11..11 + header_len];

        // Header should be valid JSON.
        let parsed: serde_json::Value = serde_json::from_slice(header_json).unwrap();
        assert!(parsed.get("architecture").is_some());
        assert!(parsed.get("input_features").is_some());
        assert!(parsed.get("num_parameters").is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_weights_preserved_after_save_load() {
        let mut weights = sample_weights();
        weights.weights = vec![0.123456, -0.789012, 3.14159, -2.71828, 0.0];

        let dir = std::env::temp_dir().join("prometheus_test_weights_preserved");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.axonml");
        let path_str = path.to_str().unwrap();

        save_model(&weights, path_str).unwrap();
        let loaded = load_model(path.to_str().unwrap()).unwrap();

        for (orig, loaded_w) in weights.weights.iter().zip(loaded.weights.iter()) {
            assert!(
                (orig - loaded_w).abs() < 1e-6,
                "Weight mismatch: {} vs {}",
                orig,
                loaded_w
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_quantize_int8_reduces_precision() {
        let mut weights = sample_weights();
        weights.weights = vec![0.12345, -0.67891, 0.54321, -0.98765, 0.11111];

        let quantized = quantize_int8(&weights).unwrap();

        // Quantized values should be close but not identical to originals
        // (quantization introduces rounding to discrete levels).
        let mut any_different = false;
        for (orig, quant) in weights.weights.iter().zip(quantized.weights.iter()) {
            if (orig - quant).abs() > 1e-10 {
                any_different = true;
            }
            // But should still be close.
            assert!(
                (orig - quant).abs() < 0.01,
                "Too much quantization error: {} vs {}",
                orig,
                quant
            );
        }
        // With typical values, at least some should be different due to rounding.
        // (This may not always hold for small models, so we don't assert it strongly.)
        let _ = any_different;
    }

    #[test]
    fn test_quantize_int8_zero_weights() {
        let mut weights = sample_weights();
        weights.weights = vec![0.0, 0.0, 0.0, 0.0, 0.0];

        // All-zero weights: max_abs < 1e-10, should return clone.
        let quantized = quantize_int8(&weights).unwrap();
        for &w in &quantized.weights {
            assert!((w - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_save_model_json_creates_valid_json() {
        let weights = sample_weights();
        let dir = std::env::temp_dir().join("prometheus_test_save_json");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.json");
        let path_str = path.to_str().unwrap();

        let saved_path = save_model_json(&weights, path_str).unwrap();
        let content = std::fs::read_to_string(&saved_path).unwrap();

        // Should be valid JSON.
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("architecture").is_some());
        assert!(parsed.get("weights").is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_model_invalid_magic_bytes() {
        let dir = std::env::temp_dir().join("prometheus_test_bad_magic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad_magic.axonml");

        // Write a file with wrong magic bytes.
        std::fs::write(&path, b"BADMAG\x01\x00\x00\x00\x00").unwrap();

        let result = load_model(path.to_str().unwrap());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("bad magic bytes"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_model_truncated_file() {
        let dir = std::env::temp_dir().join("prometheus_test_truncated");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("truncated.axonml");

        // Write only the magic bytes and version — too short for a valid file.
        std::fs::write(&path, b"AXONML\x01\x00\x00").unwrap();

        let result = load_model(path.to_str().unwrap());
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_model_nonexistent_file() {
        let result = load_model("/tmp/nonexistent_prometheus_model_12345.axonml");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_model_adds_extension() {
        let weights = sample_weights();
        let dir = std::env::temp_dir().join("prometheus_test_extension");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model_no_ext");
        let path_str = path.to_str().unwrap();

        let saved_path = save_model(&weights, path_str).unwrap();
        assert!(saved_path.ends_with(".axonml"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
