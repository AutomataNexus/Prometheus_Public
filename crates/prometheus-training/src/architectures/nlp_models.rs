// ============================================================================
// File: nlp_models.rs
// Description: NLP models including BERT bidirectional encoder and GPT-2 autoregressive decoder
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! NLP models: BERT (encoder) and GPT-2 (decoder) — powered by AxonML layers.
//!
//! BERT: TransformerEncoder + Linear classifier head.
//! GPT-2: TransformerDecoder + Linear language-model head.
//! Training: CrossEntropy loss, AdamW optimizer.

use axonml_autograd::Variable;
use axonml_nn::{Linear, Module, Parameter, Sigmoid, TransformerDecoder, TransformerEncoder};
use axonml_tensor::Tensor;

use super::TrainableModel;
use crate::Architecture;

// ---------------------------------------------------------------------------
// BERT — bidirectional encoder for classification
// ---------------------------------------------------------------------------

pub struct BertModel {
    input_features: usize,
    /// Projects each input token (input_dim) → d_model.
    embed_proj: Linear,
    /// Learned positional embeddings stored as a Parameter.
    pos_embed: Parameter,
    /// Stacked TransformerEncoder (self-attention, no causal mask).
    encoder: TransformerEncoder,
    /// Classification head from [CLS] token.
    classifier: Linear,
    /// Optional sigmoid for binary classification.
    num_classes: usize,
    sigmoid: Sigmoid,
    d_model: usize,
    max_seq_len: usize,
}

impl BertModel {
    pub fn new(
        input_dim: usize,
        num_classes: usize,
        d_model: usize,
        num_heads: usize,
        num_layers: usize,
    ) -> Self {
        let ff_dim = d_model * 4;
        let max_seq_len = 512;

        // Positional embedding: [max_seq_len, d_model]
        let pos_data = Tensor::from_vec(
            vec![0.0f32; max_seq_len * d_model],
            &[max_seq_len, d_model],
        )
        .unwrap();
        let pos_embed = Parameter::new(pos_data, true);

        Self {
            input_features: input_dim,
            embed_proj: Linear::new(input_dim, d_model),
            pos_embed,
            encoder: TransformerEncoder::new(d_model, num_heads, ff_dim, num_layers),
            classifier: Linear::new(d_model, num_classes),
            num_classes,
            sigmoid: Sigmoid,
            d_model,
            max_seq_len,
        }
    }
}

impl TrainableModel for BertModel {
    fn forward(&self, input: &Variable) -> Variable {
        let shape = input.shape();
        let batch = shape[0];
        let total = shape[1];

        // Interpret flat input as [batch, seq_len, input_dim]
        let seq_len = (total / self.input_features).max(1).min(self.max_seq_len);
        let x = input.reshape(&[batch, seq_len, self.input_features]);

        // Project to d_model: [batch, seq_len, d_model]
        let x = self.embed_proj.forward(&x);

        // Add positional embedding (broadcast over batch)
        let pos = Variable::new(self.pos_embed.data().clone(), false);
        let pos = pos.reshape(&[1, self.max_seq_len, self.d_model]);
        // Slice positional embeddings to actual sequence length
        let pos_slice = pos.narrow(1, 0, seq_len); // [1, seq_len, d_model]
        let x = x.add_var(&pos_slice);

        // TransformerEncoder: [batch, seq_len, d_model]
        let encoded = self.encoder.forward(&x);

        // Use first token ([CLS] equivalent): [batch, d_model]
        let cls = encoded.select(1, 0);

        // Classification head
        let logits = self.classifier.forward(&cls);

        if self.num_classes == 1 {
            self.sigmoid.forward(&logits)
        } else {
            logits
        }
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = self.embed_proj.parameters();
        params.push(self.pos_embed.clone());
        params.extend(self.encoder.parameters());
        params.extend(self.classifier.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::Bert
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for BertModel {}
unsafe impl Sync for BertModel {}

// ---------------------------------------------------------------------------
// GPT-2 — autoregressive decoder for generation
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct Gpt2Model {
    input_features: usize,
    /// Projects each input token (input_dim) → d_model.
    embed_proj: Linear,
    /// Learned positional embeddings.
    pos_embed: Parameter,
    /// Stacked TransformerDecoder (causal self-attention).
    decoder: TransformerDecoder,
    /// Language model head: d_model → output_dim.
    lm_head: Linear,
    d_model: usize,
    output_dim: usize,
    max_seq_len: usize,
}

impl Gpt2Model {
    pub fn new(
        input_dim: usize,
        output_dim: usize,
        d_model: usize,
        num_heads: usize,
        num_layers: usize,
    ) -> Self {
        let ff_dim = d_model * 4;
        let max_seq_len = 512;

        let pos_data = Tensor::from_vec(
            vec![0.0f32; max_seq_len * d_model],
            &[max_seq_len, d_model],
        )
        .unwrap();
        let pos_embed = Parameter::new(pos_data, true);

        Self {
            input_features: input_dim,
            embed_proj: Linear::new(input_dim, d_model),
            pos_embed,
            decoder: TransformerDecoder::new(d_model, num_heads, ff_dim, num_layers),
            lm_head: Linear::new(d_model, output_dim),
            d_model,
            output_dim,
            max_seq_len,
        }
    }
}

impl TrainableModel for Gpt2Model {
    fn forward(&self, input: &Variable) -> Variable {
        let shape = input.shape();
        let batch = shape[0];
        let total = shape[1];

        let seq_len = (total / self.input_features).max(1).min(self.max_seq_len);
        let x = input.reshape(&[batch, seq_len, self.input_features]);

        // Project to d_model
        let x = self.embed_proj.forward(&x);

        // Add positional embedding
        let pos = Variable::new(self.pos_embed.data().clone(), false);
        let pos = pos.reshape(&[1, self.max_seq_len, self.d_model]);
        let pos_slice = pos.narrow(1, 0, seq_len);
        let x = x.add_var(&pos_slice);

        // TransformerDecoder (no memory — runs as decoder-only / causal LM)
        let decoded = self.decoder.forward(&x);

        // LM head on last token: [batch, d_model] → [batch, output_dim]
        let last_idx = decoded.shape()[1] - 1;
        let last_hidden = decoded.select(1, last_idx);

        self.lm_head.forward(&last_hidden)
    }

    fn parameters(&self) -> Vec<Parameter> {
        let mut params = self.embed_proj.parameters();
        params.push(self.pos_embed.clone());
        params.extend(self.decoder.parameters());
        params.extend(self.lm_head.parameters());
        params
    }

    fn architecture(&self) -> Architecture {
        Architecture::Gpt2
    }

    fn input_features(&self) -> usize {
        self.input_features
    }
}

unsafe impl Send for Gpt2Model {}
unsafe impl Sync for Gpt2Model {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bert_creation() {
        let model = BertModel::new(8, 3, 32, 4, 2);
        assert_eq!(model.architecture(), Architecture::Bert);
        assert_eq!(model.input_features(), 8);
        assert!(model.num_parameters() > 0);
    }

    #[test]
    fn test_gpt2_creation() {
        let model = Gpt2Model::new(8, 10, 32, 4, 2);
        assert_eq!(model.architecture(), Architecture::Gpt2);
        assert_eq!(model.input_features(), 8);
        assert!(model.num_parameters() > 0);
    }
}
