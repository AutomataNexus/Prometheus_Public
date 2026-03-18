# prometheus-training

ML training pipeline orchestrator for the Prometheus platform. Built on [AxonML](https://github.com/AutomataNexus/AxonML), a pure-Rust autograd framework, this crate provides end-to-end model training with real backpropagation -- no Python runtime required.

## Architecture

The pipeline is a 5-stage async orchestrator that streams progress events over a `tokio::sync::mpsc` channel for real-time WebSocket updates to the UI.

### Pipeline Stages

1. **Validate** -- Verify configuration correctness, dataset existence, and output directory accessibility.
2. **Preprocess** -- Load CSV data, detect label columns, apply z-score normalization (computed from training set only to prevent data leakage), and split into train/val/test sets.
3. **Train** -- Build model and optimizer, run mini-batch gradient descent with AxonML autograd (forward pass, loss computation, `backward()`, optimizer `step()`). Supports early stopping based on validation loss patience.
4. **Evaluate** -- Compute comprehensive metrics on the held-out test set.
5. **Export** -- Serialize to `.axonml` binary format, optionally quantize to INT8, and optionally cross-compile for ARM edge deployment.

### Supported Architectures (13)

| Category | Architecture | Description |
|---|---|---|
| Time Series | `LstmAutoencoder` | LSTM encoder-decoder for anomaly detection via reconstruction error |
| | `GruPredictor` | Multi-horizon GRU for failure prediction at 5/15/30 min horizons |
| | `Rnn` | Vanilla RNN for sequence modeling |
| | `Sentinel` | MLP health scorer producing a 0.0--1.0 facility health score |
| | `Conv1d` | 1D convolution for temporal feature extraction |
| Computer Vision | `ResNet` | ResNet-18 image classification |
| | `Vgg` | VGG-11 image classification |
| | `ViT` | Vision Transformer with patch embedding and multi-head attention |
| | `Conv2d` | 2D convolution for spatial feature extraction |
| NLP | `Bert` | Bidirectional encoder for text classification |
| | `Gpt2` | Autoregressive transformer for next-token prediction |
| Advanced | `Nexus` | Multi-modal fusion model with transformer backbone |
| | `Phantom` | Lightweight edge model optimized for constrained devices |

All architectures implement the `TrainableModel` trait, which provides a unified interface for forward pass, parameter access, and serialization.

### Loss Functions

Loss functions are selected automatically per architecture:

- **MSE** (`MSELoss`) -- `LstmAutoencoder` (reconstruction loss)
- **Cross-Entropy** (`CrossEntropyLoss`) -- `ResNet`, `Vgg`, `ViT`, `Conv2d`, `Bert`, `Gpt2`
- **BCE** (`BCELoss`) -- `GruPredictor`, `Sentinel`, `Rnn`, `Conv1d`, `Nexus`, `Phantom`

### Optimizers

- **Adam** -- `LstmAutoencoder`, `Rnn`, `Sentinel`, `Nexus`, `Phantom`, `Conv1d`
- **AdamW** (with configurable weight decay) -- `GruPredictor`, `ResNet`, `Vgg`, `ViT`, `Conv2d`, `Bert`, `Gpt2`

### Evaluation Metrics

The `Metrics` struct reports the following on the test set:

| Metric | Field | Notes |
|---|---|---|
| Loss | `loss` | Architecture-specific loss value |
| Accuracy | `accuracy` | (TP + TN) / total |
| Precision | `precision` | TP / (TP + FP) |
| Recall | `recall` | TP / (TP + FN) |
| F1 Score | `f1` | Harmonic mean of precision and recall |
| AUC-ROC | `auc_roc` | Trapezoidal approximation over sorted predictions |
| MSE | `mse` | Mean squared error (primary for regression/autoencoder tasks) |
| MAE | `mae` | Mean absolute error |

For autoencoder models, per-sample reconstruction errors and percentile-based anomaly thresholds are also computed.

## Data Preprocessing

- **Input format**: CSV with header row. Columns named `label`, `target`, `y`, or `class` are auto-detected as label columns; all other numeric columns become features.
- **Normalization**: Z-score normalization `(x - mean) / std`, computed from the training split only. Normalization stats are embedded in the exported model for inference-time denormalization.
- **Splitting**: Configurable train/val/test ratios (default 70/15/15). Minimum 3 samples required.
- **Sequence creation**: For temporal models (LSTM, GRU, RNN), sliding-window sequences of configurable length are created from the time-series data.

## Export Format

Models are saved in the `.axonml` binary format:

```
[6 bytes] Magic: "AXONML"
[1 byte]  Version: 0x01
[4 bytes] Header length (LE u32)
[N bytes] JSON header (architecture, input_features, num_parameters, quantization info)
[4 bytes] Weights length (LE u32)
[M bytes] JSON weights (flat parameter vector, normalization stats, hyperparameters)
```

### INT8 Quantization

Optional per-tensor symmetric quantization maps f32 weights to i8 values using `scale = max_abs / 127`. Quantized models are saved as `{run_id}-int8.axonml`.

### Model Conversion

Trained `.axonml` models can be converted to other formats via the server API:

- **ONNX** -- `POST /api/v1/models/:id/convert?format=onnx` (uses `torch.onnx.export` via the Python converter at `tools/model_converter/convert.py`; all 13 architectures validated)
- **HEF** -- `POST /api/v1/models/:id/convert?format=hef` (requires Hailo DFC SDK)

Converted models are downloadable at `GET /api/v1/models/:id/download?format=onnx|hef|axonml`.

## Hyperparameters

| Parameter | Default | Description |
|---|---|---|
| `learning_rate` | 0.001 | Optimizer learning rate |
| `epochs` | 100 | Maximum training epochs |
| `batch_size` | 32 | Mini-batch size |
| `sequence_length` | 60 | Sliding window length for temporal models |
| `hidden_dim` | 64 | Hidden layer dimension |
| `num_layers` | 2 | Number of recurrent/transformer layers |
| `dropout` | 0.1 | Dropout probability |
| `weight_decay` | 0.01 | AdamW weight decay |
| `early_stopping_patience` | 10 | Epochs without val loss improvement before stopping |
| `val_check_interval` | 1 | Validate every N epochs |

## Cross-Compilation

The pipeline supports optional cross-compilation targeting `armv7-unknown-linux-musleabihf` for deployment on Raspberry Pi edge devices via the `prometheus-edge` daemon.

## Usage

```rust
use prometheus_training::{Architecture, TrainingConfig, run_pipeline};

let config = TrainingConfig::new(
    Architecture::LstmAutoencoder,
    "/data/sensors.csv",
    "/models/output",
    12, // input features
);

let result = run_pipeline(config, None).await?;
println!("F1: {:.4}, artifact: {}", result.metrics.f1, result.artifact_path);
```
