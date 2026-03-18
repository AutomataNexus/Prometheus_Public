# AxonML API Reference

Reference documentation for the AxonML framework (v0.4.1) used by the
Prometheus platform for model training, evaluation, and deployment across
13 neural-network architectures.

---

## Overview

AxonML is a pure-Rust ML framework (v0.4.1) with GPU/CUDA support that powers
Prometheus' training pipeline. It provides an **autograd engine** for real
backpropagation (single forward + backward pass per batch), **nn layers**
(Linear, LSTM, GRU, RNN, Conv1d, Conv2d, BatchNorm2d, TransformerEncoder,
TransformerDecoder, ResidualBlock, MultiHeadAttention, CrossAttention,
Sequential), **optimizers** (Adam, AdamW with weight_decay), and **loss
functions** (MSELoss, BCELoss, CrossEntropyLoss).

Prometheus architectures implement the `TrainableModel` trait:
- `forward(&self, input: &Variable) -> Variable` -- defines the forward pass
  using AxonML nn layers, building a computation graph via `Variable` tensors
- `parameters(&self) -> Vec<Parameter>` -- returns all trainable parameters
  for optimizer updates after backpropagation

The training pipeline runs 5 stages: **validate -> preprocess -> train ->
evaluate -> export**.

### Performance Optimizations

AxonML 0.4.1 includes significant performance optimizations:

- **Rayon-parallel Conv1d/Conv2d**: Forward and backward passes parallelize across batch samples
- **Fused im2col**: 5-level nested loops → 3-level with arithmetic decomposition and `unsafe get_unchecked` for bounds elimination
- **CUDA full-GPU pipeline**: Conv2d backward uses GPU-resident im2col → cuBLAS GEMM, auto-activates when tensors are on GPU device
- **Hoisted RNN/LSTM/GRU weight transposes**: Weight transpose computed once before the per-timestep loop; input-hidden projection as single batched GEMM
- **matrixmultiply threading**: All CPU BLAS GEMM operations threaded via Rayon

**Benchmark**: 8.3 → 14.2 img/s on CPU (71% speedup). 1,018 tests passing (105 autograd + 171 nn + 704 vision + 38 HVAC + 83 LLM).

**Base URL:** `https://api.prometheus.example.com/axonml/v1`

**Authentication:** Bearer token via `Authorization` header.

### Key AxonML Crates Used by Prometheus

| Crate | Purpose |
|-------|---------|
| `axonml-autograd` | Computation graph, `Variable`, `Parameter`, backward pass |
| `axonml-nn` | Neural network layers (Linear, LSTM, GRU, Conv1d, Conv2d, etc.) |
| `axonml-optim` | Optimizers (Adam, AdamW with weight_decay) |
| `axonml-core` | Loss functions (MSELoss, BCELoss, CrossEntropyLoss), activations |
| `axonml-tensor` | Tensor operations (CPU/GPU) |
| `axonml-serialize` | Model serialization (.axonml format) |
| `axonml-quant` | INT8 quantization for edge deployment |
| `axonml-onnx` | ONNX export |
| `axonml-data` | Data loading and batching |
| `axonml-vision` | Vision-specific utilities |
| `axonml-text` | Text/NLP utilities |

---

## Architecture Schema

Every architecture is described by an `ArchitectureSpec`:

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: <string>              # unique identifier
  equipment_type: <string>    # e.g. "chiller", "ahu"
  description: <string>
  tags:
    <key>: <value>

spec:
  input:
    sequence_length: <int>    # look-back window (timesteps)
    feature_count: <int>      # number of input channels
    dtype: float32            # input data type

  layers:
    - type: <layer_type>
      params:
        <key>: <value>

  output:
    type: reconstruction | prediction | classification
    dim: <int>                # output dimension

  training:
    loss: mse | mae | focal | binary_crossentropy
    optimizer: adam | adamw | sgd
    learning_rate: <float>
    batch_size: <int>
    epochs: <int>
    early_stopping:
      patience: <int>
      monitor: val_loss
    scheduler:
      type: cosine | step | plateau
      params:
        <key>: <value>
```

---

## Supported Layer Types

### `lstm`
Recurrent layer using Long Short-Term Memory cells.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| hidden_dim | int | 64 | Hidden state dimension |
| num_layers | int | 1 | Number of stacked LSTM layers |
| bidirectional | bool | false | Use bidirectional LSTM |
| dropout | float | 0.0 | Dropout between layers |
| return_sequences | bool | true | Return full sequence or last step |

### `gru`
Recurrent layer using Gated Recurrent Units.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| hidden_dim | int | 64 | Hidden state dimension |
| num_layers | int | 1 | Number of stacked GRU layers |
| bidirectional | bool | false | Use bidirectional GRU |
| dropout | float | 0.0 | Dropout between layers |
| return_sequences | bool | true | Return full sequence or last step |

### `conv1d`
1-D convolutional layer for temporal feature extraction.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| filters | int | 64 | Number of convolutional filters |
| kernel_size | int | 3 | Kernel width |
| stride | int | 1 | Convolution stride |
| padding | str | "same" | Padding mode: "same" or "valid" |
| activation | str | "relu" | Activation function |
| batch_norm | bool | true | Apply batch normalisation |

### `attention`
Multi-head self-attention layer.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| num_heads | int | 4 | Number of attention heads |
| key_dim | int | 64 | Dimension of key/query projections |
| dropout | float | 0.0 | Attention dropout |
| causal | bool | false | Use causal (autoregressive) mask |

### `dense`
Fully-connected (linear) layer.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| units | int | 64 | Output dimension |
| activation | str | "relu" | Activation function |
| dropout | float | 0.0 | Dropout rate |

### `repeat_vector`
Repeats the input *n* times to bridge encoder and decoder.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| n | int | required | Number of repetitions (= sequence_length) |

### `time_distributed`
Wraps a layer to apply it independently to each timestep.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| layer | object | required | Nested layer spec |

### `conv2d`
2-D convolutional layer for spatial feature extraction.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| filters | int | 64 | Number of convolutional filters |
| kernel_size | int | 3 | Kernel height/width |
| stride | int | 1 | Convolution stride |
| padding | str | "same" | Padding mode |
| activation | str | "relu" | Activation function |
| batch_norm | bool | true | Apply batch normalisation |

### `transformer_block`
Transformer encoder/decoder block with multi-head attention and feed-forward network.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| d_model | int | 256 | Model dimension |
| num_heads | int | 8 | Number of attention heads |
| ff_dim | int | 1024 | Feed-forward hidden dimension |
| dropout | float | 0.1 | Dropout rate |
| causal | bool | false | Use causal mask (for GPT-2 style) |
| activation | str | "gelu" | Feed-forward activation |

### `patch_embed`
Splits an image into patches and projects each to a vector (for ViT).

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| patch_size | int | 16 | Patch height/width in pixels |
| embed_dim | int | 256 | Output embedding dimension |
| in_channels | int | 3 | Input image channels |

### `residual_block`
Residual connection block with optional downsampling (for ResNet).

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| filters | int | 64 | Number of filters |
| stride | int | 1 | Stride (2 for downsampling) |
| batch_norm | bool | true | Apply batch normalisation |

---

## Pre-built Architecture Templates

### LSTM Autoencoder

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: lstm_autoencoder_default
  equipment_type: generic
spec:
  input:
    sequence_length: 48
    feature_count: 8
    dtype: float32
  layers:
    # Encoder
    - type: lstm
      params: { hidden_dim: 128, num_layers: 1, return_sequences: true, dropout: 0.2 }
    - type: lstm
      params: { hidden_dim: 64, num_layers: 1, return_sequences: false, dropout: 0.2 }
    # Latent
    - type: dense
      params: { units: 32, activation: relu }
    # Decoder
    - type: repeat_vector
      params: { n: 48 }
    - type: lstm
      params: { hidden_dim: 64, num_layers: 1, return_sequences: true, dropout: 0.2 }
    - type: lstm
      params: { hidden_dim: 128, num_layers: 1, return_sequences: true, dropout: 0.2 }
    - type: time_distributed
      params:
        layer:
          type: dense
          params: { units: 8, activation: linear }
  output:
    type: reconstruction
    dim: 8
  training:
    loss: mse
    optimizer: adam
    learning_rate: 0.001
    batch_size: 64
    epochs: 100
    early_stopping: { patience: 10, monitor: val_loss }
    scheduler: { type: plateau, params: { factor: 0.5, patience: 5 } }
```

### GRU Predictor

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: gru_predictor_default
  equipment_type: generic
spec:
  input:
    sequence_length: 24
    feature_count: 6
    dtype: float32
  layers:
    - type: gru
      params: { hidden_dim: 64, num_layers: 2, return_sequences: false, dropout: 0.1 }
    - type: dense
      params: { units: 32, activation: relu, dropout: 0.1 }
    - type: dense
      params: { units: 1, activation: linear }
  output:
    type: prediction
    dim: 1
  training:
    loss: mse
    optimizer: adam
    learning_rate: 0.001
    batch_size: 128
    epochs: 80
    early_stopping: { patience: 10, monitor: val_loss }
    scheduler: { type: cosine, params: { T_max: 80 } }
```

### Sentinel (Hybrid CNN-Attention)

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: sentinel_default
  equipment_type: generic
spec:
  input:
    sequence_length: 168
    feature_count: 14
    dtype: float32
  layers:
    # Feature extraction
    - type: conv1d
      params: { filters: 64, kernel_size: 7, stride: 1, activation: relu, batch_norm: true }
    - type: conv1d
      params: { filters: 128, kernel_size: 5, stride: 1, activation: relu, batch_norm: true }
    # Sequence modelling
    - type: attention
      params: { num_heads: 8, key_dim: 64, dropout: 0.1 }
    - type: lstm
      params: { hidden_dim: 256, num_layers: 1, return_sequences: false, dropout: 0.2 }
    # Classification head
    - type: dense
      params: { units: 128, activation: relu, dropout: 0.2 }
    - type: dense
      params: { units: 1, activation: sigmoid }
  output:
    type: classification
    dim: 1
  training:
    loss: focal
    optimizer: adamw
    learning_rate: 0.0005
    batch_size: 32
    epochs: 150
    early_stopping: { patience: 15, monitor: val_loss }
    scheduler: { type: cosine, params: { T_max: 150 } }
```

### ResNet Image Classifier

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: resnet_default
  description: ResNet-18 image classifier
spec:
  input:
    image_size: 32
    channels: 3
  layers:
    - type: conv2d
      params: { filters: 64, kernel_size: 7, stride: 2, batch_norm: true }
    - type: residual_block
      params: { filters: 64, stride: 1 }
    - type: residual_block
      params: { filters: 128, stride: 2 }
    - type: residual_block
      params: { filters: 256, stride: 2 }
    - type: residual_block
      params: { filters: 512, stride: 2 }
    - type: dense
      params: { units: 10, activation: softmax }
  output:
    type: classification
    dim: 10
  training:
    loss: cross_entropy
    optimizer: adamw
    learning_rate: 0.0003
    batch_size: 32
    epochs: 100
    early_stopping: { patience: 10, monitor: val_loss }
```

### VGG Image Classifier

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: vgg11_default
  description: VGG-11 image classifier
spec:
  input:
    image_size: 32
    channels: 3
  layers:
    - type: conv2d
      params: { filters: 64, kernel_size: 3 }
    - type: conv2d
      params: { filters: 128, kernel_size: 3 }
    - type: conv2d
      params: { filters: 256, kernel_size: 3 }
    - type: conv2d
      params: { filters: 512, kernel_size: 3 }
    - type: dense
      params: { units: 512, activation: relu }
    - type: dense
      params: { units: 10, activation: softmax }
  output:
    type: classification
    dim: 10
  training:
    loss: cross_entropy
    optimizer: adamw
    learning_rate: 0.0003
    batch_size: 32
    epochs: 100
```

### Vision Transformer (ViT)

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: vit_default
  description: Vision Transformer for image classification
spec:
  input:
    image_size: 32
    channels: 3
  layers:
    - type: patch_embed
      params: { patch_size: 4, embed_dim: 256, in_channels: 3 }
    - type: transformer_block
      params: { d_model: 256, num_heads: 8, ff_dim: 512, dropout: 0.1 }
    - type: transformer_block
      params: { d_model: 256, num_heads: 8, ff_dim: 512, dropout: 0.1 }
    - type: dense
      params: { units: 10, activation: softmax }
  output:
    type: classification
    dim: 10
  training:
    loss: cross_entropy
    optimizer: adamw
    learning_rate: 0.0001
    batch_size: 32
    epochs: 100
```

### BERT Text Classifier

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: bert_default
  description: BERT for text/sequence classification
spec:
  input:
    sequence_length: 128
    vocab_size: 30522
  layers:
    - type: dense
      params: { units: 256, activation: linear }  # token embedding
    - type: transformer_block
      params: { d_model: 256, num_heads: 8, ff_dim: 512, causal: false }
    - type: transformer_block
      params: { d_model: 256, num_heads: 8, ff_dim: 512, causal: false }
    - type: dense
      params: { units: 2, activation: softmax }   # [CLS] classification
  output:
    type: classification
    dim: 2
  training:
    loss: cross_entropy
    optimizer: adamw
    learning_rate: 0.00005
    batch_size: 16
    epochs: 50
```

### GPT-2 Language Model

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: gpt2_default
  description: GPT-2 causal language model
spec:
  input:
    sequence_length: 128
    vocab_size: 30522
  layers:
    - type: dense
      params: { units: 256, activation: linear }  # token embedding
    - type: transformer_block
      params: { d_model: 256, num_heads: 8, ff_dim: 512, causal: true }
    - type: transformer_block
      params: { d_model: 256, num_heads: 8, ff_dim: 512, causal: true }
    - type: dense
      params: { units: 30522, activation: softmax }  # LM head
  output:
    type: prediction
    dim: 30522
  training:
    loss: cross_entropy
    optimizer: adamw
    learning_rate: 0.0001
    batch_size: 16
    epochs: 50
```

### Vanilla RNN

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: rnn_default
  description: Simple RNN for temporal patterns
spec:
  input:
    sequence_length: 30
    feature_count: 8
  layers:
    - type: rnn
      params: { hidden_dim: 64, num_layers: 2 }
    - type: dense
      params: { units: 1, activation: sigmoid }
  output:
    type: prediction
    dim: 1
  training:
    loss: mse
    optimizer: adam
    learning_rate: 0.001
    batch_size: 64
    epochs: 100
```

### Conv1D Temporal

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: conv1d_default
  description: 1D CNN for temporal feature extraction
spec:
  input:
    sequence_length: 60
    feature_count: 8
  layers:
    - type: conv1d
      params: { filters: 32, kernel_size: 3 }
    - type: conv1d
      params: { filters: 64, kernel_size: 3 }
    - type: dense
      params: { units: 1, activation: sigmoid }
  output:
    type: prediction
    dim: 1
  training:
    loss: mse
    optimizer: adam
    learning_rate: 0.001
    batch_size: 64
    epochs: 100
```

### Conv2D Spatial

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: conv2d_default
  description: 2D CNN for image classification
spec:
  input:
    image_size: 32
    channels: 3
  layers:
    - type: conv2d
      params: { filters: 32, kernel_size: 3 }
    - type: conv2d
      params: { filters: 64, kernel_size: 3 }
    - type: conv2d
      params: { filters: 128, kernel_size: 3 }
    - type: dense
      params: { units: 10, activation: softmax }
  output:
    type: classification
    dim: 10
  training:
    loss: cross_entropy
    optimizer: adamw
    learning_rate: 0.0003
    batch_size: 32
    epochs: 100
```

### Nexus Multimodal

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: nexus_default
  description: Multimodal fusion with cross-attention
spec:
  input:
    modalities: [sensor, image, text]
    feature_dims: [32, 128, 64]
  layers:
    - type: dense
      params: { units: 128, activation: relu }  # per-modality encoder
    - type: attention
      params: { num_heads: 4, key_dim: 128 }    # cross-modal fusion
    - type: dense
      params: { units: 1, activation: sigmoid }
  output:
    type: prediction
    dim: 1
  training:
    loss: mse
    optimizer: adamw
    learning_rate: 0.0003
    batch_size: 32
    epochs: 100
```

### Phantom Edge Model

```yaml
kind: ArchitectureSpec
version: v1
metadata:
  name: phantom_default
  description: Ultra-lightweight model for microcontrollers
spec:
  input:
    feature_count: 8
  layers:
    - type: dense
      params: { units: 32, activation: relu6 }
    - type: dense
      params: { units: 16, activation: relu6 }
    - type: dense
      params: { units: 1, activation: sigmoid }
  output:
    type: prediction
    dim: 1
  training:
    loss: mse
    optimizer: adam
    learning_rate: 0.001
    batch_size: 128
    epochs: 200
```

---

## API Endpoints

### `POST /architectures`
Create and register a new architecture.

**Request body:** `ArchitectureSpec` (YAML or JSON).

**Response:** `201 Created`
```json
{
  "architecture_id": "arch_abc123",
  "name": "lstm_autoencoder_chiller_v2",
  "status": "registered",
  "created_at": "2026-01-15T10:30:00Z"
}
```

### `GET /architectures/{architecture_id}`
Retrieve an architecture spec by ID.

### `GET /architectures?equipment_type=chiller`
List architectures, optionally filtered by equipment type.

### `POST /architectures/{architecture_id}/validate`
Validate the architecture spec without creating a training run.

**Response:** `200 OK`
```json
{
  "valid": true,
  "warnings": [],
  "estimated_parameters": 1245184,
  "estimated_memory_mb": 48.5
}
```

### `DELETE /architectures/{architecture_id}`
Delete a registered architecture.

---

## Error Codes

| Code | Meaning |
|------|---------|
| `INVALID_LAYER_TYPE` | Unknown layer type in spec |
| `DIMENSION_MISMATCH` | Output dim of one layer does not match input dim of next |
| `MISSING_REQUIRED_PARAM` | A required layer parameter is absent |
| `SEQUENCE_LENGTH_TOO_SHORT` | Sequence length < 4 is not supported |
| `FEATURE_COUNT_ZERO` | Feature count must be >= 1 |
| `UNSUPPORTED_LOSS` | Loss function not supported for the output type |

---

## Best Practices

1. **Start with a template** and customise rather than building from scratch.
2. **Match sequence length to data resolution:** hourly data -> 24-168; 15-min data -> 96-672.
3. **Use batch normalisation** in CNN layers for faster convergence.
4. **Prefer AdamW** over Adam when using weight decay.
5. **Enable early stopping** to avoid over-fitting on small datasets.
6. **Use focal loss** for imbalanced anomaly detection tasks (anomaly ratio < 5 %).
7. **Validate before training** using the `/validate` endpoint to catch schema errors cheaply.
8. **Use cross-entropy loss** for multi-class classification (vision, NLP models).
9. **Use AdamW** for vision and NLP architectures; Adam is fine for temporal/tabular.
10. **Lower learning rates** for transformers: 1e-4 for ViT, 5e-5 for BERT.
11. **Use Phantom** for edge deployment on devices with <1MB RAM budget.
