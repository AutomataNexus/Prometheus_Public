# Training History Template

Template and reference for recording past Prometheus training runs.  Each
entry captures architecture, hyperparameters, data characteristics, results
and operator notes to inform future architecture selection and tuning.

---

## Record Schema

```yaml
run_id: <string>                    # Unique run identifier
timestamp: <ISO 8601>               # When the run started
operator: <string>                  # Who initiated the run (user or agent)

# Equipment & data
equipment_type: <string>            # e.g. "chiller", "ahu"
equipment_id: <string>              # Specific asset ID
dataset_id: <string>                # Reference to training dataset
dataset_rows: <int>                 # Number of rows in training set
dataset_features: <int>             # Number of input features
anomaly_ratio: <float>              # Fraction of labelled anomalies (0.0-1.0)
data_quality_score: <float>         # 0-100 score from DataAnalystAgent

# Architecture
architecture: <string>              # lstm_autoencoder | gru_predictor | sentinel | rnn | resnet | vgg | vit | bert | gpt2 | nexus | phantom | conv1d | conv2d
architecture_id: <string>           # AxonML architecture registry ID
sequence_length: <int>
hyperparameters:
  learning_rate: <float>
  batch_size: <int>
  epochs: <int>
  early_stopping_patience: <int>
  dropout: <float>
  hidden_dim: <int>
  # ... additional architecture-specific params

# Training
training_duration_minutes: <float>
final_epoch: <int>                  # Epoch at which training stopped
stopped_early: <bool>               # Whether early stopping triggered
training_loss_final: <float>
validation_loss_final: <float>

# Evaluation metrics
metrics:
  precision: <float>
  recall: <float>
  f1: <float>
  auc_roc: <float>
  accuracy: <float>
  mse: <float>                      # For reconstruction-based models
  mae: <float>

# Benchmark comparison
benchmark_passed: <bool>
benchmark_details:
  precision_threshold: <float>
  recall_threshold: <float>
  f1_threshold: <float>
  auc_roc_threshold: <float>

# Deployment
deploy_ready: <bool>
deployed: <bool>
deployment_id: <string>             # If deployed, the deployment reference
deployment_timestamp: <ISO 8601>

# Notes
notes: |
  Free-form operator or agent notes about the run.
  Include any observations, anomalies during training, or recommendations.
tags:
  <key>: <value>
```

---

## Example Records

### Run: trn_20260115_chiller_001

```yaml
run_id: trn_20260115_chiller_001
timestamp: "2026-01-15T14:00:00Z"
operator: prometheus_forge_agent

equipment_type: chiller
equipment_id: CHL-BLDG-A-01
dataset_id: ds_chiller_2025q4
dataset_rows: 87600
dataset_features: 14
anomaly_ratio: 0.032
data_quality_score: 88.5

architecture: sentinel
architecture_id: arch_sentinel_chiller_v3
sequence_length: 168
hyperparameters:
  learning_rate: 0.0005
  batch_size: 32
  epochs: 150
  early_stopping_patience: 15
  dropout: 0.2
  conv_filters: [64, 128]
  attention_heads: 8
  hidden_dim: 256

training_duration_minutes: 47.3
final_epoch: 112
stopped_early: true
training_loss_final: 0.0043
validation_loss_final: 0.0051

metrics:
  precision: 0.91
  recall: 0.87
  f1: 0.89
  auc_roc: 0.95
  accuracy: 0.97

benchmark_passed: true
benchmark_details:
  precision_threshold: 0.88
  recall_threshold: 0.85
  f1_threshold: 0.86
  auc_roc_threshold: 0.92

deploy_ready: true
deployed: true
deployment_id: dep_20260115_chiller_001
deployment_timestamp: "2026-01-15T15:30:00Z"

notes: |
  Sentinel v3 trained on Q4 2025 chiller data.  Early stopping triggered at
  epoch 112 (patience 15).  All benchmarks exceeded.  Deployed to production
  edge node for Building A chiller plant.
tags:
  building: A
  version: v3
  season: winter
```

### Run: trn_20260201_ahu_002

```yaml
run_id: trn_20260201_ahu_002
timestamp: "2026-02-01T09:00:00Z"
operator: prometheus_forge_agent

equipment_type: ahu
equipment_id: AHU-BLDG-B-03
dataset_id: ds_ahu_2025_full
dataset_rows: 52560
dataset_features: 10
anomaly_ratio: 0.018
data_quality_score: 92.0

architecture: lstm_autoencoder
architecture_id: arch_lstm_ae_ahu_v2
sequence_length: 48
hyperparameters:
  learning_rate: 0.001
  batch_size: 64
  epochs: 100
  early_stopping_patience: 10
  dropout: 0.2
  encoder_layers: [128, 64]
  latent_dim: 32

training_duration_minutes: 22.1
final_epoch: 78
stopped_early: true
training_loss_final: 0.0021
validation_loss_final: 0.0029

metrics:
  precision: 0.89
  recall: 0.84
  f1: 0.86
  auc_roc: 0.93
  mse: 0.0029

benchmark_passed: true
benchmark_details:
  precision_threshold: 0.85
  recall_threshold: 0.82
  f1_threshold: 0.83
  auc_roc_threshold: 0.91

deploy_ready: true
deployed: true
deployment_id: dep_20260201_ahu_002
deployment_timestamp: "2026-02-01T11:00:00Z"

notes: |
  LSTM Autoencoder v2 on full-year AHU data for Building B.  Good
  reconstruction accuracy.  Recall slightly below chiller benchmarks
  but exceeds AHU thresholds.  Deployed.
tags:
  building: B
  version: v2
  season: all
```

### Run: trn_20260210_pump_001

```yaml
run_id: trn_20260210_pump_001
timestamp: "2026-02-10T16:00:00Z"
operator: manual

equipment_type: pump
equipment_id: PMP-CW-01
dataset_id: ds_pump_vibration_jan2026
dataset_rows: 8640
dataset_features: 7
anomaly_ratio: 0.045
data_quality_score: 75.0

architecture: gru_predictor
architecture_id: arch_gru_pred_pump_v1
sequence_length: 24
hyperparameters:
  learning_rate: 0.001
  batch_size: 128
  epochs: 80
  early_stopping_patience: 10
  dropout: 0.1
  hidden_dim: 64
  num_layers: 2

training_duration_minutes: 8.5
final_epoch: 80
stopped_early: false
training_loss_final: 0.0085
validation_loss_final: 0.0112

metrics:
  precision: 0.82
  recall: 0.78
  f1: 0.80
  auc_roc: 0.88

benchmark_passed: false
benchmark_details:
  precision_threshold: 0.84
  recall_threshold: 0.80
  f1_threshold: 0.82
  auc_roc_threshold: 0.90

deploy_ready: false
deployed: false

notes: |
  GRU predictor on 10 days of pump vibration data.  Data quality score
  impacted by small dataset size and 4.5 % anomaly ratio.  Model did not
  meet pump benchmarks — recall and AUC below thresholds.
  Recommendations:
  - Collect at least 30 days of data.
  - Consider LSTM Autoencoder if dataset grows.
  - Review outlier labels for correctness.
tags:
  building: A
  version: v1
  issue: insufficient_data
```

---

## Aggregated Statistics

Use these aggregated statistics to inform architecture selection defaults
and expected training times.

| Equipment | Architecture | Avg F1 | Avg AUC | Avg Duration (min) | Pass Rate |
|-----------|-------------|--------|---------|---------------------|-----------|
| Chiller | Sentinel | 0.88 | 0.94 | 45 | 85% |
| Chiller | LSTM AE | 0.84 | 0.91 | 30 | 70% |
| AHU | LSTM AE | 0.87 | 0.93 | 22 | 90% |
| AHU | GRU Predictor | 0.81 | 0.89 | 12 | 65% |
| Boiler | LSTM AE | 0.89 | 0.94 | 18 | 88% |
| Pump | GRU Predictor | 0.83 | 0.90 | 8 | 72% |
| Pump | LSTM AE | 0.86 | 0.92 | 15 | 80% |
| Fan Coil | GRU Predictor | 0.82 | 0.88 | 6 | 75% |
| Steam | LSTM AE | 0.86 | 0.92 | 20 | 82% |
| Steam | Sentinel | 0.88 | 0.94 | 40 | 80% |
| Image       | ResNet       | 0.85 | 0.91 | 35 | 78% |
| Image       | VGG          | 0.83 | 0.89 | 42 | 72% |
| Image       | ViT          | 0.87 | 0.93 | 50 | 80% |
| Text        | BERT         | 0.88 | 0.94 | 30 | 82% |
| Text        | GPT-2        | 0.82 | 0.88 | 55 | 68% |
| Temporal    | RNN          | 0.79 | 0.85 | 6  | 60% |
| Temporal    | Conv1D       | 0.84 | 0.90 | 10 | 75% |
| Spatial     | Conv2D       | 0.82 | 0.88 | 18 | 70% |
| Multimodal  | Nexus        | 0.86 | 0.92 | 40 | 76% |
| Edge        | Phantom      | 0.78 | 0.84 | 3  | 65% |

---

## Querying History

To query training history from the Prometheus API:

```
GET /api/v1/training/history?equipment_type=chiller&limit=10&sort=timestamp:desc
GET /api/v1/training/history?architecture=sentinel&benchmark_passed=true
GET /api/v1/training/history?run_id=trn_20260115_chiller_001
```

Filters: `equipment_type`, `architecture`, `benchmark_passed`, `deployed`,
`date_from`, `date_to`, `operator`, `tags.*`.
