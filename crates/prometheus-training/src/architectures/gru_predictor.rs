// ============================================================================
// File: gru_predictor.rs
// Description: Multi-horizon GRU predictor for failure probability at 5/15/30 minute horizons
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Multi-Horizon GRU Predictor — powered by AxonML layers.
//!
//! Architecture:
//! - GRU(input_features, hidden_dim, num_layers)
//! - Linear(hidden_dim, hidden_dim) -> ReLU
//! - Linear(hidden_dim, 3) -> Sigmoid
//!
//! Predicts failure probabilities at 3 horizons (5/15/30 min).
//! Training: BCE loss, AdamW optimizer.

use axonml_autograd::Variable;
use axonml_nn::{Linear, Module, Parameter, ReLU, Sigmoid, GRU};

use super::TrainableModel;
use crate::Architecture;

pub struct GruPredictor {
    input_features: usize,
    gru: GRU,
    fc1: Linear,
    relu: ReLU,
    fc2: Linear,
    sigmoid: Sigmoid,
}

impl GruPredictor {
    pub fn new(input_features: usize, hidden_dim: usize, num_layers: usize, _sequence_length: usize) -> Self {
        Self {
            input_features,
            gru: GRU::new(input_features, hidden_dim, num_layers),
            fc1: Linear::new(hidden_dim, hidden_dim),
            relu: ReLU,
            fc2: Linear::new(hidden_dim, 3),
            sigmoid: Sigmoid,
        }
    }
}

impl TrainableModel for GruPredictor {
    fn forward(&self, input: &Variable) -> Variable {
        // Input: [batch, seq_len * features] (flattened)
        let shape = input.shape();
        let batch = shape[0];
        let total = shape[1];
        let seq_len = (total / self.input_features).max(1);

        // Reshape to [batch, seq_len, features]
        let reshaped = input.reshape(&[batch, seq_len, self.input_features]);

        // GRU: [batch, seq_len, hidden_dim]
        let gru_out = self.gru.forward(&reshaped);

        // Take last timestep: [batch, hidden_dim]
        let last_idx = gru_out.shape()[1] - 1;
        let last_hidden = gru_out.select(1, last_idx);

        // FC layers: [batch, 3] with sigmoid
        let h = self.relu.forward(&self.fc1.forward(&last_hidden));
        self.sigmoid.forward(&self.fc2.forward(&h))
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = self.gru.parameters();
        params.extend(self.fc1.parameters());
        params.extend(self.fc2.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::GruPredictor
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for GruPredictor {}
unsafe impl Sync for GruPredictor {}

#[cfg(test)]
mod tests {
    use super::*;
    use axonml_tensor::Tensor;

    #[test]
    fn test_gru_predictor_forward() {
        let model = GruPredictor::new(5, 16, 1, 10);
        let input = Variable::new(
            Tensor::from_vec(vec![0.1f32; 100], &[2, 50]).unwrap(),
            false,
        );
        let output = model.forward(&input);
        assert_eq!(output.shape(), vec![2, 3]);
        let vals = output.data().to_vec();
        for &v in &vals {
            assert!(v >= 0.0 && v <= 1.0);
        }
    }
}
