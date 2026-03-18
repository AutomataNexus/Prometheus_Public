// ============================================================================
// File: lib.rs
// Description: Core types, enums, and configuration for the AxonML training pipeline orchestrator
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! # prometheus-training
//!
//! AxonML training pipeline orchestrator for Prometheus.
//!
//! Provides a 5-stage ML training pipeline (validate -> preprocess -> train -> evaluate -> export)
//! that trains LSTM Autoencoder, GRU Predictor, and Sentinel health scorer models for
//! edge deployment on Raspberry Pi controllers.

pub mod architectures;
pub mod cross_compile;
pub mod export;
pub mod metrics;
pub mod pipeline;
pub mod preprocessor;

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Unified error type for the training crate.
#[derive(Debug, Error)]
pub enum TrainingError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("preprocessing failed: {0}")]
    Preprocessing(String),

    #[error("training failed: {0}")]
    Training(String),

    #[error("evaluation failed: {0}")]
    Evaluation(String),

    #[error("export failed: {0}")]
    Export(String),

    #[error("cross-compilation failed: {0}")]
    CrossCompile(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV parsing error: {0}")]
    Csv(#[from] csv::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("channel send error: {0}")]
    ChannelSend(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("model not found: {0}")]
    ModelNotFound(String),
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, TrainingError>;

// ---------------------------------------------------------------------------
// Core enums
// ---------------------------------------------------------------------------

/// Supported neural network architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    // -- Time series / sequence models --
    /// LSTM Autoencoder for anomaly detection via reconstruction error.
    LstmAutoencoder,
    /// Multi-horizon GRU for failure prediction at 5/15/30 minute horizons.
    GruPredictor,
    /// Vanilla RNN for simple sequence modeling.
    Rnn,
    /// MLP health scorer producing a 0.0-1.0 facility health score.
    Sentinel,

    // -- Computer vision models --
    /// ResNet-18/34/50 image classification.
    ResNet,
    /// VGG-11/13/16/19 image classification.
    Vgg,
    /// Vision Transformer (ViT) image classification.
    ViT,

    // -- NLP / language models --
    /// BERT bidirectional encoder for text classification.
    Bert,
    /// GPT-2 autoregressive text generation.
    Gpt2,

    // -- Advanced architectures --
    /// Nexus multi-modal fusion model.
    Nexus,
    /// Phantom lightweight edge model.
    Phantom,
    /// Conv1d temporal feature extraction.
    Conv1d,
    /// Conv2d spatial feature extraction.
    Conv2d,
}

impl Architecture {
    /// Whether this architecture operates on image data.
    pub fn is_vision(&self) -> bool {
        matches!(self, Self::ResNet | Self::Vgg | Self::ViT | Self::Conv2d)
    }

    /// Whether this architecture operates on text data.
    pub fn is_nlp(&self) -> bool {
        matches!(self, Self::Bert | Self::Gpt2)
    }

    /// Whether this architecture operates on temporal/sequence data.
    pub fn is_temporal(&self) -> bool {
        matches!(self, Self::LstmAutoencoder | Self::GruPredictor | Self::Rnn | Self::Sentinel | Self::Conv1d)
    }
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Architecture::LstmAutoencoder => write!(f, "LSTM Autoencoder"),
            Architecture::GruPredictor => write!(f, "GRU Predictor"),
            Architecture::Rnn => write!(f, "RNN"),
            Architecture::Sentinel => write!(f, "Sentinel"),
            Architecture::ResNet => write!(f, "ResNet"),
            Architecture::Vgg => write!(f, "VGG"),
            Architecture::ViT => write!(f, "Vision Transformer"),
            Architecture::Bert => write!(f, "BERT"),
            Architecture::Gpt2 => write!(f, "GPT-2"),
            Architecture::Nexus => write!(f, "Nexus"),
            Architecture::Phantom => write!(f, "Phantom"),
            Architecture::Conv1d => write!(f, "Conv1d"),
            Architecture::Conv2d => write!(f, "Conv2d"),
        }
    }
}

/// Current status of a training run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingStatus {
    /// Queued but not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Successfully finished.
    Completed,
    /// Terminated due to error.
    Failed,
}

/// Progress events emitted during pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TrainingEvent {
    /// Pipeline stage transition.
    StageChange {
        stage: String,
        message: String,
    },
    /// An epoch has completed.
    EpochComplete {
        epoch: usize,
        total_epochs: usize,
        train_loss: f32,
        val_loss: Option<f32>,
    },
    /// Validation metrics computed.
    ValidationResult {
        epoch: usize,
        metrics: metrics::Metrics,
    },
    /// Training has finished successfully.
    TrainingDone {
        result: TrainingResult,
    },
    /// An error occurred.
    Error {
        message: String,
    },
}

// ---------------------------------------------------------------------------
// Hyperparameters
// ---------------------------------------------------------------------------

/// Hyperparameter configuration for training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperparameters {
    /// Learning rate for the optimizer.
    pub learning_rate: f64,
    /// Number of training epochs.
    pub epochs: usize,
    /// Batch size for mini-batch gradient descent.
    pub batch_size: usize,
    /// Sequence length for temporal models (LSTM/GRU).
    pub sequence_length: usize,
    /// Hidden dimension for recurrent layers.
    pub hidden_dim: usize,
    /// Number of recurrent layers.
    pub num_layers: usize,
    /// Dropout probability (0.0 = no dropout).
    pub dropout: f64,
    /// Weight decay for AdamW.
    pub weight_decay: f64,
    /// Early stopping patience (epochs without improvement).
    pub early_stopping_patience: usize,
    /// Validation check interval (in epochs).
    pub val_check_interval: usize,
}

impl Default for Hyperparameters {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            epochs: 100,
            batch_size: 32,
            sequence_length: 60,
            hidden_dim: 64,
            num_layers: 2,
            dropout: 0.1,
            weight_decay: 0.01,
            early_stopping_patience: 10,
            val_check_interval: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Training configuration
// ---------------------------------------------------------------------------

/// Complete training configuration for a pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// Unique identifier for this training run.
    pub run_id: String,
    /// Neural network architecture to train.
    pub architecture: Architecture,
    /// Hyperparameters governing training behavior.
    pub hyperparameters: Hyperparameters,
    /// Path to the input dataset (CSV file or directory).
    pub dataset_path: String,
    /// Directory for saving trained model artifacts.
    pub output_path: String,
    /// Number of input features (columns in the dataset).
    pub input_features: usize,
    /// Train/validation/test split ratios.
    pub train_split: f64,
    pub val_split: f64,
    pub test_split: f64,
    /// Whether to quantize the model to INT8 after training.
    pub quantize: bool,
    /// Optional cross-compilation target triple.
    pub cross_compile_target: Option<String>,
    /// Optional path to a pre-trained .axonml file to resume training from.
    /// When set, the model is initialized with these weights before training.
    pub resume_from: Option<String>,
}

impl TrainingConfig {
    /// Create a new training configuration with sensible defaults.
    pub fn new(
        architecture: Architecture,
        dataset_path: impl Into<String>,
        output_path: impl Into<String>,
        input_features: usize,
    ) -> Self {
        Self {
            run_id: Uuid::new_v4().to_string(),
            architecture,
            hyperparameters: Hyperparameters::default(),
            dataset_path: dataset_path.into(),
            output_path: output_path.into(),
            input_features,
            train_split: 0.7,
            val_split: 0.15,
            test_split: 0.15,
            quantize: true,
            cross_compile_target: Some("armv7-unknown-linux-musleabihf".to_string()),
            resume_from: None,
        }
    }

    /// Validate the configuration for correctness.
    pub fn validate(&self) -> Result<()> {
        if self.input_features == 0 {
            return Err(TrainingError::InvalidConfig(
                "input_features must be > 0".into(),
            ));
        }
        let split_sum = self.train_split + self.val_split + self.test_split;
        if (split_sum - 1.0).abs() > 1e-6 {
            return Err(TrainingError::InvalidConfig(format!(
                "split ratios must sum to 1.0, got {split_sum}"
            )));
        }
        if self.hyperparameters.learning_rate <= 0.0 {
            return Err(TrainingError::InvalidConfig(
                "learning_rate must be positive".into(),
            ));
        }
        if self.hyperparameters.epochs == 0 {
            return Err(TrainingError::InvalidConfig(
                "epochs must be > 0".into(),
            ));
        }
        if self.hyperparameters.batch_size == 0 {
            return Err(TrainingError::InvalidConfig(
                "batch_size must be > 0".into(),
            ));
        }
        if self.dataset_path.is_empty() {
            return Err(TrainingError::InvalidConfig(
                "dataset_path must not be empty".into(),
            ));
        }
        if self.output_path.is_empty() {
            return Err(TrainingError::InvalidConfig(
                "output_path must not be empty".into(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Training result
// ---------------------------------------------------------------------------

/// Result of a completed training pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingResult {
    /// Unique identifier matching the TrainingConfig.
    pub run_id: String,
    /// Architecture that was trained.
    pub architecture: Architecture,
    /// Final evaluation metrics on the test set.
    pub metrics: metrics::Metrics,
    /// Path to the saved model artifact (.axonml file).
    pub artifact_path: String,
    /// Path to the quantized model (if quantization was enabled).
    pub quantized_artifact_path: Option<String>,
    /// Path to the cross-compiled ARM binary (if cross-compilation was enabled).
    pub arm_binary_path: Option<String>,
    /// Total wall-clock training time.
    pub training_duration: Duration,
    /// Number of epochs actually trained (may be < configured if early stopping triggered).
    pub epochs_trained: usize,
    /// Final training loss.
    pub final_train_loss: f32,
    /// Final validation loss.
    pub final_val_loss: f32,
    /// Timestamp when training completed.
    pub completed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Re-exports for convenience
// ---------------------------------------------------------------------------

pub use architectures::ModelWeights;
pub use metrics::Metrics;
pub use pipeline::run_pipeline;
pub use preprocessor::Dataset;
