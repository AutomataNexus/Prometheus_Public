# prometheus-edge

Edge inference daemon for deploying trained AxonML models to Raspberry Pi controllers. Runs as a lightweight HTTP service that polls sensor data from the NexusEdge hardware daemon and executes real-time anomaly detection inference.

## Architecture

```
NexusEdge Daemon (port 6100)     prometheus-edge (port 6200)
   /sensors  ──────────────────>  SensorPoller (background thread)
                                       │
                                       v
                                  InferenceEngine
                                       │
                                       v
                                  HTTP API ──> predict, metrics, health
```

Three threads run concurrently:

- **Main thread** -- `tiny_http` accept loop, spawns per-request handler threads
- **Sensor poller** -- polls NexusEdge `/sensors` at a configurable interval, caches latest reading behind `Arc<Mutex<>>`
- **Inference loop** -- runs forward-pass inference at a separate interval, stores predictions in `RwLock`

## Model Format (.axonml)

Binary file with a metadata header and concatenated f32 weights:

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | Magic bytes `AXON` |
| 4 | 4 | Version (u32 LE, 1 or 2) |
| 8 | 4 | Header length (u32 LE) |
| 12 | N | Header JSON (`ModelMetadata`) |
| 12+N | ... | f32 LE weight values |

The header JSON describes layer dimensions, activation functions (relu, sigmoid, tanh, none), and bias flags. The inference engine rebuilds a stack of dense layers from these descriptors and runs a sequential forward pass.

## HTTP API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Liveness check with uptime, model name, version |
| GET | `/predict` | Inference on latest cached sensor data |
| POST | `/predict` | Inference on provided JSON (raw vector, named map, or `{"values": {...}}`) |
| GET | `/metrics` | Model metadata, prediction count, anomaly/health scores |

All responses are JSON. Non-200 responses return `{"error": "...", "message": "..."}`.

## Configuration

Supports JSON and TOML (lightweight built-in parser, no `toml` crate dependency). Default path: `/etc/prometheus/edge.json`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model_path` | string | `model.axonml` | Path to .axonml model file |
| `nexus_url` | string | `http://127.0.0.1:6100` | NexusEdge daemon URL |
| `poll_interval_secs` | u64 | 1 | Sensor poll frequency |
| `inference_interval_secs` | u64 | 30 | Inference loop frequency |
| `http_port` | u16 | 6200 | HTTP server port |
| `features` | string[] | [] | Ordered sensor names for feature extraction |
| `normalization` | map | {} | Per-feature min/max normalization to [0,1] |
| `anomaly_threshold` | f32 | 0.75 | Score above which readings are flagged |
| `unit_id` | string | "" | Human-readable device identifier |

## Usage

```bash
prometheus-edge --config /etc/prometheus/edge.json
prometheus-edge -c /etc/prometheus/edge.toml
```

## Key Design Decisions

- **No async runtime** -- uses `std::thread` and synchronous I/O (`tiny_http`, `minreq`) to minimize binary size and memory on ARM devices.
- **Exponential backoff** -- sensor polling backs off on consecutive failures (capped at 30s), recovers automatically.
- **Anomaly scoring** -- single-output models use direct probability; multi-output models use RMS magnitude clamped to [0,1].
- **Feature normalization** -- configurable per-feature min/max scaling ensures models receive consistently scaled inputs regardless of sensor range.

## Dependencies

- `tiny_http` -- synchronous HTTP server
- `minreq` -- minimal HTTP client for sensor polling
- `chrono`, `serde`, `serde_json` -- timestamps and serialization
- `tracing` / `tracing-subscriber` -- structured logging
