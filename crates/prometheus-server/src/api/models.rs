// ============================================================================
// File: models.rs
// Description: ML model CRUD, ONNX/HEF conversion, and model file download endpoints
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{
    extract::{Path, State, Query},
    Extension, Json,
};
use serde::Deserialize;
use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub async fn list_models(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let docs = state.aegis_list_docs("models").await?;
    let visible: Vec<serde_json::Value> = if auth.is_admin() {
        docs
    } else {
        docs.into_iter().filter(|d| {
            d.get("created_by").and_then(|v| v.as_str()) == Some(&auth.user_id)
        }).collect()
    };

    // Backfill missing param counts from .axonml files
    let mut result = Vec::with_capacity(visible.len());
    for mut doc in visible {
        let params = doc.get("parameters").and_then(|v| v.as_u64()).unwrap_or(0);
        if params == 0 {
            if let Some(file_path) = doc.get("file_path").and_then(|v| v.as_str()) {
                if let Ok(weights) = prometheus_training::export::load_model(file_path) {
                    if let Some(obj) = doc.as_object_mut() {
                        obj.insert("parameters".into(), serde_json::json!(weights.weights.len()));
                    }
                }
            }
        }
        result.push(doc);
    }
    Ok(Json(result))
}

pub async fn get_model(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let mut doc = state.aegis_get_doc("models", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    // Backfill missing metadata from .axonml file for older models
    let needs_backfill = doc.get("input_features").and_then(|v| v.as_u64()).unwrap_or(0) == 0;
    if needs_backfill {
        if let Some(file_path) = doc.get("file_path").and_then(|v| v.as_str()) {
            if let Ok(weights) = prometheus_training::export::load_model(file_path) {
                if let Some(obj) = doc.as_object_mut() {
                    obj.insert("input_features".into(), serde_json::json!(weights.input_features));
                    obj.insert("hidden_dim".into(), serde_json::json!(weights.hyperparameters.hidden_dim));
                    obj.insert("num_layers".into(), serde_json::json!(weights.hyperparameters.num_layers));
                    obj.insert("sequence_length".into(), serde_json::json!(weights.hyperparameters.sequence_length));
                    obj.insert("bottleneck_dim".into(), serde_json::json!(weights.hyperparameters.hidden_dim / 2));
                    obj.insert("batch_size".into(), serde_json::json!(weights.hyperparameters.batch_size));
                    let param_count = weights.weights.len();
                    obj.insert("parameters".into(), serde_json::json!(param_count));
                    // Persist the backfill
                    let _ = state.aegis_update_doc("models", &id, serde_json::json!({
                        "input_features": weights.input_features,
                        "hidden_dim": weights.hyperparameters.hidden_dim,
                        "num_layers": weights.hyperparameters.num_layers,
                        "sequence_length": weights.hyperparameters.sequence_length,
                        "bottleneck_dim": weights.hyperparameters.hidden_dim / 2,
                        "batch_size": weights.hyperparameters.batch_size,
                        "parameters": param_count,
                    })).await;
                }
            }
        }
    }

    Ok(Json(doc))
}

/// Rename a model.
pub async fn rename_model(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("models", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    let new_name = body.get("name").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("name required".into()))?;
    let _ = state.aegis_update_doc("models", &id, serde_json::json!({ "name": new_name })).await;
    Ok(Json(serde_json::json!({ "id": id, "name": new_name })))
}

#[allow(dead_code)]
pub async fn download_model(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<axum::response::Response> {
    let doc = state.aegis_get_doc("models", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    let file_path = doc
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::NotFound("Model file not found".into()))?;

    let data = tokio::fs::read(file_path)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read model: {e}")))?;

    let filename = file_path.split('/').last().unwrap_or("model.axonml");

    Ok(axum::response::Response::builder()
        .header("Content-Type", "application/octet-stream")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        )
        .body(axum::body::Body::from(data))
        .unwrap())
}

pub async fn delete_model(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let doc = state.aegis_get_doc("models", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    state.aegis_delete_doc("models", &id).await?;
    Ok(Json(serde_json::json!({ "deleted": id })))
}

#[derive(Deserialize)]
pub struct ConvertQuery {
    pub format: Option<String>,
}

/// Convert a model to ONNX or HEF format.
///
/// POST /api/v1/models/:id/convert?format=onnx
///
/// Shells out to the Python converter tool which reconstructs the model
/// in PyTorch and exports to standard ONNX protobuf format.
/// Returns JSON with the download path on success.
pub async fn convert_model(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Query(query): Query<ConvertQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let format = query.format.as_deref().unwrap_or("onnx");
    if format != "onnx" && format != "hef" {
        return Err(AppError::BadRequest(
            "format must be 'onnx' or 'hef'".into(),
        ));
    }

    let doc = state.aegis_get_doc("models", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    // For quantized models, use the source .axonml file for conversion
    let file_path = if doc.get("quantized").and_then(|v| v.as_bool()).unwrap_or(false) {
        if let Some(source_id) = doc.get("source_model_id").and_then(|v| v.as_str()) {
            let source_doc = state.aegis_get_doc("models", source_id).await?;
            source_doc.get("file_path").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::NotFound("Source model file not found".into()))?
                .to_string()
        } else {
            return Err(AppError::BadRequest("Quantized model has no source model. Convert the original model instead.".into()));
        }
    } else {
        doc.get("file_path").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::NotFound("Model file not found".into()))?
            .to_string()
    };

    // Verify the .axonml file exists
    if !tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
        return Err(AppError::NotFound(format!(
            "Model file does not exist: {file_path}"
        )));
    }

    // Determine output path
    let ext = format;
    let base = file_path.strip_suffix(".axonml").unwrap_or(&file_path).to_string();
    let output_path = format!("{base}.{ext}");

    // Run the Python converter — use Hailo DFC venv for HEF, standard venv for ONNX
    let converter_venv = if format == "hef" {
        std::env::var("HAILO_DFC_VENV")
            .unwrap_or_else(|_| "/opt/hailo-dfc-env".to_string())
    } else {
        std::env::var("CONVERTER_VENV")
            .unwrap_or_else(|_| "/opt/Prometheus/tools/converter-venv".to_string())
    };
    let python = format!("{converter_venv}/bin/python");
    let converter_script = std::env::var("CONVERTER_SCRIPT")
        .unwrap_or_else(|_| "/opt/Prometheus/tools/model_converter/convert.py".to_string());

    tracing::info!("Converting model {} to {}: {} -> {}", id, format, &file_path, &output_path);

    let result = tokio::process::Command::new(&python)
        .arg(&converter_script)
        .arg(&file_path)
        .arg("--format")
        .arg(format)
        .arg("--output")
        .arg(&output_path)
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to run converter: {e}")))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        tracing::error!("Converter failed: {stderr}\n{stdout}");

        // Check if output was actually created despite warnings/stderr noise
        if tokio::fs::try_exists(&output_path).await.unwrap_or(false) {
            let file_size = tokio::fs::metadata(&output_path).await.map(|m| m.len()).unwrap_or(0);
            if file_size > 0 {
                tracing::info!("Conversion succeeded despite stderr output ({} bytes)", file_size);
                return Ok(Json(serde_json::json!({
                    "status": "converted",
                    "format": format,
                    "output_path": output_path,
                    "file_size": file_size,
                    "file_size_bytes": file_size,
                })));
            }
        }

        // Check if HAR was saved as fallback (HEF compilation may fail but HAR is usable)
        let har_path = output_path.replace(".hef", ".har");
        if format == "hef" && tokio::fs::try_exists(&har_path).await.unwrap_or(false) {
            let har_size = tokio::fs::metadata(&har_path).await.map(|m| m.len()).unwrap_or(0);
            if har_size > 0 {
                tracing::info!("HEF compilation failed but HAR saved ({} bytes)", har_size);
                return Ok(Json(serde_json::json!({
                    "status": "partial",
                    "format": "har",
                    "message": "Hailo Archive (HAR) created. Full HEF compilation requires Hailo-8 hardware or updated DFC. The HAR file can be compiled with: hailo compiler model.har",
                    "output_path": har_path,
                    "file_size": har_size,
                    "file_size_bytes": har_size,
                })));
            }
        }

        // Extract the last meaningful error line for user display
        let last_error = stderr.lines().rev()
            .find(|l| !l.is_empty() && !l.starts_with(' ') && !l.contains("Warning") && !l.contains("DeprecationWarning"))
            .unwrap_or("Unknown conversion error");
        return Err(AppError::Internal(format!("Model conversion failed: {last_error}")));
    }

    let file_size = tokio::fs::metadata(&output_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    tracing::info!(
        "Conversion complete: {} -> {} ({} bytes)",
        file_path,
        output_path,
        file_size
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "format": format,
        "file_size": file_size,
        "download_url": format!("/api/v1/models/{id}/download?format={ext}"),
    })))
}

/// Download a model in the requested format.
///
/// GET /api/v1/models/:id/download?format=onnx
///
/// If format is "onnx" or "hef", serves the converted file (must have been
/// converted first via POST /convert). Default format is "axonml".
pub async fn download_model_format(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Query(query): Query<ConvertQuery>,
) -> AppResult<axum::response::Response> {
    let doc = state.aegis_get_doc("models", &id).await?;
    if !auth.is_admin() && doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    let base_path = doc
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::NotFound("Model file not found".into()))?;

    let format = query.format.as_deref().unwrap_or("axonml");

    let file_path = if format == "axonml" {
        base_path.to_string()
    } else {
        base_path
            .strip_suffix(".axonml")
            .unwrap_or(base_path)
            .to_string()
            + "."
            + format
    };

    let data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| AppError::NotFound(format!("Converted model not found: {e}. Run conversion first.")))?;

    let model_name = doc
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("model");
    let filename = format!("{model_name}.{format}");

    Ok(axum::response::Response::builder()
        .header("Content-Type", "application/octet-stream")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        )
        .body(axum::body::Body::from(data))
        .unwrap())
}

pub async fn compare_models(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let other_id = body
        .get("compare_with")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("compare_with required".into()))?;

    let model_a = state.aegis_get_doc("models", &id).await?;
    let model_b = state.aegis_get_doc("models", other_id).await?;

    Ok(Json(serde_json::json!({
        "model_a": model_a,
        "model_b": model_b,
    })))
}

/// Quantize a model using AxonML quant crate. Creates a new model entry.
pub async fn quantize_model(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let model_id = body.get("model_id").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("model_id required".into()))?
        .to_string();
    let quant_type_str = body.get("quant_type").and_then(|v| v.as_str())
        .unwrap_or("q8_0")
        .to_string();

    let model_doc = state.aegis_get_doc("models", &model_id).await?;
    if !auth.is_admin() && model_doc.get("created_by").and_then(|v| v.as_str()) != Some(&auth.user_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let file_path = model_doc.get("file_path").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::NotFound("Model file not found".into()))?
        .to_string();
    let architecture = model_doc.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let orig_name = model_doc.get("name").and_then(|v| v.as_str()).unwrap_or("Model").to_string();

    // Parse quant type
    let quant_type = axonml_quant::QuantType::from_str(&quant_type_str)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid quant_type: {quant_type_str}. Use q8_0, q4_0, q4_1, or f16")))?;

    // Load model weights
    let weights = prometheus_training::export::load_model(&file_path)
        .map_err(|e| AppError::Internal(format!("Failed to load model: {e}")))?;

    // Quantize the flat weight vector
    let original_size = weights.weights.len() * 4; // f32 = 4 bytes
    let weight_tensor = axonml_tensor::Tensor::from_vec(
        weights.weights.clone(),
        &[weights.weights.len()],
    ).map_err(|e| AppError::Internal(format!("Failed to create tensor: {e}")))?;
    let quantized_tensor = axonml_quant::quantize_tensor(&weight_tensor, quant_type)
        .map_err(|e| AppError::Internal(format!("Quantization failed: {e}")))?;
    let quantized_size = quantized_tensor.size_bytes();
    let compression_ratio = if quantized_size > 0 { original_size as f64 / quantized_size as f64 } else { 1.0 };

    // Build QuantizedModel and serialize to AXQT format
    let quant_model = axonml_quant::QuantizedModel {
        quantized_params: vec![quantized_tensor],
        quant_type,
        total_params: weights.weights.len(),
        total_bytes: quantized_size,
        original_bytes: original_size,
    };
    let quant_bytes = axonml_quant::serialize_quantized(&quant_model);

    // Save quantized model file
    let new_model_id = format!("mdl_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let quant_dir = format!("{}/models", state.config.data_dir);
    let _ = tokio::fs::create_dir_all(&quant_dir).await;
    let quant_path = format!("{}/{}.axqt", quant_dir, new_model_id);
    tokio::fs::write(&quant_path, &quant_bytes).await
        .map_err(|e| AppError::Internal(format!("Failed to save quantized model: {e}")))?;

    // Create new model doc
    let quant_label = quant_type_str.to_uppercase();
    let new_doc = serde_json::json!({
        "id": new_model_id,
        "name": format!("{} ({})", orig_name, quant_label),
        "architecture": architecture,
        "dataset_id": model_doc.get("dataset_id"),
        "source_model_id": model_id,
        "quantized": true,
        "quant_type": quant_type_str,
        "compression_ratio": compression_ratio,
        "original_size_bytes": original_size,
        "file_size_bytes": quant_bytes.len(),
        "file_path": quant_path,
        "parameters": weights.weights.len(),
        "input_features": weights.input_features,
        "hidden_dim": weights.hyperparameters.hidden_dim,
        "num_layers": weights.hyperparameters.num_layers,
        "sequence_length": weights.hyperparameters.sequence_length,
        "metrics": model_doc.get("metrics"),
        "status": "ready",
        "created_by": auth.user_id,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    state.aegis_create_doc("models", new_doc).await?;

    Ok(Json(serde_json::json!({
        "quantized_model_id": new_model_id,
        "quant_type": quant_type_str,
        "original_size_bytes": original_size,
        "quantized_size_bytes": quant_bytes.len(),
        "compression_ratio": compression_ratio,
    })))
}
