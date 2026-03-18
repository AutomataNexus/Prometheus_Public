// ============================================================================
// File: conv_models.rs
// Description: Conv1d temporal and Conv2d spatial feature extraction models using AxonML layers
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Conv1d and Conv2d models — powered by AxonML layers.

use axonml_autograd::Variable;
use axonml_nn::{Conv1d, Conv2d, Linear, MaxPool2d, Module, Parameter, ReLU, Sequential, Sigmoid};

use super::TrainableModel;
use crate::Architecture;

// ---------------------------------------------------------------------------
// Conv1dModel — temporal feature extraction
// ---------------------------------------------------------------------------

pub struct Conv1dModel {
    input_features: usize,
    conv_layers: Vec<Conv1d>,
    fc: Linear,
    sigmoid: Sigmoid,
    hidden_dim: usize,
}

impl Conv1dModel {
    pub fn new(input_features: usize, hidden_dim: usize, num_layers: usize) -> Self {
        let mut conv_layers = Vec::new();
        let mut in_ch = input_features;
        for _ in 0..num_layers.max(1) {
            conv_layers.push(Conv1d::new(in_ch, hidden_dim, 3));
            in_ch = hidden_dim;
        }

        Self {
            input_features,
            conv_layers,
            fc: Linear::new(hidden_dim, 1),
            sigmoid: Sigmoid,
            hidden_dim,
        }
    }
}

impl TrainableModel for Conv1dModel {
    fn forward(&self, input: &Variable) -> Variable {
        let shape = input.shape();
        let batch = shape[0];
        let total = shape[1];

        // Reshape to [batch, channels=input_features, length]
        let length = (total / self.input_features).max(1);
        let mut x = input.reshape(&[batch, self.input_features, length]);

        // Conv layers with ReLU
        for conv in &self.conv_layers {
            x = conv.forward(&x);
            // Apply ReLU via element-wise max with 0
            x = x.relu();
        }

        // Global average pool over temporal dim: [batch, hidden_dim]
        let t = x.shape()[2];
        let x = x.reshape(&[batch, self.hidden_dim, t]);
        let x = x.mean_dim(2, false);

        // FC + sigmoid: [batch, 1]
        self.sigmoid.forward(&self.fc.forward(&x))
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = Vec::new();
        for conv in &self.conv_layers {
            params.extend(conv.parameters());
        }
        params.extend(self.fc.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::Conv1d
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for Conv1dModel {}
unsafe impl Sync for Conv1dModel {}

// ---------------------------------------------------------------------------
// Conv2dModel — spatial feature extraction
// ---------------------------------------------------------------------------

pub struct Conv2dModel {
    input_features: usize,
    features: Sequential,
    classifier: Linear,
    in_channels: usize,
    img_size: usize,
    final_channels: usize,
}

impl Conv2dModel {
    pub fn new(in_channels: usize, num_classes: usize, img_size: usize) -> Self {
        let input_features = in_channels * img_size * img_size;

        let features = Sequential::new()
            .add(Conv2d::new(in_channels, 32, 3))
            .add(ReLU)
            .add(MaxPool2d::new(2))
            .add(Conv2d::new(32, 64, 3))
            .add(ReLU)
            .add(MaxPool2d::new(2));

        let classifier = Linear::new(64, num_classes);

        Self {
            input_features,
            features,
            classifier,
            in_channels,
            img_size,
            final_channels: 64,
        }
    }
}

impl TrainableModel for Conv2dModel {
    fn forward(&self, input: &Variable) -> Variable {
        let shape = input.shape();
        let batch = shape[0];
        let flat_dim = shape[1];
        let target_dim = self.in_channels * self.img_size * self.img_size;

        let x = if flat_dim != target_dim {
            let mut padded = vec![0.0f32; batch * target_dim];
            let src = input.data().to_vec();
            let copy_len = flat_dim.min(target_dim);
            for b in 0..batch {
                padded[b * target_dim..b * target_dim + copy_len]
                    .copy_from_slice(&src[b * flat_dim..b * flat_dim + copy_len]);
            }
            Variable::new(
                axonml_tensor::Tensor::from_vec(padded, &[batch, target_dim]).unwrap(),
                input.requires_grad(),
            )
        } else {
            input.clone()
        };

        let x = x.reshape(&[batch, self.in_channels, self.img_size, self.img_size]);
        let x = self.features.forward(&x);

        // Global average pool
        let spatial = x.shape();
        let h = spatial.get(2).copied().unwrap_or(1);
        let w = spatial.get(3).copied().unwrap_or(1);
        let x = x.reshape(&[batch, self.final_channels, h * w]);
        let x = x.mean_dim(2, false);

        self.classifier.forward(&x)
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = self.features.parameters();
        params.extend(self.classifier.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::Conv2d
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for Conv2dModel {}
unsafe impl Sync for Conv2dModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use axonml_tensor::Tensor;

    #[test]
    fn test_conv1d_forward() {
        let model = Conv1dModel::new(5, 16, 2);
        let input = Variable::new(
            Tensor::from_vec(vec![0.1f32; 100], &[2, 50]).unwrap(),
            false,
        );
        let output = model.forward(&input);
        assert_eq!(output.shape(), vec![2, 1]);
    }

    #[test]
    fn test_conv2d_forward() {
        let model = Conv2dModel::new(1, 10, 16);
        assert_eq!(model.architecture(), Architecture::Conv2d);
        assert!(model.num_parameters() > 0);
    }
}
