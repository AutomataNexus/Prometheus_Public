// ============================================================================
// File: mod.rs
// Description: Neural network architecture registry with model building, training, and evaluation via AxonML
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Neural network architecture registry.
//!
//! Uses AxonML's autograd, layers (Linear, LSTM, GRU, Conv2d, TransformerEncoder, etc.),
//! and optimizers (Adam, AdamW) for real backpropagation training.

pub mod gru_predictor;
pub mod lstm_autoencoder;
pub mod sentinel;
pub mod rnn_model;
pub mod resnet_model;
pub mod vgg_model;
pub mod conv_models;
pub mod nlp_models;
pub mod advanced_models;

use serde::{Deserialize, Serialize};

use axonml_autograd::Variable;
use axonml_nn::Parameter;
use axonml_tensor::Tensor;

use crate::{Architecture, Hyperparameters, Result};

// ---------------------------------------------------------------------------
// TrainableModel trait — unified interface for all architectures
// ---------------------------------------------------------------------------

/// Core trait for all Prometheus model architectures.
///
/// Each architecture struct holds AxonML layers (Linear, LSTM, Conv2d, etc.)
/// and implements this trait to provide forward pass, parameter access,
/// and metadata for the training pipeline.
pub trait TrainableModel: Send + Sync {
    /// Run the forward pass on a batched input Variable.
    ///
    /// Input shape depends on the architecture:
    /// - Tabular/MLP: [batch, features]
    /// - Temporal/RNN: [batch, seq_len, features]
    /// - Vision/CNN: [batch, channels, height, width]
    fn forward(&self, input: &Variable) -> Variable;

    /// Return all learnable parameters (for optimizer construction).
    fn parameters(&self) -> Vec<Parameter>;

    /// Return the total number of trainable scalar parameters.
    fn num_parameters(&self) -> usize {
        self.parameters()
            .iter()
            .filter(|p| p.requires_grad())
            .map(|p| p.numel())
            .sum()
    }

    /// Return the architecture type.
    fn architecture(&self) -> Architecture;

    /// Return the number of input features expected.
    fn input_features(&self) -> usize;

    /// Extract all parameters as a flat f32 vector (for serialization/export).
    fn flat_parameters(&self) -> Vec<f32> {
        self.parameters()
            .iter()
            .flat_map(|p| p.data().to_vec())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Model weights (serializable parameter container)
// ---------------------------------------------------------------------------

/// Serializable container for model weights and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelWeights {
    /// Architecture that produced these weights.
    pub architecture: Architecture,
    /// Number of input features.
    pub input_features: usize,
    /// Hyperparameters used during training.
    pub hyperparameters: Hyperparameters,
    /// Flattened parameter vector (f32).
    pub weights: Vec<f32>,
    /// Per-feature normalization means (for inference).
    pub norm_means: Vec<f32>,
    /// Per-feature normalization stds (for inference).
    pub norm_stds: Vec<f32>,
    /// Anomaly threshold (for autoencoder models).
    pub anomaly_threshold: Option<f32>,
}

// ---------------------------------------------------------------------------
// Loss function factory
// ---------------------------------------------------------------------------

/// Build the appropriate loss function for an architecture.
///
/// Returns a closure that computes loss as a Variable (tracked by autograd).
pub fn build_loss(architecture: Architecture) -> Box<dyn Fn(&Variable, &Variable) -> Variable + Send + Sync> {
    use axonml_nn::{BCELoss, CrossEntropyLoss, MSELoss};

    match architecture {
        Architecture::LstmAutoencoder => {
            Box::new(|pred, target| MSELoss::new().compute(pred, target))
        }
        Architecture::ResNet | Architecture::Vgg | Architecture::ViT
        | Architecture::Conv2d | Architecture::Bert | Architecture::Gpt2 => {
            Box::new(|pred, target| CrossEntropyLoss::new().compute(pred, target))
        }
        _ => {
            // GruPredictor, Sentinel, Rnn, Conv1d, Nexus, Phantom
            Box::new(|pred, target| BCELoss::new().compute(pred, target))
        }
    }
}

// ---------------------------------------------------------------------------
// Architecture registry — build a model from config
// ---------------------------------------------------------------------------

/// Compute a safe image size from flat input features.
///
/// Rounds the square root UP so that `in_ch * img_size * img_size >= input_features`.
/// Vision model `forward()` methods should zero-pad or truncate the extra elements.
fn safe_img_size(input_features: usize, in_channels: usize, min_size: usize) -> usize {
    let pixels = input_features / in_channels;
    let s = (pixels as f32).sqrt().ceil() as usize;
    s.max(min_size)
}

/// Build a model for the given architecture and input features.
pub fn build_model(
    architecture: Architecture,
    input_features: usize,
    hyperparameters: &Hyperparameters,
) -> Result<Box<dyn TrainableModel>> {
    match architecture {
        Architecture::LstmAutoencoder => {
            let model = lstm_autoencoder::LstmAutoencoder::new(
                input_features,
                hyperparameters.hidden_dim,
                hyperparameters.num_layers,
            );
            Ok(Box::new(model))
        }
        Architecture::GruPredictor => {
            let model = gru_predictor::GruPredictor::new(
                input_features,
                hyperparameters.hidden_dim,
                hyperparameters.num_layers,
                hyperparameters.sequence_length,
            );
            Ok(Box::new(model))
        }
        Architecture::Rnn => {
            let model = rnn_model::RnnModel::new(
                input_features,
                hyperparameters.hidden_dim,
                hyperparameters.num_layers,
                hyperparameters.sequence_length,
            );
            Ok(Box::new(model))
        }
        Architecture::Sentinel => {
            let model = sentinel::Sentinel::new(input_features);
            Ok(Box::new(model))
        }
        Architecture::ResNet => {
            let in_ch = if input_features > 3072 { 3 } else { 1 };
            let img_size = safe_img_size(input_features, in_ch, 8);
            let num_classes = hyperparameters.hidden_dim.min(1000).max(2);
            let model = resnet_model::ResNetModel::resnet18(in_ch, num_classes, img_size);
            Ok(Box::new(model))
        }
        Architecture::Vgg => {
            let in_ch = if input_features > 3072 { 3 } else { 1 };
            let img_size = safe_img_size(input_features, in_ch, 8);
            let num_classes = hyperparameters.hidden_dim.min(1000).max(2);
            let model = vgg_model::VggModel::vgg11(in_ch, num_classes, img_size);
            Ok(Box::new(model))
        }
        Architecture::ViT => {
            let in_ch = if input_features > 3072 { 3 } else { 1 };
            let mut img_size = safe_img_size(input_features, in_ch, 16);
            let patch_size = (img_size / 4).max(4);
            // Ensure img_size is divisible by patch_size
            img_size = (img_size / patch_size) * patch_size;
            let img_size = img_size.max(patch_size);
            let num_classes = hyperparameters.hidden_dim.min(1000).max(2);
            let d_model = 128;
            let model = advanced_models::ViTModel::new(
                in_ch, num_classes, img_size, patch_size, d_model, 4, hyperparameters.num_layers,
            );
            Ok(Box::new(model))
        }
        Architecture::Bert => {
            let d_model = hyperparameters.hidden_dim.max(64);
            let num_classes = 2;
            let model = nlp_models::BertModel::new(
                input_features, num_classes, d_model, 4, hyperparameters.num_layers,
            );
            Ok(Box::new(model))
        }
        Architecture::Gpt2 => {
            let d_model = hyperparameters.hidden_dim.max(64);
            let output_dim = input_features;
            let model = nlp_models::Gpt2Model::new(
                input_features, output_dim, d_model, 4, hyperparameters.num_layers,
            );
            Ok(Box::new(model))
        }
        Architecture::Nexus => {
            let d_model = hyperparameters.hidden_dim.max(64);
            let model = advanced_models::NexusModel::new(input_features, d_model, 1);
            Ok(Box::new(model))
        }
        Architecture::Phantom => {
            let model = advanced_models::PhantomModel::new(input_features, 1);
            Ok(Box::new(model))
        }
        Architecture::Conv1d => {
            let model = conv_models::Conv1dModel::new(
                input_features, hyperparameters.hidden_dim, hyperparameters.num_layers,
            );
            Ok(Box::new(model))
        }
        Architecture::Conv2d => {
            let in_ch = if input_features > 3072 { 3 } else { 1 };
            let img_size = safe_img_size(input_features, in_ch, 8);
            let num_classes = hyperparameters.hidden_dim.min(1000).max(2);
            let model = conv_models::Conv2dModel::new(in_ch, num_classes, img_size);
            Ok(Box::new(model))
        }
    }
}

/// Build an AxonML optimizer for the given architecture and model parameters.
pub fn build_optimizer(
    architecture: Architecture,
    model: &dyn TrainableModel,
    hyperparameters: &Hyperparameters,
) -> Box<dyn axonml_optim::Optimizer> {
    let params = model.parameters();
    let lr = hyperparameters.learning_rate as f32;

    match architecture {
        Architecture::LstmAutoencoder | Architecture::Rnn | Architecture::Sentinel
        | Architecture::Nexus | Architecture::Phantom | Architecture::Conv1d => {
            Box::new(axonml_optim::Adam::new(params, lr))
        }
        Architecture::GruPredictor | Architecture::ResNet | Architecture::Vgg
        | Architecture::ViT | Architecture::Conv2d | Architecture::Bert
        | Architecture::Gpt2 => {
            Box::new(
                axonml_optim::AdamW::new(params, lr)
                    .weight_decay(hyperparameters.weight_decay as f32),
            )
        }
    }
}

/// Train a model for one epoch using AxonML autograd.
///
/// Performs real backpropagation: forward → loss → backward → optimizer step
/// (single forward + backward pass per batch).
pub fn train_epoch(
    model: &dyn TrainableModel,
    optimizer: &mut dyn axonml_optim::Optimizer,
    loss_fn: &dyn Fn(&Variable, &Variable) -> Variable,
    data: &[Vec<f32>],
    targets: &[Vec<f32>],
    batch_size: usize,
) -> f32 {
    let num_samples = data.len();
    if num_samples == 0 {
        return 0.0;
    }

    let num_batches = (num_samples + batch_size - 1) / batch_size;
    let mut total_loss = 0.0f32;

    for batch_idx in 0..num_batches {
        let start = batch_idx * batch_size;
        let end = (start + batch_size).min(num_samples);
        let actual_batch = end - start;

        // Flatten batch data into a single tensor [batch_size, features]
        let input_dim = data[start].len();
        let input_flat: Vec<f32> = data[start..end]
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect();
        let input_tensor = Tensor::from_vec(input_flat, &[actual_batch, input_dim])
            .expect("failed to create input tensor");
        let input_var = Variable::new(input_tensor, false);

        // Flatten batch targets
        let target_dim = targets[start].len();
        let target_flat: Vec<f32> = targets[start..end]
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect();
        let target_tensor = Tensor::from_vec(target_flat, &[actual_batch, target_dim])
            .expect("failed to create target tensor");
        let target_var = Variable::new(target_tensor, false);

        // Forward pass (builds computational graph)
        let output = model.forward(&input_var);

        // Compute loss (tracked by autograd)
        let loss = loss_fn(&output, &target_var);

        // Extract scalar loss value
        let loss_val = loss.data().to_vec()[0];
        total_loss += loss_val;

        // Zero gradients, backward pass, optimizer step
        optimizer.zero_grad();
        loss.backward();
        optimizer.step();
    }

    total_loss / num_batches as f32
}

/// Compute validation/test loss without gradient tracking.
pub fn compute_validation_loss(
    model: &dyn TrainableModel,
    loss_fn: &dyn Fn(&Variable, &Variable) -> Variable,
    inputs: &[Vec<f32>],
    targets: &[Vec<f32>],
) -> f32 {
    if inputs.is_empty() {
        return 0.0;
    }

    // Run in no_grad context for efficiency
    let _guard = axonml_autograd::NoGradGuard::new();

    let input_dim = inputs[0].len();
    let target_dim = targets[0].len();
    let n = inputs.len();

    let input_flat: Vec<f32> = inputs.iter().flat_map(|r| r.iter().copied()).collect();
    let target_flat: Vec<f32> = targets.iter().flat_map(|r| r.iter().copied()).collect();

    let input_tensor = Tensor::from_vec(input_flat, &[n, input_dim])
        .expect("failed to create input tensor");
    let target_tensor = Tensor::from_vec(target_flat, &[n, target_dim])
        .expect("failed to create target tensor");

    let input_var = Variable::new(input_tensor, false);
    let target_var = Variable::new(target_tensor, false);

    let output = model.forward(&input_var);
    let loss = loss_fn(&output, &target_var);

    loss.data().to_vec()[0]
}

/// Compute comprehensive evaluation metrics using the model's predictions.
pub fn compute_eval_metrics(
    model: &dyn TrainableModel,
    loss_fn: &dyn Fn(&Variable, &Variable) -> Variable,
    inputs: &[Vec<f32>],
    targets: &[Vec<f32>],
) -> crate::metrics::Metrics {
    if inputs.is_empty() || targets.is_empty() {
        return crate::metrics::Metrics::default();
    }

    let _guard = axonml_autograd::NoGradGuard::new();

    let input_dim = inputs[0].len();
    let target_dim = targets[0].len();
    let n = inputs.len();

    let input_flat: Vec<f32> = inputs.iter().flat_map(|r| r.iter().copied()).collect();
    let target_flat: Vec<f32> = targets.iter().flat_map(|r| r.iter().copied()).collect();

    let input_tensor = Tensor::from_vec(input_flat, &[n, input_dim])
        .expect("failed to create input tensor");
    let target_tensor = Tensor::from_vec(target_flat, &[n, target_dim])
        .expect("failed to create target tensor");

    let input_var = Variable::new(input_tensor, false);
    let target_var = Variable::new(target_tensor, false);

    let output = model.forward(&input_var);
    let loss = loss_fn(&output, &target_var);

    let all_preds = output.data().to_vec();
    let all_targets: Vec<f32> = targets.iter().flat_map(|r| r.iter().copied()).collect();

    let mut m = crate::metrics::compute_metrics(&all_preds, &all_targets);
    m.loss = loss.data().to_vec()[0];

    m
}

/// Sigmoid activation (standalone helper for data preprocessing).
pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigmoid() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-6);
        assert!(sigmoid(10.0) > 0.99);
        assert!(sigmoid(-10.0) < 0.01);
    }

    #[test]
    fn test_build_model() {
        let hp = Hyperparameters::default();

        let lstm = build_model(Architecture::LstmAutoencoder, 10, &hp).unwrap();
        assert_eq!(lstm.architecture(), Architecture::LstmAutoencoder);
        assert_eq!(lstm.input_features(), 10);

        let gru = build_model(Architecture::GruPredictor, 10, &hp).unwrap();
        assert_eq!(gru.architecture(), Architecture::GruPredictor);

        let sentinel = build_model(Architecture::Sentinel, 10, &hp).unwrap();
        assert_eq!(sentinel.architecture(), Architecture::Sentinel);
    }

    #[test]
    fn test_build_loss_returns_callable() {
        let mse_loss = build_loss(Architecture::LstmAutoencoder);
        let bce_loss = build_loss(Architecture::Sentinel);
        let ce_loss = build_loss(Architecture::ResNet);

        // Verify they produce a scalar output
        let pred = Variable::new(
            Tensor::from_vec(vec![0.5; 4], &[2, 2]).unwrap(),
            true,
        );
        let target = Variable::new(
            Tensor::from_vec(vec![1.0; 4], &[2, 2]).unwrap(),
            false,
        );

        let loss = mse_loss(&pred, &target);
        assert_eq!(loss.numel(), 1);

        let loss = bce_loss(&pred, &target);
        assert!(loss.data().to_vec()[0].is_finite());

        let target_ce = Variable::new(
            Tensor::from_vec(vec![0.0, 1.0], &[2]).unwrap(),
            false,
        );
        let loss = ce_loss(&pred, &target_ce);
        assert!(loss.data().to_vec()[0].is_finite());
    }
}
