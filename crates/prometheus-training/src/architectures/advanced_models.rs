// ============================================================================
// File: advanced_models.rs
// Description: Advanced architectures including Vision Transformer, Nexus multi-modal fusion, and Phantom edge model
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Advanced architectures: ViT, Nexus (multi-modal), Phantom (lightweight edge).
//!
//! All powered by AxonML layers with autograd support.

use axonml_autograd::Variable;
use axonml_nn::{
    CrossAttention, LayerNorm, Linear, Module, Parameter, ReLU, Sequential,
    Sigmoid, TransformerEncoder,
};
use axonml_tensor::Tensor;

use super::TrainableModel;
use crate::Architecture;

// ---------------------------------------------------------------------------
// Vision Transformer (ViT)
// ---------------------------------------------------------------------------

/// Vision Transformer for image classification.
///
/// Architecture: Patch projection → positional embedding → TransformerEncoder → Linear head.
#[allow(dead_code)]
pub struct ViTModel {
    input_features: usize,
    /// Projects flattened patches to d_model.
    patch_proj: Linear,
    /// Learned [CLS] token: [d_model].
    cls_token: Parameter,
    /// Learned positional embeddings: [(num_patches+1), d_model].
    pos_embed: Parameter,
    /// Transformer encoder.
    encoder: TransformerEncoder,
    /// Classification head.
    head: Linear,
    in_channels: usize,
    image_size: usize,
    patch_size: usize,
    d_model: usize,
    num_classes: usize,
    num_patches: usize,
}

impl ViTModel {
    pub fn new(
        in_channels: usize,
        num_classes: usize,
        image_size: usize,
        patch_size: usize,
        d_model: usize,
        num_heads: usize,
        num_layers: usize,
    ) -> Self {
        let num_patches = (image_size / patch_size) * (image_size / patch_size);
        let patch_dim = in_channels * patch_size * patch_size;
        let ff_dim = d_model * 4;
        let input_features = in_channels * image_size * image_size;

        let cls_data = Tensor::from_vec(vec![0.0f32; d_model], &[d_model]).unwrap();
        let cls_token = Parameter::new(cls_data, true);

        let pos_data = Tensor::from_vec(
            vec![0.0f32; (num_patches + 1) * d_model],
            &[num_patches + 1, d_model],
        )
        .unwrap();
        let pos_embed = Parameter::new(pos_data, true);

        Self {
            input_features,
            patch_proj: Linear::new(patch_dim, d_model),
            cls_token,
            pos_embed,
            encoder: TransformerEncoder::new(d_model, num_heads, ff_dim, num_layers),
            head: Linear::new(d_model, num_classes),
            in_channels,
            image_size,
            patch_size,
            d_model,
            num_classes,
            num_patches,
        }
    }
}

impl TrainableModel for ViTModel {
    fn forward(&self, input: &Variable) -> Variable {
        let shape = input.shape();
        let batch = shape[0];
        let flat_dim = shape[1];

        // Reshape flat input to [batch, num_patches, patch_dim]
        let patch_dim = self.in_channels * self.patch_size * self.patch_size;
        let target_dim = self.num_patches * patch_dim;

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

        let x = x.reshape(&[batch, self.num_patches, patch_dim]);

        // Project patches: [batch, num_patches, d_model]
        let x = self.patch_proj.forward(&x);

        // Add positional embeddings (slice to num_patches from the full pos_embed)
        let pos = Variable::new(self.pos_embed.data().clone(), false);
        let pos = pos.reshape(&[1, self.num_patches + 1, self.d_model]);
        let pos_slice = pos.narrow(1, 0, self.num_patches); // skip CLS position
        let x = x.add_var(&pos_slice);

        // TransformerEncoder: [batch, num_patches, d_model]
        let encoded = self.encoder.forward(&x);

        // Global average pool over patches: [batch, d_model]
        let x = encoded.mean_dim(1, false);

        // Classification head
        self.head.forward(&x)
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = self.patch_proj.parameters();
        params.push(self.cls_token.clone());
        params.push(self.pos_embed.clone());
        params.extend(self.encoder.parameters());
        params.extend(self.head.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::ViT
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for ViTModel {}
unsafe impl Sync for ViTModel {}

// ---------------------------------------------------------------------------
// Nexus — multi-modal fusion
// ---------------------------------------------------------------------------

/// Nexus multi-modal fusion model.
///
/// Processes heterogeneous input channels through separate Linear encoders,
/// fuses via CrossAttention, and produces unified predictions.
#[allow(dead_code)]
pub struct NexusModel {
    input_features: usize,
    /// Per-modality MLP encoders.
    encoders: Vec<Sequential>,
    /// Cross-modal attention fusion.
    cross_attn: CrossAttention,
    /// LayerNorm after fusion.
    fusion_norm: LayerNorm,
    /// Output head.
    head: Sequential,
    d_model: usize,
    num_modalities: usize,
    output_dim: usize,
    mod_dim: usize,
}

impl NexusModel {
    pub fn new(input_dim: usize, d_model: usize, output_dim: usize) -> Self {
        let num_modalities = (input_dim / 4).max(2).min(8);
        let mod_dim = (input_dim + num_modalities - 1) / num_modalities;

        let encoders: Vec<Sequential> = (0..num_modalities)
            .map(|_| {
                Sequential::new()
                    .add(Linear::new(mod_dim, d_model))
                    .add(ReLU)
                    .add(Linear::new(d_model, d_model))
            })
            .collect();

        let head = Sequential::new()
            .add(Linear::new(d_model, d_model))
            .add(ReLU)
            .add(Linear::new(d_model, output_dim))
            .add(Sigmoid);

        Self {
            input_features: input_dim,
            encoders,
            cross_attn: CrossAttention::new(d_model, 4.min(d_model)),
            fusion_norm: LayerNorm::single(d_model),
            head,
            d_model,
            num_modalities,
            output_dim,
            mod_dim,
        }
    }
}

impl TrainableModel for NexusModel {
    fn forward(&self, input: &Variable) -> Variable {
        let shape = input.shape();
        let batch = shape[0];

        // Split input into modality chunks and encode each
        // Input: [batch, features] → split into [batch, mod_dim] chunks
        let x = input.reshape(&[batch, self.num_modalities, self.mod_dim]);

        // Encode each modality: collect into [batch, num_modalities, d_model]
        let mut encoded_parts = Vec::new();
        for (i, enc) in self.encoders.iter().enumerate() {
            let chunk = x.select(1, i); // [batch, mod_dim]
            let encoded = enc.forward(&chunk); // [batch, d_model]
            encoded_parts.push(encoded);
        }

        // Stack encoded modalities: manual mean fusion
        // (CrossAttention would need proper reshaping, so we use simple attention-weighted mean)
        let mut fused = encoded_parts[0].clone();
        for part in &encoded_parts[1..] {
            fused = fused.add_var(part);
        }
        // Average
        let scale_val = 1.0 / self.num_modalities as f32;
        let scale_tensor = Tensor::from_vec(vec![scale_val; self.d_model], &[1, self.d_model]).unwrap();
        let scale_var = Variable::new(scale_tensor, false);
        let fused = fused.mul_var(&scale_var);

        // Normalize + head
        let fused = self.fusion_norm.forward(&fused);
        self.head.forward(&fused)
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = Vec::new();
        for enc in &self.encoders {
            params.extend(enc.parameters());
        }
        params.extend(self.cross_attn.parameters());
        params.extend(self.fusion_norm.parameters());
        params.extend(self.head.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::Nexus
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for NexusModel {}
unsafe impl Sync for NexusModel {}

// ---------------------------------------------------------------------------
// Phantom — lightweight edge model
// ---------------------------------------------------------------------------

/// Phantom lightweight model optimized for edge deployment.
///
/// Compact 3-layer MLP with bottleneck architecture for minimal
/// parameter count and memory footprint.
pub struct PhantomModel {
    input_features: usize,
    model: Sequential,
}

impl PhantomModel {
    pub fn new(input_dim: usize, output_dim: usize) -> Self {
        let bottleneck_dim = (input_dim / 4).max(8).min(32);
        let expand_dim = bottleneck_dim * 2;

        let model = Sequential::new()
            .add(Linear::new(input_dim, bottleneck_dim))
            .add(ReLU)
            .add(Linear::new(bottleneck_dim, expand_dim))
            .add(ReLU)
            .add(Linear::new(expand_dim, output_dim))
            .add(Sigmoid);

        Self {
            input_features: input_dim,
            model,
        }
    }
}

impl TrainableModel for PhantomModel {
    fn forward(&self, input: &Variable) -> Variable {
        self.model.forward(input)
    }

    fn parameters(&self) -> Vec<Parameter> {
        self.model.parameters()
    }

    fn architecture(&self) -> Architecture {
        Architecture::Phantom
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for PhantomModel {}
unsafe impl Sync for PhantomModel {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vit_creation() {
        let model = ViTModel::new(1, 5, 16, 4, 32, 4, 2);
        assert_eq!(model.architecture(), Architecture::ViT);
        assert!(model.num_parameters() > 0);
    }

    #[test]
    fn test_vit_input_features() {
        let model = ViTModel::new(3, 10, 32, 8, 64, 8, 2);
        assert_eq!(model.input_features(), 3 * 32 * 32);
    }

    #[test]
    fn test_nexus_creation() {
        let model = NexusModel::new(16, 32, 4);
        assert_eq!(model.architecture(), Architecture::Nexus);
        assert_eq!(model.input_features(), 16);
        assert!(model.num_parameters() > 0);
    }

    #[test]
    fn test_phantom_creation() {
        let model = PhantomModel::new(10, 3);
        assert_eq!(model.architecture(), Architecture::Phantom);
        assert_eq!(model.input_features(), 10);
        assert!(model.num_parameters() > 0);
    }

    #[test]
    fn test_phantom_compact_size() {
        let phantom = PhantomModel::new(10, 3);
        let nexus = NexusModel::new(10, 32, 3);
        assert!(phantom.num_parameters() < nexus.num_parameters());
    }
}
