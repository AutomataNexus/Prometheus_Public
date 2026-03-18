// ============================================================================
// File: resnet_model.rs
// Description: ResNet-18 image classification model with residual blocks and batch normalization
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! ResNet model — powered by AxonML layers.
//!
//! Uses Conv2d, BatchNorm2d, ResidualBlock, AdaptiveAvgPool2d, Linear.
//! Training: CrossEntropy loss, AdamW optimizer.

use axonml_autograd::Variable;
use axonml_nn::{
    AdaptiveAvgPool2d, BatchNorm2d, Conv2d, Linear, MaxPool2d, Module, Parameter, ReLU,
    ResidualBlock, Sequential,
};

use super::TrainableModel;
use crate::Architecture;

#[allow(dead_code)]
pub struct ResNetModel {
    input_features: usize,
    // Initial conv + bn
    conv1: Conv2d,
    bn1: BatchNorm2d,
    relu: ReLU,
    pool: MaxPool2d,
    // Residual blocks
    blocks: Vec<ResidualBlock>,
    // Classification head
    avgpool: AdaptiveAvgPool2d,
    fc: Linear,
    num_classes: usize,
    in_channels: usize,
    img_size: usize,
}

impl ResNetModel {
    pub fn resnet18(in_channels: usize, num_classes: usize, img_size: usize) -> Self {
        let input_features = in_channels * img_size * img_size;

        // Initial conv: in_channels -> 64
        let conv1 = Conv2d::new(in_channels, 64, 3);
        let bn1 = BatchNorm2d::new(64);

        // 4 groups of 2 residual blocks each: 64->64, 64->128, 128->256, 256->512
        let mut blocks = Vec::new();
        let channels = [64, 128, 256, 512];
        let mut in_ch = 64;
        for &out_ch in &channels {
            // First block: may change channels (with downsample)
            let main1 = Sequential::new()
                .add(Conv2d::new(in_ch, out_ch, 3))
                .add(BatchNorm2d::new(out_ch))
                .add(ReLU)
                .add(Conv2d::new(out_ch, out_ch, 3))
                .add(BatchNorm2d::new(out_ch));
            let block1 = if in_ch != out_ch {
                let ds = Sequential::new()
                    .add(Conv2d::new(in_ch, out_ch, 1))
                    .add(BatchNorm2d::new(out_ch));
                ResidualBlock::new(main1).with_downsample(ds)
            } else {
                ResidualBlock::new(main1)
            };
            blocks.push(block1);

            // Second block: same channels
            let main2 = Sequential::new()
                .add(Conv2d::new(out_ch, out_ch, 3))
                .add(BatchNorm2d::new(out_ch))
                .add(ReLU)
                .add(Conv2d::new(out_ch, out_ch, 3))
                .add(BatchNorm2d::new(out_ch));
            blocks.push(ResidualBlock::new(main2));

            in_ch = out_ch;
        }

        let avgpool = AdaptiveAvgPool2d::new((1, 1));
        let fc = Linear::new(512, num_classes);

        Self {
            input_features,
            conv1,
            bn1,
            relu: ReLU,
            pool: MaxPool2d::new(2),
            blocks,
            avgpool,
            fc,
            num_classes,
            in_channels,
            img_size,
        }
    }
}

impl TrainableModel for ResNetModel {
    fn forward(&self, input: &Variable) -> Variable {
        let shape = input.shape();
        let batch = shape[0];
        let flat_dim = shape[1];
        let target_dim = self.in_channels * self.img_size * self.img_size;

        // Pad or truncate flat input to match img_size grid
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

        // Reshape to [batch, channels, h, w]
        let x = x.reshape(&[batch, self.in_channels, self.img_size, self.img_size]);

        // Initial conv -> bn -> relu -> pool
        let x = self.relu.forward(&self.bn1.forward(&self.conv1.forward(&x)));
        let mut x = self.pool.forward(&x);

        // Residual blocks
        for block in &self.blocks {
            x = block.forward(&x);
        }

        // Global average pool -> flatten -> fc
        let x = self.avgpool.forward(&x);
        let x = x.reshape(&[batch, 512]);
        self.fc.forward(&x)
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = self.conv1.parameters();
        params.extend(self.bn1.parameters());
        for block in &self.blocks {
            params.extend(block.parameters());
        }
        params.extend(self.fc.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::ResNet
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for ResNetModel {}
unsafe impl Sync for ResNetModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use axonml_tensor::Tensor;

    #[test]
    fn test_resnet_creation() {
        let model = ResNetModel::resnet18(1, 10, 32);
        assert_eq!(model.architecture(), Architecture::ResNet);
        assert!(model.num_parameters() > 0);
    }
}
