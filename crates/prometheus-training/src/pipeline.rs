// ============================================================================
// File: pipeline.rs
// Description: Five-stage training pipeline orchestrating validate, preprocess, train, evaluate, and export
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! The 5-stage training pipeline: validate -> preprocess -> train -> evaluate -> export.
//!
//! Orchestrates the full model training lifecycle, sending progress events
//! through an optional `tokio::sync::mpsc::Sender<TrainingEvent>` channel
//! for real-time UI updates via WebSocket.

use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;
use tracing::{info, instrument, warn};

use crate::architectures::{self, TrainableModel};
use crate::cross_compile::{self, CrossCompileConfig};
use crate::export;
use crate::metrics::Metrics;
use crate::preprocessor::{self, Dataset, NormalizationStats};
use crate::{
    Architecture, Result, TrainingConfig, TrainingError, TrainingEvent,
    TrainingResult,
};

// ---------------------------------------------------------------------------
// Pipeline entry point
// ---------------------------------------------------------------------------

/// Run the full 5-stage training pipeline.
///
/// # Stages
/// 1. **Validate** — Check configuration and dataset accessibility
/// 2. **Preprocess** — Load CSV, normalize, split into train/val/test, create sequences
/// 3. **Train** — Build model, run training loop with validation
/// 4. **Evaluate** — Compute final metrics on the held-out test set
/// 5. **Export** — Save model to `.axonml` format, optionally quantize and cross-compile
///
/// # Arguments
/// - `config`: Training configuration specifying architecture, hyperparameters, paths
/// - `sender`: Optional channel for sending progress events to the UI
///
/// # Returns
/// A `TrainingResult` with final metrics, artifact paths, and training statistics.
#[instrument(skip_all, fields(run_id = %config.run_id, arch = %config.architecture))]
pub async fn run_pipeline(
    config: TrainingConfig,
    sender: Option<mpsc::Sender<TrainingEvent>>,
) -> Result<TrainingResult> {
    let pipeline_start = Instant::now();

    info!(
        "Starting training pipeline: run_id={}, architecture={}, dataset={}",
        config.run_id, config.architecture, config.dataset_path
    );

    // ------------------------------------------------------------------
    // Stage 1: Validate
    // ------------------------------------------------------------------
    send_event(
        &sender,
        TrainingEvent::StageChange {
            stage: "validate".into(),
            message: "Validating configuration and dataset".into(),
        },
    )
    .await;

    let validated_config = validate_stage(&config)?;
    info!("Stage 1/5 [Validate] — complete");

    // ------------------------------------------------------------------
    // Stage 2: Preprocess
    // ------------------------------------------------------------------
    send_event(
        &sender,
        TrainingEvent::StageChange {
            stage: "preprocess".into(),
            message: "Loading and preprocessing dataset".into(),
        },
    )
    .await;

    let (train_set, val_set, test_set, norm_stats, actual_features) =
        preprocess_stage(&validated_config).await?;
    info!(
        "Stage 2/5 [Preprocess] — train={}, val={}, test={}",
        train_set.num_samples, val_set.num_samples, test_set.num_samples
    );

    // Update config with actual feature count from dataset to prevent
    // dimension mismatches in model construction (e.g., LSTM reshape panic).
    let mut validated_config = validated_config;
    if validated_config.input_features != actual_features {
        info!(
            "Adjusting input_features {} → {} to match dataset",
            validated_config.input_features, actual_features
        );
        validated_config.input_features = actual_features;
    }

    // ------------------------------------------------------------------
    // Stage 3: Train
    // ------------------------------------------------------------------
    send_event(
        &sender,
        TrainingEvent::StageChange {
            stage: "train".into(),
            message: format!(
                "Training {} for {} epochs",
                config.architecture, config.hyperparameters.epochs
            ),
        },
    )
    .await;

    let (model, epochs_trained, final_train_loss, final_val_loss) =
        train_stage(&validated_config, &train_set, &val_set, &sender).await?;
    info!(
        "Stage 3/5 [Train] — epochs={}, train_loss={:.6}, val_loss={:.6}",
        epochs_trained, final_train_loss, final_val_loss
    );

    // ------------------------------------------------------------------
    // Stage 4: Evaluate
    // ------------------------------------------------------------------
    send_event(
        &sender,
        TrainingEvent::StageChange {
            stage: "evaluate".into(),
            message: "Evaluating model on test set".into(),
        },
    )
    .await;

    let test_metrics = evaluate_stage(model.as_ref(), &test_set, &validated_config)?;
    info!("Stage 4/5 [Evaluate] — {}", test_metrics);

    // ------------------------------------------------------------------
    // Stage 5: Export
    // ------------------------------------------------------------------
    send_event(
        &sender,
        TrainingEvent::StageChange {
            stage: "export".into(),
            message: "Exporting model artifacts".into(),
        },
    )
    .await;

    let (artifact_path, quantized_path, arm_binary_path) = export_stage(
        model.as_ref(),
        &validated_config,
        &norm_stats,
    )?;
    info!("Stage 5/5 [Export] — artifact={}", artifact_path);

    // ------------------------------------------------------------------
    // Assemble result
    // ------------------------------------------------------------------
    let training_duration = pipeline_start.elapsed();

    let result = TrainingResult {
        run_id: config.run_id.clone(),
        architecture: config.architecture,
        metrics: test_metrics.clone(),
        artifact_path,
        quantized_artifact_path: quantized_path,
        arm_binary_path,
        training_duration,
        epochs_trained,
        final_train_loss,
        final_val_loss,
        completed_at: Utc::now(),
    };

    // Send completion event.
    send_event(
        &sender,
        TrainingEvent::TrainingDone {
            result: result.clone(),
        },
    )
    .await;

    info!(
        "Pipeline complete: run_id={}, duration={:.1}s, test_f1={:.4}",
        config.run_id,
        training_duration.as_secs_f64(),
        test_metrics.f1
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// Stage 1: Validate
// ---------------------------------------------------------------------------

fn validate_stage(config: &TrainingConfig) -> Result<TrainingConfig> {
    config.validate()?;

    // Verify dataset path exists.
    let dataset_path = std::path::Path::new(&config.dataset_path);
    if !dataset_path.exists() {
        return Err(TrainingError::Validation(format!(
            "dataset path does not exist: {}",
            config.dataset_path
        )));
    }

    // Verify output directory can be created.
    let output_path = std::path::Path::new(&config.output_path);
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                TrainingError::Validation(format!(
                    "cannot create output directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
    }

    Ok(config.clone())
}

// ---------------------------------------------------------------------------
// Stage 2: Preprocess
// ---------------------------------------------------------------------------

async fn preprocess_stage(
    config: &TrainingConfig,
) -> Result<(Dataset, Dataset, Dataset, NormalizationStats, usize)> {
    // Load the CSV dataset.
    let dataset = preprocessor::load_csv(&config.dataset_path)?;

    let actual_features = dataset.num_features;
    if actual_features != config.input_features {
        warn!(
            "Dataset has {} features but config expects {}; using actual count",
            actual_features, config.input_features
        );
    }

    // Split and normalize.
    let (train, val, test) = preprocessor::split_and_normalize(
        dataset,
        config.train_split,
        config.val_split,
        config.test_split,
    )?;

    // Recompute stats from the training set (proper methodology).
    let train_norm_stats = NormalizationStats::from_data(&train.data)?;

    Ok((train, val, test, train_norm_stats, actual_features))
}

// ---------------------------------------------------------------------------
// Stage 3: Train
// ---------------------------------------------------------------------------

async fn train_stage(
    config: &TrainingConfig,
    train_set: &Dataset,
    val_set: &Dataset,
    sender: &Option<mpsc::Sender<TrainingEvent>>,
) -> Result<(Box<dyn TrainableModel>, usize, f32, f32)> {
    let hp = &config.hyperparameters;

    // Build model.
    let model = architectures::build_model(
        config.architecture,
        config.input_features,
        hp,
    )?;

    // Resume from pre-trained weights if specified.
    if let Some(ref resume_path) = config.resume_from {
        info!("Resuming from pre-trained weights: {resume_path}");
        let saved = crate::export::load_model(resume_path)?;
        let params = model.parameters();
        let mut offset = 0;
        for param in &params {
            let numel = param.numel();
            if offset + numel <= saved.weights.len() {
                let param_data = &saved.weights[offset..offset + numel];
                let tensor = axonml_tensor::Tensor::from_vec(
                    param_data.to_vec(),
                    &param.shape(),
                ).expect("failed to create tensor for weight restore");
                param.update_data(tensor);
                offset += numel;
            }
        }
        info!("Restored {} / {} weight values from checkpoint", offset, saved.weights.len());
    }

    // Build optimizer using AxonML (operates on model parameters directly).
    let num_params = model.num_parameters();
    let mut optimizer = architectures::build_optimizer(
        config.architecture,
        model.as_ref(),
        hp,
    );

    // Build loss function.
    let loss_fn = architectures::build_loss(config.architecture);

    info!(
        "Model built: {} ({} parameters)",
        config.architecture, num_params
    );

    // Prepare training data.
    let (train_inputs, train_targets) =
        prepare_training_data(train_set, config.architecture, hp.sequence_length);
    let (val_inputs, val_targets) =
        prepare_training_data(val_set, config.architecture, hp.sequence_length);

    if train_inputs.is_empty() {
        return Err(TrainingError::Training(
            "no training samples after preprocessing".into(),
        ));
    }

    // Training loop.
    let mut best_val_loss = f32::MAX;
    let mut patience_counter = 0usize;
    let mut final_train_loss = f32::MAX;
    let mut final_val_loss = f32::MAX;
    let mut epochs_trained = 0usize;

    for epoch in 0..hp.epochs {
        epochs_trained = epoch + 1;

        // Yield to let the tokio runtime process events, cancellation, etc.
        tokio::task::yield_now().await;

        // Train one epoch using AxonML autograd (forward → loss → backward → step).
        let train_loss = architectures::train_epoch(
            model.as_ref(),
            optimizer.as_mut(),
            &*loss_fn,
            &train_inputs,
            &train_targets,
            hp.batch_size,
        );
        final_train_loss = train_loss;

        // Validation.
        let val_loss = if epoch % hp.val_check_interval == 0 && !val_inputs.is_empty() {
            let vl = architectures::compute_validation_loss(
                model.as_ref(),
                &*loss_fn,
                &val_inputs,
                &val_targets,
            );
            final_val_loss = vl;

            // Compute validation metrics periodically.
            if epoch % (hp.val_check_interval * 5) == 0 {
                let val_metrics = architectures::compute_eval_metrics(
                    model.as_ref(),
                    &*loss_fn,
                    &val_inputs,
                    &val_targets,
                );
                send_event(
                    sender,
                    TrainingEvent::ValidationResult {
                        epoch: epochs_trained,
                        metrics: val_metrics,
                    },
                )
                .await;
            }

            Some(vl)
        } else {
            None
        };

        // Send epoch progress event.
        send_event(
            sender,
            TrainingEvent::EpochComplete {
                epoch: epochs_trained,
                total_epochs: hp.epochs,
                train_loss,
                val_loss,
            },
        )
        .await;

        // Early stopping check.
        if let Some(vl) = val_loss {
            if vl < best_val_loss {
                best_val_loss = vl;
                patience_counter = 0;
            } else {
                patience_counter += 1;
                if patience_counter >= hp.early_stopping_patience {
                    info!(
                        "Early stopping at epoch {} (best val_loss={:.6})",
                        epochs_trained, best_val_loss
                    );
                    break;
                }
            }
        }
    }

    Ok((model, epochs_trained, final_train_loss, final_val_loss))
}

// ---------------------------------------------------------------------------
// Stage 4: Evaluate
// ---------------------------------------------------------------------------

fn evaluate_stage(
    model: &dyn TrainableModel,
    test_set: &Dataset,
    config: &TrainingConfig,
) -> Result<Metrics> {
    let hp = &config.hyperparameters;
    let (test_inputs, test_targets) =
        prepare_training_data(test_set, config.architecture, hp.sequence_length);

    if test_inputs.is_empty() {
        return Err(TrainingError::Evaluation(
            "no test samples for evaluation".into(),
        ));
    }

    let loss_fn = architectures::build_loss(config.architecture);
    let metrics =
        architectures::compute_eval_metrics(model, &*loss_fn, &test_inputs, &test_targets);

    Ok(metrics)
}

// ---------------------------------------------------------------------------
// Stage 5: Export
// ---------------------------------------------------------------------------

fn export_stage(
    model: &dyn TrainableModel,
    config: &TrainingConfig,
    norm_stats: &NormalizationStats,
) -> Result<(String, Option<String>, Option<String>)> {
    // Create model weights container.
    let model_weights = export::create_model_weights(
        model,
        &config.hyperparameters,
        Some(norm_stats),
        None, // anomaly_threshold computed separately for autoencoders
    );

    // Save the model.
    let model_path = format!("{}/{}.axonml", config.output_path, config.run_id);
    let artifact_path = export::save_model(&model_weights, &model_path)?;

    // Quantize if requested.
    let quantized_path = if config.quantize {
        let quantized_weights = export::quantize_int8(&model_weights)?;
        let quant_path = format!("{}/{}-int8.axonml", config.output_path, config.run_id);
        let path = export::save_model(&quantized_weights, &quant_path)?;
        Some(path)
    } else {
        None
    };

    // Cross-compile if target specified.
    let arm_binary_path = if let Some(ref target) = config.cross_compile_target {
        let source_model = quantized_path.as_deref().unwrap_or(&artifact_path);
        let cc_config = CrossCompileConfig::new(
            source_model,
            format!("{}/edge", config.output_path),
        )
        .with_target(target.clone());

        match cross_compile::build_inference_binary(&cc_config) {
            Ok(path) => Some(path),
            Err(e) => {
                warn!("Cross-compilation skipped: {e}");
                None
            }
        }
    } else {
        None
    };

    Ok((artifact_path, quantized_path, arm_binary_path))
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Prepare flat input/target vectors from a dataset for training.
///
/// For temporal models (LSTM/GRU), creates sequences. For the Sentinel MLP,
/// uses raw feature vectors.
fn prepare_training_data(
    dataset: &Dataset,
    architecture: Architecture,
    sequence_length: usize,
) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
    // Temporal models: create sequences from time-series data
    if architecture.is_temporal() {
        match architecture {
            Architecture::LstmAutoencoder => {
                let sequences = preprocessor::create_sequences(&dataset.data, sequence_length);
                if sequences.is_empty() {
                    let inputs = dataset.data.clone();
                    let targets = dataset.data.clone();
                    return (inputs, targets);
                }
                let inputs: Vec<Vec<f32>> = sequences
                    .iter()
                    .map(|(seq, _)| seq.iter().flatten().copied().collect())
                    .collect();
                let targets = inputs.clone();
                (inputs, targets)
            }
            Architecture::GruPredictor => {
                if let Some(ref labels) = dataset.labels {
                    let labeled_seqs =
                        preprocessor::create_labeled_sequences(&dataset.data, labels, sequence_length);
                    if labeled_seqs.is_empty() {
                        let inputs = dataset.data.clone();
                        let targets: Vec<Vec<f32>> = labels
                            .iter()
                            .map(|&l| vec![l, l, l])
                            .collect();
                        return (inputs, targets);
                    }
                    let inputs: Vec<Vec<f32>> = labeled_seqs
                        .iter()
                        .map(|(seq, _)| seq.iter().flatten().copied().collect())
                        .collect();
                    let targets: Vec<Vec<f32>> = labeled_seqs
                        .iter()
                        .map(|(_, label)| vec![*label, *label, *label])
                        .collect();
                    (inputs, targets)
                } else {
                    let sequences =
                        preprocessor::create_sequences(&dataset.data, sequence_length);
                    if sequences.is_empty() {
                        return (Vec::new(), Vec::new());
                    }
                    let inputs: Vec<Vec<f32>> = sequences
                        .iter()
                        .map(|(seq, _)| seq.iter().flatten().copied().collect())
                        .collect();
                    let targets: Vec<Vec<f32>> = sequences
                        .iter()
                        .map(|(_, target)| {
                            let mut t = vec![0.0f32; 3];
                            for (i, val) in target.iter().take(3).enumerate() {
                                t[i] = architectures::sigmoid(*val);
                            }
                            t
                        })
                        .collect();
                    (inputs, targets)
                }
            }
            // Rnn, Conv1d, Sentinel: single-output temporal/tabular models
            _ => {
                let inputs = dataset.data.clone();
                let targets: Vec<Vec<f32>> = if let Some(ref labels) = dataset.labels {
                    labels.iter().map(|&l| vec![l]).collect()
                } else {
                    dataset.data.iter().map(|row| {
                        let mean = row.iter().sum::<f32>() / row.len().max(1) as f32;
                        vec![architectures::sigmoid(mean)]
                    }).collect()
                };
                (inputs, targets)
            }
        }
    } else if architecture.is_vision() {
        // Vision models: flatten image data, use labels as one-hot class targets
        let inputs = dataset.data.clone();
        let num_classes = dataset.data.first().map(|r| r.len()).unwrap_or(10).min(1000).max(2);
        let targets: Vec<Vec<f32>> = if let Some(ref labels) = dataset.labels {
            labels.iter().map(|&l| {
                let class = (l as usize).min(num_classes - 1);
                let mut one_hot = vec![0.0f32; num_classes];
                one_hot[class] = 1.0;
                one_hot
            }).collect()
        } else {
            // No labels: uniform distribution as target
            let uniform = 1.0 / num_classes as f32;
            dataset.data.iter().map(|_| vec![uniform; num_classes]).collect()
        };
        (inputs, targets)
    } else if architecture.is_nlp() {
        // NLP models: use features as token embeddings, labels as class targets
        let inputs = dataset.data.clone();
        let targets: Vec<Vec<f32>> = if let Some(ref labels) = dataset.labels {
            if architecture == Architecture::Gpt2 {
                // GPT-2: next-token prediction — target is shifted input
                dataset.data.iter().skip(1).map(|row| row.clone()).collect()
            } else {
                // BERT: classification
                labels.iter().map(|&l| {
                    let mut t = vec![0.0f32; 2];
                    t[if l > 0.5 { 1 } else { 0 }] = 1.0;
                    t
                }).collect()
            }
        } else {
            dataset.data.iter().map(|row| row.clone()).collect()
        };
        // Ensure equal lengths
        let min_len = inputs.len().min(targets.len());
        (inputs[..min_len].to_vec(), targets[..min_len].to_vec())
    } else {
        // Nexus, Phantom: tabular with single output
        let inputs = dataset.data.clone();
        let targets: Vec<Vec<f32>> = if let Some(ref labels) = dataset.labels {
            labels.iter().map(|&l| vec![l]).collect()
        } else {
            dataset.data.iter().map(|row| {
                let mean = row.iter().sum::<f32>() / row.len().max(1) as f32;
                vec![architectures::sigmoid(mean)]
            }).collect()
        };
        (inputs, targets)
    }
}

/// Send a training event through the channel (if available).
async fn send_event(
    sender: &Option<mpsc::Sender<TrainingEvent>>,
    event: TrainingEvent,
) {
    if let Some(tx) = sender {
        // Use try_send to avoid blocking the training thread on a full channel.
        // If the channel is full, we just drop the event — the next one will get through.
        if let Err(e) = tx.try_send(event) {
            warn!("Failed to send training event: {}", e);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Hyperparameters;

    fn make_test_dataset(n: usize, features: usize) -> Dataset {
        let data: Vec<Vec<f32>> = (0..n)
            .map(|i| {
                (0..features)
                    .map(|j| (i * features + j) as f32 * 0.01)
                    .collect()
            })
            .collect();
        let labels: Vec<f32> = (0..n).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();
        Dataset::new(data, Some(labels), (0..features).map(|i| format!("f{i}")).collect())
            .unwrap()
    }

    #[test]
    fn test_prepare_sentinel_data() {
        let ds = make_test_dataset(100, 5);
        let (inputs, targets) = prepare_training_data(&ds, Architecture::Sentinel, 10);
        assert_eq!(inputs.len(), 100);
        assert_eq!(targets.len(), 100);
        assert_eq!(targets[0].len(), 1); // Single health score
    }

    #[test]
    fn test_prepare_lstm_data() {
        let ds = make_test_dataset(100, 5);
        let (inputs, targets) = prepare_training_data(&ds, Architecture::LstmAutoencoder, 10);
        // With 100 samples and seq_len=10, we get 90 sequences.
        assert_eq!(inputs.len(), 90);
        assert_eq!(targets.len(), 90);
        // Each input is a flattened 10x5 sequence.
        assert_eq!(inputs[0].len(), 50);
    }

    #[test]
    fn test_prepare_gru_data() {
        let ds = make_test_dataset(100, 5);
        let (inputs, targets) = prepare_training_data(&ds, Architecture::GruPredictor, 10);
        assert!(!inputs.is_empty());
        assert_eq!(targets[0].len(), 3); // 3 failure horizons
    }

    #[test]
    fn test_compute_validation_loss() {
        let model = architectures::build_model(
            Architecture::Sentinel,
            5,
            &Hyperparameters::default(),
        )
        .unwrap();

        let inputs = vec![vec![0.1f32; 5]; 10];
        let targets = vec![vec![0.5f32]; 10];

        let loss_fn = architectures::build_loss(Architecture::Sentinel);
        let loss = architectures::compute_validation_loss(
            model.as_ref(),
            &*loss_fn,
            &inputs,
            &targets,
        );
        assert!(loss.is_finite());
        assert!(loss >= 0.0);
    }

    // -----------------------------------------------------------------------
    // Additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_training_config_missing_dataset_path_fails() {
        let mut config = TrainingConfig::new(
            Architecture::Sentinel,
            "/some/dataset.csv",
            "/tmp/output",
            5,
        );
        config.dataset_path = String::new();
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_training_config_missing_output_path_fails() {
        let mut config = TrainingConfig::new(
            Architecture::Sentinel,
            "/some/dataset.csv",
            "/tmp/output",
            5,
        );
        config.output_path = String::new();
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_training_config_zero_features_fails() {
        let mut config = TrainingConfig::new(
            Architecture::Sentinel,
            "/some/dataset.csv",
            "/tmp/output",
            5,
        );
        config.input_features = 0;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_training_config_invalid_split_ratios() {
        let mut config = TrainingConfig::new(
            Architecture::Sentinel,
            "/some/dataset.csv",
            "/tmp/output",
            5,
        );
        config.train_split = 0.5;
        config.val_split = 0.5;
        config.test_split = 0.5; // sums to 1.5
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_training_config_with_all_fields_valid() {
        let config = TrainingConfig::new(
            Architecture::GruPredictor,
            "/some/dataset.csv",
            "/tmp/output",
            10,
        );
        // Default config should pass validation.
        let result = config.validate();
        assert!(result.is_ok());
        assert!(!config.run_id.is_empty());
        assert_eq!(config.architecture, Architecture::GruPredictor);
        assert_eq!(config.input_features, 10);
    }

    #[test]
    fn test_training_config_default_values() {
        let config = TrainingConfig::new(
            Architecture::Sentinel,
            "/data.csv",
            "/output",
            5,
        );
        // Check default hyperparameters.
        assert_eq!(config.hyperparameters.epochs, 100);
        assert_eq!(config.hyperparameters.batch_size, 32);
        assert!((config.hyperparameters.learning_rate - 0.001).abs() < 1e-10);
        assert_eq!(config.hyperparameters.sequence_length, 60);
        assert_eq!(config.hyperparameters.hidden_dim, 64);

        // Check default split ratios.
        assert!((config.train_split - 0.7).abs() < 1e-10);
        assert!((config.val_split - 0.15).abs() < 1e-10);
        assert!((config.test_split - 0.15).abs() < 1e-10);

        // Quantize default.
        assert!(config.quantize);
    }

    #[test]
    fn test_training_config_zero_epochs_fails() {
        let mut config = TrainingConfig::new(
            Architecture::Sentinel,
            "/data.csv",
            "/output",
            5,
        );
        config.hyperparameters.epochs = 0;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_training_config_zero_batch_size_fails() {
        let mut config = TrainingConfig::new(
            Architecture::Sentinel,
            "/data.csv",
            "/output",
            5,
        );
        config.hyperparameters.batch_size = 0;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_training_config_negative_lr_fails() {
        let mut config = TrainingConfig::new(
            Architecture::Sentinel,
            "/data.csv",
            "/output",
            5,
        );
        config.hyperparameters.learning_rate = -0.001;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_event_channel_receives_events() {
        let (tx, mut rx) = mpsc::channel::<TrainingEvent>(16);

        send_event(
            &Some(tx),
            TrainingEvent::StageChange {
                stage: "test".into(),
                message: "hello".into(),
            },
        )
        .await;

        let event = rx.recv().await.unwrap();
        match event {
            TrainingEvent::StageChange { stage, message } => {
                assert_eq!(stage, "test");
                assert_eq!(message, "hello");
            }
            _ => panic!("unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_send_event_none_sender_does_not_panic() {
        // Sending with None sender should silently do nothing.
        send_event(
            &None,
            TrainingEvent::StageChange {
                stage: "noop".into(),
                message: "ignored".into(),
            },
        )
        .await;
    }

    #[test]
    fn test_compute_validation_loss_empty_inputs() {
        let model = architectures::build_model(
            Architecture::Sentinel,
            5,
            &Hyperparameters::default(),
        )
        .unwrap();

        let loss_fn = architectures::build_loss(Architecture::Sentinel);
        let loss = architectures::compute_validation_loss(
            model.as_ref(),
            &*loss_fn,
            &[],
            &[],
        );
        assert!((loss - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_eval_metrics_empty() {
        let model = architectures::build_model(
            Architecture::Sentinel,
            5,
            &Hyperparameters::default(),
        )
        .unwrap();

        let loss_fn = architectures::build_loss(Architecture::Sentinel);
        let m = architectures::compute_eval_metrics(
            model.as_ref(),
            &*loss_fn,
            &[],
            &[],
        );
        // Should return default metrics for empty input.
        assert_eq!(m.accuracy, 0.0);
    }

    #[test]
    fn test_prepare_training_data_gru_no_labels() {
        let data: Vec<Vec<f32>> = (0..20).map(|i| vec![i as f32 * 0.01; 3]).collect();
        let ds = Dataset::new(data, None, vec!["a".into(), "b".into(), "c".into()]).unwrap();

        let (inputs, targets) = prepare_training_data(&ds, Architecture::GruPredictor, 5);
        // With no labels and 20 samples, seq_len=5 => 15 sequences.
        assert_eq!(inputs.len(), 15);
        // GRU targets should have 3 elements (failure horizons).
        assert_eq!(targets[0].len(), 3);
    }

    #[test]
    fn test_make_test_dataset_helper() {
        let ds = make_test_dataset(50, 3);
        assert_eq!(ds.num_samples, 50);
        assert_eq!(ds.num_features, 3);
        assert!(ds.labels.is_some());
        assert_eq!(ds.labels.as_ref().unwrap().len(), 50);
    }

    #[test]
    fn test_training_event_serialization() {
        let event = TrainingEvent::EpochComplete {
            epoch: 5,
            total_epochs: 100,
            train_loss: 0.1,
            val_loss: Some(0.2),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("epoch_complete"));
        assert!(json.contains("\"epoch\":5"));
    }
}
