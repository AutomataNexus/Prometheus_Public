// ============================================================================
// File: lstm_autoencoder.rs
// Description: LSTM Autoencoder for anomaly detection via reconstruction error using AxonML layers
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! LSTM Autoencoder for anomaly detection — powered by AxonML layers.
//!
//! Architecture:
//! - LSTM encoder (input_features -> hidden_dim)
//! - Linear bottleneck (hidden_dim -> bottleneck -> hidden_dim)
//! - LSTM decoder (hidden_dim -> hidden_dim)
//! - Linear output projection (hidden_dim -> input_features)
//!
//! Training: MSE reconstruction loss, Adam optimizer.

use axonml_autograd::Variable;
use axonml_nn::{Linear, Module, Parameter, LSTM};

use super::TrainableModel;
use crate::Architecture;

/// LSTM Autoencoder using AxonML layers.
pub struct LstmAutoencoder {
    input_features: usize,
    encoder: LSTM,
    bottleneck_down: Linear,
    bottleneck_up: Linear,
    decoder: LSTM,
    output_proj: Linear,
}

impl LstmAutoencoder {
    pub fn new(input_features: usize, hidden_dim: usize, num_layers: usize) -> Self {
        let bottleneck_dim = (hidden_dim / 2).max(1);
        Self {
            input_features,
            encoder: LSTM::new(input_features, hidden_dim, num_layers),
            bottleneck_down: Linear::new(hidden_dim, bottleneck_dim),
            bottleneck_up: Linear::new(bottleneck_dim, hidden_dim),
            decoder: LSTM::new(hidden_dim, hidden_dim, num_layers),
            output_proj: Linear::new(hidden_dim, input_features),
        }
    }
}

impl TrainableModel for LstmAutoencoder {
    fn forward(&self, input: &Variable) -> Variable {
        // Input: [batch, seq_len * features] (flattened sequence)
        let shape = input.shape();
        let batch = shape[0];
        let total = shape[1];
        let seq_len = (total / self.input_features).max(1);

        // Reshape to [batch, seq_len, features]
        let reshaped = input.reshape(&[batch, seq_len, self.input_features]);

        // Encode
        let encoded = self.encoder.forward(&reshaped);

        // Take last timestep hidden state: [batch, hidden_dim]
        let last_idx = encoded.shape()[1] - 1;
        let last_hidden = encoded.select(1, last_idx);

        // Bottleneck
        let compressed = self.bottleneck_down.forward(&last_hidden);
        let expanded = self.bottleneck_up.forward(&compressed);

        // Expand to sequence: [batch, seq_len, hidden_dim]
        let hidden_dim = expanded.shape()[expanded.shape().len() - 1];
        let expanded_seq = expanded.unsqueeze(1).expand(&[batch, seq_len, hidden_dim]);

        // Decode
        let decoded = self.decoder.forward(&expanded_seq);

        // Project to output features: [batch, seq_len, input_features]
        let output = self.output_proj.forward(&decoded);

        // Flatten: [batch, seq_len * features]
        output.reshape(&[batch, seq_len * self.input_features])
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = self.encoder.parameters();
        params.extend(self.bottleneck_down.parameters());
        params.extend(self.bottleneck_up.parameters());
        params.extend(self.decoder.parameters());
        params.extend(self.output_proj.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::LstmAutoencoder
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for LstmAutoencoder {}
unsafe impl Sync for LstmAutoencoder {}

#[cfg(test)]
mod tests {
    use super::*;
    use axonml_tensor::Tensor;

    #[test]
    fn test_lstm_autoencoder_creation() {
        let model = LstmAutoencoder::new(10, 32, 1);
        assert_eq!(model.input_features(), 10);
        assert_eq!(model.architecture(), Architecture::LstmAutoencoder);
        assert!(model.num_parameters() > 0);
    }

    #[test]
    fn test_lstm_autoencoder_forward() {
        let model = LstmAutoencoder::new(5, 16, 1);
        let input = Variable::new(
            Tensor::from_vec(vec![0.1f32; 40], &[2, 20]).unwrap(),
            false,
        );
        let output = model.forward(&input);
        assert_eq!(output.shape(), vec![2, 20]);
    }
}
