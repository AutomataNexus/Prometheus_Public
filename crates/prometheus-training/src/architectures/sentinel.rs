// ============================================================================
// File: sentinel.rs
// Description: Sentinel MLP health scorer producing a 0.0-1.0 facility health score
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Sentinel MLP Health Scorer — powered by AxonML layers.
//!
//! Architecture:
//! - Linear(input_features, 128) -> ReLU
//! - Linear(128, 64) -> ReLU
//! - Linear(64, 1) -> Sigmoid
//!
//! Produces a single health score in [0.0, 1.0].
//! Training: BCE loss, Adam optimizer.

use axonml_autograd::Variable;
use axonml_nn::{Linear, Module, Parameter, ReLU, Sequential, Sigmoid};

use super::TrainableModel;
use crate::Architecture;

/// Sentinel MLP health scorer using AxonML layers.
pub struct Sentinel {
    input_features: usize,
    model: Sequential,
}

impl Sentinel {
    /// Create a new Sentinel health scorer.
    pub fn new(input_features: usize) -> Self {
        let model = Sequential::new()
            .add(Linear::new(input_features, 128))
            .add(ReLU)
            .add(Linear::new(128, 64))
            .add(ReLU)
            .add(Linear::new(64, 1))
            .add(Sigmoid);

        Self {
            input_features,
            model,
        }
    }
}

impl TrainableModel for Sentinel {
    fn forward(&self, input: &Variable) -> Variable {
        self.model.forward(input)
    }

    fn parameters(&self) -> Vec<Parameter> {
        self.model.parameters()
    }

    fn architecture(&self) -> Architecture {
        Architecture::Sentinel
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

// Implement Send + Sync (Sequential is already Send + Sync via AxonML)
unsafe impl Send for Sentinel {}
unsafe impl Sync for Sentinel {}

#[cfg(test)]
mod tests {
    use super::*;
    use axonml_tensor::Tensor;

    #[test]
    fn test_sentinel_creation() {
        let model = Sentinel::new(10);
        assert_eq!(model.input_features(), 10);
        assert_eq!(model.architecture(), Architecture::Sentinel);
        // Should have parameters from 3 Linear layers
        assert!(model.num_parameters() > 0);
    }

    #[test]
    fn test_sentinel_forward() {
        let model = Sentinel::new(5);
        let input = Variable::new(
            Tensor::from_vec(vec![0.5f32; 10], &[2, 5]).unwrap(),
            false,
        );
        let output = model.forward(&input);
        // Output shape: [batch=2, 1]
        assert_eq!(output.shape(), vec![2, 1]);
        // Sigmoid output should be in [0, 1]
        let vals = output.data().to_vec();
        for &v in &vals {
            assert!(v >= 0.0 && v <= 1.0, "output {} not in [0,1]", v);
        }
    }

    #[test]
    fn test_sentinel_flat_parameters() {
        let model = Sentinel::new(5);
        let flat = model.flat_parameters();
        assert_eq!(flat.len(), model.num_parameters());
    }
}
