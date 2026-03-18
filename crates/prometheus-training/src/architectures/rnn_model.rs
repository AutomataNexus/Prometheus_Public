// ============================================================================
// File: rnn_model.rs
// Description: Vanilla RNN model for simple sequence modeling with sigmoid output
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Vanilla RNN model — powered by AxonML layers.
//!
//! Architecture:
//! - RNN(input_features, hidden_dim, num_layers)
//! - Linear(hidden_dim, 1) -> Sigmoid
//!
//! Training: BCE loss, Adam optimizer.

use axonml_autograd::Variable;
use axonml_nn::{Linear, Module, Parameter, Sigmoid, RNN};

use super::TrainableModel;
use crate::Architecture;

pub struct RnnModel {
    input_features: usize,
    rnn: RNN,
    fc: Linear,
    sigmoid: Sigmoid,
}

impl RnnModel {
    pub fn new(input_features: usize, hidden_dim: usize, num_layers: usize, _sequence_length: usize) -> Self {
        Self {
            input_features,
            rnn: RNN::new(input_features, hidden_dim, num_layers),
            fc: Linear::new(hidden_dim, 1),
            sigmoid: Sigmoid,
        }
    }
}

impl TrainableModel for RnnModel {
    fn forward(&self, input: &Variable) -> Variable {
        let shape = input.shape();
        let batch = shape[0];
        let total = shape[1];
        let seq_len = (total / self.input_features).max(1);

        let reshaped = input.reshape(&[batch, seq_len, self.input_features]);
        let rnn_out = self.rnn.forward(&reshaped);

        // Last timestep
        let last_idx = rnn_out.shape()[1] - 1;
        let last_hidden = rnn_out.select(1, last_idx);

        self.sigmoid.forward(&self.fc.forward(&last_hidden))
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = self.rnn.parameters();
        params.extend(self.fc.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::Rnn
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for RnnModel {}
unsafe impl Sync for RnnModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use axonml_tensor::Tensor;

    #[test]
    fn test_rnn_model_forward() {
        let model = RnnModel::new(5, 16, 1, 10);
        let input = Variable::new(
            Tensor::from_vec(vec![0.1f32; 50], &[2, 25]).unwrap(),
            false,
        );
        let output = model.forward(&input);
        assert_eq!(output.shape(), vec![2, 1]);
    }
}
