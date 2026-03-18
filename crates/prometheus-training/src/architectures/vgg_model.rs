// ============================================================================
// File: vgg_model.rs
// Description: VGG-11 image classification model with Conv2d feature extractor and Linear classifier
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! VGG model — powered by AxonML layers.
//!
//! Uses Sequential chains of Conv2d + ReLU + MaxPool2d, followed by Linear classifier.
//! Training: CrossEntropy loss, AdamW optimizer.

use axonml_autograd::Variable;
use axonml_nn::{Conv2d, Linear, MaxPool2d, Module, Parameter, ReLU, Sequential};

use super::TrainableModel;
use crate::Architecture;

pub struct VggModel {
    input_features: usize,
    features: Sequential,
    classifier: Sequential,
    in_channels: usize,
    img_size: usize,
    final_channels: usize,
}

impl VggModel {
    /// VGG-11 configuration: [64, 'M', 128, 'M', 256, 256, 'M', 512, 512, 'M', 512, 512, 'M']
    pub fn vgg11(in_channels: usize, num_classes: usize, img_size: usize) -> Self {
        let input_features = in_channels * img_size * img_size;

        // Feature extractor
        let features = Sequential::new()
            .add(Conv2d::new(in_channels, 64, 3))
            .add(ReLU)
            .add(MaxPool2d::new(2))
            .add(Conv2d::new(64, 128, 3))
            .add(ReLU)
            .add(MaxPool2d::new(2))
            .add(Conv2d::new(128, 256, 3))
            .add(ReLU)
            .add(Conv2d::new(256, 256, 3))
            .add(ReLU)
            .add(MaxPool2d::new(2))
            .add(Conv2d::new(256, 512, 3))
            .add(ReLU)
            .add(Conv2d::new(512, 512, 3))
            .add(ReLU)
            .add(MaxPool2d::new(2));

        // Classifier
        let classifier = Sequential::new()
            .add(Linear::new(512, 256))
            .add(ReLU)
            .add(Linear::new(256, num_classes));

        Self {
            input_features,
            features,
            classifier,
            in_channels,
            img_size,
            final_channels: 512,
        }
    }
}

impl TrainableModel for VggModel {
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

        // Global average pool to handle variable spatial dims
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
        Architecture::Vgg
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for VggModel {}
unsafe impl Sync for VggModel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vgg_creation() {
        let model = VggModel::vgg11(1, 10, 32);
        assert_eq!(model.architecture(), Architecture::Vgg);
        assert!(model.num_parameters() > 0);
    }
}
