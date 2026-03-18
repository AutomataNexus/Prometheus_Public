# Prometheus -- API Reference

Base URL: `http://localhost:3030`

All endpoints except `/health`, public auth routes (login, signup, verify-email, forgot-password, reset-password), and the Stripe webhook require an `Authorization: Bearer <token>` header. Tokens are opaque bearer tokens obtained from the login endpoint (NOT JWTs).

---

## Authentication

### POST /api/v1/auth/login

Login with username and password. Returns an opaque bearer token.

**Request:**
```json
{
  "username": "admin",
  "password": "your_password"
}
```

**Response (200):**
```json
{
  "token": "opaque_bearer_token_here",
  "user": {
    "id": "usr_abc123",
    "username": "admin",
    "email": "admin@example.com",
    "role": "admin"
  },
  "email_verified": true,
  "mfa_required": false
}
```

If `email_verified` is `false`, login is blocked with `403 Forbidden` (user must verify email first). If `mfa_required` is `true`, the client must call `POST /auth/mfa/validate` with a TOTP code before the token is usable.

**Errors:**
- `401 Unauthorized` -- Invalid credentials
- `403 Forbidden` -- Email not verified
- `429 Too Many Requests` -- Rate limit exceeded (30 attempts/min per IP)

### POST /api/v1/auth/logout

Invalidate the current session.

**Headers:** `Authorization: Bearer <token>`

**Response (200):**
```json
{ "message": "Logged out" }
```

### GET /api/v1/auth/session

Validate the current session token.

**Headers:** `Authorization: Bearer <token>`

**Response (200):**
```json
{
  "valid": true,
  "user": {
    "id": "usr_abc123",
    "username": "admin",
    "role": "admin"
  }
}
```

**Errors:** `401 Unauthorized` -- Token invalid or expired

### POST /api/v1/auth/signup

Register a new user account. A Free tier subscription is automatically created. A 6-digit email verification code is sent.

**Request:**
```json
{
  "username": "newuser",
  "email": "user@example.com",
  "password": "secure_password"
}
```

**Response (201):**
```json
{
  "message": "Account created. Check your email for verification code.",
  "user_id": "newuser"
}
```

**Errors:** `400 Bad Request` -- Username taken, invalid email, or password too short (min 8 chars)

### POST /api/v1/auth/verify-email

Verify email address with the 6-digit code sent during signup.

**Request:**
```json
{
  "email": "user@example.com",
  "code": "482917"
}
```

**Response (200):**
```json
{ "message": "Email verified successfully" }
```

**Errors:** `400 Bad Request` -- Invalid or expired code (15-minute expiry)

### POST /api/v1/auth/resend-verification

Resend the email verification code.

**Request:**
```json
{ "email": "user@example.com" }
```

**Response (200):**
```json
{ "message": "Verification code sent" }
```

### POST /api/v1/auth/forgot-password

Request a password reset. Always returns success to prevent user enumeration.

**Request:**
```json
{ "email": "user@example.com" }
```

**Response (200):**
```json
{ "message": "If an account exists with that email, a reset link has been sent." }
```

### POST /api/v1/auth/reset-password

Reset password using the token from the password reset email.

**Request:**
```json
{
  "token": "uuid-reset-token",
  "new_password": "new_secure_password"
}
```

**Response (200):**
```json
{ "message": "Password reset successfully" }
```

**Errors:** `400 Bad Request` -- Invalid or expired token (30-minute expiry)

### GET /api/v1/auth/me

Get the current authenticated user's profile.

**Headers:** `Authorization: Bearer <token>`

**Response (200):**
```json
{
  "id": "usr_abc123",
  "username": "admin",
  "email": "admin@example.com",
  "role": "admin",
  "created_at": "2026-01-01T00:00:00Z"
}
```

---

## Datasets

### GET /api/v1/datasets

List all datasets.

**Response (200):**
```json
[
  {
    "id": "ds_abc123",
    "name": "Warren AHU-1 Sensors",
    "equipment_type": "air_handler",
    "location": "Warren",
    "unit": "AHU-1",
    "source": "csv_upload",
    "row_count": 86400,
    "columns": ["timestamp", "supply_temp", "return_temp", "outside_air_temp", "discharge_temp", "fan_speed", "damper_position", "filter_dp"],
    "file_size_bytes": 12582912,
    "created_at": "2026-03-06T12:00:00Z",
    "created_by": "admin"
  }
]
```

### POST /api/v1/datasets

Upload a CSV dataset. Uses `multipart/form-data`.

**Fields:**
- `file` (required) -- CSV file (max 100 MB)
- `name` (required) -- Dataset display name
- `equipment_type` (required) -- One of: `air_handler`, `boiler`, `pump`, `chiller`, `fan_coil`, `steam`
- `location` (optional) -- Building location name
- `unit` (optional) -- Equipment unit identifier (e.g., "AHU-1")

**Response (201):**
```json
{
  "id": "ds_def456",
  "name": "Uploaded Dataset",
  "equipment_type": "air_handler",
  "row_count": 100,
  "columns": ["timestamp", "supply_temp", "return_temp"],
  "column_stats": {
    "supply_temp": { "min": 52.1, "max": 78.3, "mean": 65.2, "std": 4.7 },
    "return_temp": { "min": 68.0, "max": 80.0, "mean": 73.5, "std": 3.2 }
  },
  "time_range": { "start": "2026-01-01T00:00:00Z", "end": "2026-01-02T01:00:00Z" },
  "created_at": "2026-03-06T12:00:00Z"
}
```

**Errors:** `400 Bad Request` -- Invalid CSV or missing required fields

### POST /api/v1/datasets/connect

Connect to an external data source and import sensor data into Prometheus. Supports 7 source types with **AegisControlBridge** as the default/native path for edge controller data.

#### Two-Tier Data Architecture

1. **Aegis-to-Aegis (default)** -- Native data path via AegisControlBridge. Edge controllers run Aegis-DB locally; AegisControlBridge syncs data to the cloud Prometheus Aegis-DB instance. Use `source_type: "aegis_bridge"` to pull data directly from an edge controller's Aegis-DB.

2. **External Sources** -- Import from third-party databases (InfluxDB, PostgreSQL, TiDB, SQLite3, MongoDB, SpaceTimeDB). Data is normalized to CSV internally and stored as a Prometheus dataset.

#### Common Request Fields

All source types share these fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `source_type` | string | No | Source type (default: `"aegis_bridge"`) |
| `name` | string | Yes | Dataset display name |
| `equipment_type` | string | Yes | One of: `air_handler`, `boiler`, `pump`, `chiller`, `fan_coil`, `steam` |

#### Source Type: `aegis_bridge` (Default)

Pull data directly from an edge controller's local Aegis-DB via AegisControlBridge.

**Request:**
```json
{
  "source_type": "aegis_bridge",
  "controller_ip": "100.124.76.93",
  "aegis_port": 9090,
  "collection": "hardware_metrics",
  "time_range": {
    "start": "2026-01-01T00:00:00Z",
    "end": "2026-01-31T23:59:59Z"
  },
  "name": "Warren AHU-1 Edge Metrics",
  "equipment_type": "air_handler"
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `controller_ip` | string | Yes | -- | Edge controller IP (Tailscale or LAN) |
| `aegis_port` | integer | No | `9090` | Aegis-DB port on the edge controller |
| `collection` | string | No | `"hardware_metrics"` | Aegis-DB document collection to query |
| `time_range` | object | No | Last 30 days | Time range filter |

#### Source Type: `influxdb`

Import from an InfluxDB v3 instance using the SQL query API.

**Request:**
```json
{
  "source_type": "influxdb",
  "url": "http://influxdb-host:8086",
  "database": "building_sensors",
  "measurement": "ahu_readings",
  "time_range": {
    "start": "2026-01-01T00:00:00Z",
    "end": "2026-01-31T23:59:59Z"
  },
  "name": "Warren AHU-1 InfluxDB",
  "equipment_type": "air_handler"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | InfluxDB base URL (e.g., `http://host:8086`) |
| `database` | string | Yes | InfluxDB database name |
| `measurement` | string | Yes | Measurement name to query |
| `time_range` | object | No | Time range filter |

#### Source Type: `postgresql`

Import from a PostgreSQL database via Aegis-DB federated query or a direct REST API.

**Request:**
```json
{
  "source_type": "postgresql",
  "connection_string": "postgresql://user:pass@host:5432/dbname",
  "query": "SELECT * FROM sensor_readings WHERE timestamp > '2026-01-01'",
  "name": "External PostgreSQL Data",
  "equipment_type": "boiler"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `connection_string` | string | Yes* | PostgreSQL connection URL |
| `api_url` | string | Yes* | Alternative: direct REST API URL returning JSON rows |
| `query` | string | Yes | SQL query to execute |

*Provide either `connection_string` (routed through Aegis-DB federated query) or `api_url` (direct REST endpoint).

#### Source Type: `tidb`

Import from a TiDB cluster. Uses the same interface as PostgreSQL (Aegis-DB federated query or direct REST).

**Request:**
```json
{
  "source_type": "tidb",
  "connection_string": "mysql://user:pass@tidb-host:4000/dbname",
  "query": "SELECT * FROM equipment_telemetry LIMIT 100000",
  "name": "TiDB Equipment Telemetry",
  "equipment_type": "chiller"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `connection_string` | string | Yes* | TiDB/MySQL connection URL |
| `api_url` | string | Yes* | Alternative: direct REST API URL |
| `query` | string | Yes | SQL query to execute |

#### Source Type: `sqlite`

Import from a SQLite3 database file accessible to the server.

**Request:**
```json
{
  "source_type": "sqlite",
  "connection_string": "/path/to/local/database.db",
  "query": "SELECT * FROM readings WHERE date > '2026-01-01'",
  "name": "Local SQLite Readings",
  "equipment_type": "pump"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `connection_string` | string | Yes | Path to the SQLite3 file |
| `query` | string | Yes | SQL query to execute |

#### Source Type: `mongodb`

Import from a MongoDB instance using the MongoDB Data API.

**Request:**
```json
{
  "source_type": "mongodb",
  "api_url": "https://data.mongodb-api.com/app/data-xxxxx/endpoint/data/v1",
  "api_key": "your-mongodb-data-api-key",
  "database": "building_data",
  "mongodb_collection": "sensor_readings",
  "filter": { "location": "Warren", "timestamp": { "$gte": "2026-01-01T00:00:00Z" } },
  "name": "MongoDB Sensor Data",
  "equipment_type": "air_handler"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `api_url` | string | Yes | MongoDB Data API base URL |
| `api_key` | string | Yes | MongoDB Data API key |
| `database` | string | Yes | MongoDB database name |
| `mongodb_collection` | string | Yes | Collection name |
| `filter` | object | No | MongoDB query filter (default: `{}`) |

#### Source Type: `spacetimedb`

Import from a SpaceTimeDB instance using the SQL query endpoint.

**Request:**
```json
{
  "source_type": "spacetimedb",
  "api_url": "https://spacetimedb.example.com",
  "database": "building_sensors",
  "query": "SELECT * FROM temperature_readings",
  "auth_token": "optional-bearer-token",
  "name": "SpaceTimeDB Temperature Data",
  "equipment_type": "fan_coil"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `api_url` | string | Yes | SpaceTimeDB base URL |
| `database` | string | Yes | SpaceTimeDB database name |
| `query` | string | Yes | SQL query to execute |
| `auth_token` | string | No | Bearer token for authentication |

#### Common Response (all source types)

**Response (201):**
```json
{
  "id": "ds_ghi789",
  "name": "Warren AHU-1 Edge Metrics",
  "source": "aegis_bridge",
  "row_count": 43200,
  "columns": ["timestamp", "supply_temp", "return_temp", "fan_speed", "damper_pos"],
  "column_stats": {
    "supply_temp": { "min": 52.1, "max": 78.3, "mean": 65.2, "std": 4.7 },
    "return_temp": { "min": 68.0, "max": 80.0, "mean": 73.5, "std": 3.2 }
  },
  "time_range": { "start": "2026-01-01T00:00:00Z", "end": "2026-01-31T23:59:59Z" },
  "created_at": "2026-03-06T12:00:00Z"
}
```

**Errors:**
- `400 Bad Request` -- Unsupported source_type, missing required fields, or invalid configuration
- `502 Bad Gateway` -- Failed to connect to the external data source
- `504 Gateway Timeout` -- External source query timed out

### GET /api/v1/datasets/:id

Get dataset detail including column statistics.

**Response (200):**
```json
{
  "id": "ds_abc123",
  "name": "Warren AHU-1 Sensors",
  "equipment_type": "air_handler",
  "row_count": 86400,
  "columns": ["timestamp", "supply_temp", "return_temp", "..."],
  "column_stats": {
    "supply_temp": { "min": 52.1, "max": 78.3, "mean": 65.2, "std": 4.7 }
  },
  "time_range": { "start": "2026-01-01T00:00:00Z", "end": "2026-01-31T23:59:59Z" },
  "file_size_bytes": 12582912,
  "created_at": "2026-03-06T12:00:00Z"
}
```

### GET /api/v1/datasets/:id/preview

Get a paginated, sortable preview of the dataset.

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | integer | `1` | Page number |
| `page_size` | integer | `100` | Rows per page (max: 1000) |
| `sort_col` | string | -- | Column name to sort by (optional) |
| `sort_dir` | string | `asc` | Sort direction: `asc` or `desc` (optional) |

**Response (200):**
```json
{
  "headers": ["timestamp", "supply_temp", "return_temp"],
  "rows": [
    ["2026-01-01T00:00:00Z", "55.2", "72.1"],
    ["2026-01-01T00:15:00Z", "55.4", "72.0"]
  ],
  "total_rows": 86400,
  "page": 1,
  "total_pages": 864
}
```

### DELETE /api/v1/datasets/:id

Delete a dataset and its associated data.

**Response:** `204 No Content`

**Errors:** `404 Not Found`

### POST /api/v1/datasets/:id/validate

Validate a dataset for training readiness. Scans all rows for type consistency, missing values, and column width mismatches. On success, the dataset is locked (frozen) and auto-compressed with zstd.

**Response (200):**
```json
{
  "valid": true,
  "issues": [],
  "columns_checked": 8,
  "rows_checked": 86400,
  "is_validated": true,
  "locked": true
}
```

If validation fails:
```json
{
  "valid": false,
  "issues": [
    "Column 'supply_temp': mixed types detected (numeric: 85000, string: 1400)",
    "Column 'damper_pos': 230 missing values (0.27%)"
  ],
  "columns_checked": 8,
  "rows_checked": 86400,
  "is_validated": false,
  "locked": false
}
```

### POST /api/v1/datasets/:id/unlock

Unlock a validated dataset, allowing modifications. Auto-decompresses if compressed. Requires re-validation before training.

**Response (200):**
```json
{ "message": "Dataset unlocked", "is_validated": false, "locked": false }
```

### GET /api/v1/datasets/:id/recommend

Get AI-powered model architecture recommendations based on dataset analysis.

**Response (200):**
```json
{
  "dataset_id": "ds_abc123",
  "recommendations": [
    {
      "architecture": "lstm_autoencoder",
      "match_score": 0.95,
      "use_case": "Anomaly Detection",
      "description": "Learns normal patterns and detects deviations...",
      "inputs": "8 sensor columns, 60-step sequences",
      "outputs": "Reconstruction error score (0.0-1.0)",
      "inference_result": "Real-time anomaly alerts when score exceeds threshold",
      "hyperparameters": { "hidden_dim": 64, "num_layers": 2, "epochs": 100 }
    }
  ]
}
```

---

## Training

### GET /api/v1/training

List all training runs.

**Response (200):**
```json
[
  {
    "id": "tr_ghi789",
    "dataset_id": "ds_abc123",
    "model_id": "mdl_def456",
    "architecture": "lstm_autoencoder",
    "status": "completed",
    "current_epoch": 100,
    "total_epochs": 100,
    "best_val_loss": 0.0082,
    "training_time_seconds": 342,
    "started_at": "2026-03-06T14:00:00Z",
    "completed_at": "2026-03-06T14:05:42Z"
  }
]
```

### POST /api/v1/training/start

Start a new training run.

**Request:**
```json
{
  "dataset_id": "ds_abc123",
  "architecture": "lstm_autoencoder",
  "hyperparameters": {
    "learning_rate": 0.001,
    "batch_size": 64,
    "epochs": 100,
    "hidden_dim": 64,
    "bottleneck_dim": 32,
    "num_layers": 2,
    "sequence_length": 60,
    "dropout": 0.1,
    "optimizer": "adam",
    "loss": "mse"
  }
}
```

Valid architectures: `lstm_autoencoder`, `gru_predictor`, `sentinel`

**Response (201):**
```json
{
  "id": "tr_new123",
  "status": "running",
  "dataset_id": "ds_abc123",
  "architecture": "lstm_autoencoder",
  "started_at": "2026-03-06T14:00:00Z"
}
```

Response when queued (server at capacity):
```json
{
  "id": "tr_new123",
  "status": "queued",
  "dataset_id": "ds_abc123",
  "architecture": "lstm_autoencoder",
  "started_at": "2026-03-06T14:00:00Z"
}
```

**Errors:**
- `400 Bad Request` -- Invalid architecture or hyperparameters
- `400 Bad Request` -- Dataset has not been validated
- `404 Not Found` -- Dataset not found

### GET /api/v1/training/queue

Get training queue status.

**Response (200):**
```json
{
  "active_trainings": 4,
  "max_concurrent": 8,
  "queued": 2,
  "capacity_available": 4
}
```

### GET /api/v1/training/:id

Get training run detail including current metrics.

**Response (200):**
```json
{
  "id": "tr_ghi789",
  "dataset_id": "ds_abc123",
  "model_id": "mdl_def456",
  "architecture": "lstm_autoencoder",
  "hyperparameters": { "..." },
  "status": "running",
  "current_epoch": 45,
  "total_epochs": 100,
  "best_val_loss": 0.0098,
  "current_train_loss": 0.0112,
  "current_val_loss": 0.0098,
  "training_time_seconds": 145,
  "estimated_remaining_seconds": 178,
  "started_at": "2026-03-06T14:00:00Z"
}
```

### POST /api/v1/training/:id/stop

Stop a running training job.

**Response (200):**
```json
{
  "id": "tr_ghi789",
  "status": "stopped",
  "current_epoch": 45,
  "total_epochs": 100
}
```

### WebSocket /ws/training/:id

Live training progress via WebSocket. Connect and receive JSON messages per epoch:

```json
{
  "type": "epoch_update",
  "current_epoch": 42,
  "total_epochs": 100,
  "train_loss": 0.0125,
  "val_loss": 0.0098,
  "best_val_loss": 0.0092,
  "learning_rate": 0.001,
  "elapsed_seconds": 120,
  "estimated_remaining_seconds": 168
}
```

When training completes, a final message is sent:

```json
{
  "type": "training_complete",
  "status": "completed",
  "current_epoch": 100,
  "total_epochs": 100,
  "best_val_loss": 0.0082,
  "final_metrics": [
    { "train_loss": 0.15, "val_loss": 0.14 },
    { "train_loss": 0.08, "val_loss": 0.07 }
  ]
}
```

---

## Models

### GET /api/v1/models

List all trained models.

**Response (200):**
```json
[
  {
    "id": "mdl_def456",
    "name": "Aether -- Warren AHU-1 Anomaly Detector",
    "architecture": "lstm_autoencoder",
    "equipment_type": "air_handler",
    "parameters": 65536,
    "metrics": { "f1": 0.925, "val_loss": 0.0082 },
    "file_size_bytes": 262144,
    "quantized": true,
    "status": "ready",
    "created_at": "2026-03-06T14:30:00Z"
  }
]
```

### GET /api/v1/models/:id

Get model detail including architecture, hyperparameters, and evaluation metrics.

**Response (200):**
```json
{
  "id": "mdl_def456",
  "name": "Aether -- Warren AHU-1 Anomaly Detector",
  "architecture": "lstm_autoencoder",
  "dataset_id": "ds_abc123",
  "equipment_type": "air_handler",
  "parameters": 65536,
  "input_features": 7,
  "hidden_dim": 64,
  "bottleneck_dim": 32,
  "num_layers": 2,
  "sequence_length": 60,
  "training_run_id": "tr_ghi789",
  "metrics": {
    "reconstruction_error_threshold": 0.015,
    "val_loss": 0.0082,
    "precision": 0.94,
    "recall": 0.91,
    "f1": 0.925
  },
  "file_path": "/data/models/warren-ahu1-aether.axonml",
  "file_size_bytes": 262144,
  "quantized": true,
  "quantized_size_bytes": 65536,
  "status": "ready",
  "created_at": "2026-03-06T14:30:00Z"
}
```

### GET /api/v1/models/:id/download

Download the `.axonml` model weights file.

**Response:** `200 OK` with `Content-Type: application/octet-stream`

### DELETE /api/v1/models/:id

Delete a model.

**Response:** `204 No Content`

### POST /api/v1/models/:id/compare

Compare this model with another model.

**Request:**
```json
{ "compare_with": "mdl_other456" }
```

**Response (200):**
```json
{
  "models": [
    {
      "id": "mdl_def456",
      "name": "Model A",
      "architecture": "lstm_autoencoder",
      "metrics": { "f1": 0.925, "val_loss": 0.0082, "precision": 0.94, "recall": 0.91 }
    },
    {
      "id": "mdl_other456",
      "name": "Model B",
      "architecture": "gru_predictor",
      "metrics": { "f1": 0.910, "val_loss": 0.0095, "precision": 0.92, "recall": 0.90 }
    }
  ],
  "winner": "mdl_def456",
  "advantage": { "f1": 0.015, "val_loss": -0.0013 }
}
```

---

## Deployments

### GET /api/v1/deployments

List all deployments.

**Response (200):**
```json
[
  {
    "id": "dep_mno345",
    "model_id": "mdl_def456",
    "target_ip": "100.124.76.93",
    "target_name": "Warren AHU-1",
    "target_arch": "armv7-unknown-linux-musleabihf",
    "status": "deployed",
    "binary_size_bytes": 2097152,
    "deployed_at": "2026-03-06T15:00:00Z",
    "deployed_by": "admin"
  }
]
```

### POST /api/v1/deployments

Deploy a model to an edge target.

**Request:**
```json
{
  "model_id": "mdl_def456",
  "target_ip": "100.124.76.93",
  "target_name": "Warren AHU-1"
}
```

**Response (201):**
```json
{
  "id": "dep_new789",
  "status": "compiling",
  "model_id": "mdl_def456",
  "target_ip": "100.124.76.93"
}
```

Status progression: `compiling` -> `quantizing` -> `packaging` -> `transferring` -> `deployed`

### GET /api/v1/deployments/:id

Get deployment status.

### GET /api/v1/deployments/:id/binary

Download the cross-compiled ARM binary.

**Response:** `200 OK` with `Content-Type: application/octet-stream`

### GET /api/v1/deployments/targets

List registered edge controllers.

**Response (200):**
```json
[
  {
    "ip": "100.124.76.93",
    "name": "Warren AHU-1",
    "status": "online",
    "current_model": "mdl_def456",
    "current_model_version": "v1.2",
    "arch": "armv7-unknown-linux-musleabihf",
    "last_seen": "2026-03-06T15:30:00Z"
  }
]
```

---

## Agent

### POST /api/v1/agent/chat

Chat with the PrometheusForge AI agent.

**Request:**
```json
{
  "message": "What model architecture should I use for this AHU dataset?",
  "conversation_id": "conv_abc123"
}
```

The `conversation_id` field is optional. Omit it to start a new conversation.

**Response (200):**
```json
{
  "response": "Based on the sensor patterns in your AHU dataset, I recommend an LSTM Autoencoder...",
  "conversation_id": "conv_abc123",
  "training_plan": null
}
```

If the agent generates a training plan, the `training_plan` field will contain:
```json
{
  "training_plan": {
    "architecture": "lstm_autoencoder",
    "dataset_id": "ds_abc123",
    "hyperparameters": { "..." }
  }
}
```

### POST /api/v1/agent/analyze

Send a dataset for AI analysis.

**Request:**
```json
{
  "dataset_id": "ds_abc123",
  "question": "Analyze this dataset and recommend a model architecture."
}
```

**Response (200):**
```json
{
  "response": "Analysis of the AHU dataset reveals seasonal patterns...",
  "analysis": {
    "data_quality": "good",
    "seasonality": true,
    "anomalies_detected": 3,
    "recommended_architecture": "lstm_autoencoder",
    "recommended_hyperparameters": { "..." }
  },
  "training_plan": { "..." }
}
```

### GET /api/v1/agent/history

Get conversation history.

**Response (200):**
```json
[
  {
    "id": "conv_abc123",
    "messages": [
      { "role": "user", "content": "Analyze my AHU data", "timestamp": "2026-03-06T12:00:00Z" },
      { "role": "agent", "content": "I recommend an LSTM Autoencoder...", "timestamp": "2026-03-06T12:00:05Z" }
    ],
    "created_at": "2026-03-06T12:00:00Z"
  }
]
```

---

## Evaluations

### GET /api/v1/evaluations

List all evaluations.

### GET /api/v1/evaluations/:id

Get evaluation detail with all metrics.

**Response (200):**
```json
{
  "id": "eval_pqr678",
  "model_id": "mdl_def456",
  "metrics": {
    "accuracy": 0.952,
    "precision": 0.94,
    "recall": 0.91,
    "f1": 0.925,
    "auc_roc": 0.978,
    "val_loss": 0.0082,
    "reconstruction_error_threshold": 0.015
  },
  "confusion_matrix": {
    "true_positive": 182,
    "false_positive": 11,
    "true_negative": 1547,
    "false_negative": 18
  },
  "training_curves": {
    "epochs": [1, 2, 3, "..."],
    "train_loss": [0.15, 0.08, 0.05, "..."],
    "val_loss": [0.14, 0.07, 0.04, "..."]
  },
  "gradient_evaluation": null,
  "created_at": "2026-03-06T14:35:00Z"
}
```

### POST /api/v1/evaluations/:id/gradient

Run Gradient AI evaluation on the model.

**Response (202 Accepted):**
```json
{
  "status": "running",
  "message": "Gradient evaluation started"
}
```

After completion (poll `GET /api/v1/evaluations/:id`), the `gradient_evaluation` field will contain the 19 Gradient metrics.

---

## Billing & Subscriptions

### GET /api/v1/billing/subscription

Get the current user's subscription details.

**Response (200):**
```json
{
  "id": "user-1",
  "user_id": "user-1",
  "tier": "pro",
  "stripe_customer_id": "cus_...",
  "stripe_subscription_id": "sub_...",
  "current_period_start": "2026-03-01T00:00:00Z",
  "current_period_end": "2026-04-01T00:00:00Z",
  "token_balance": 45000,
  "tokens_used_this_period": 5000,
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-03-07T00:00:00Z"
}
```

### POST /api/v1/billing/checkout

Create a Stripe Checkout session for upgrading to a paid tier.

**Request:**
```json
{
  "tier": "pro",
  "success_url": "https://app.example.com/billing?success=true",
  "cancel_url": "https://app.example.com/billing"
}
```

**Response (200):**
```json
{ "checkout_url": "https://checkout.stripe.com/c/pay/cs_..." }
```

### POST /api/v1/billing/portal

Create a Stripe Customer Portal session for managing subscriptions.

**Response (200):**
```json
{ "portal_url": "https://billing.stripe.com/p/session/..." }
```

### GET /api/v1/billing/usage

Get current usage statistics and tier limits.

**Response (200):**
```json
{
  "tier": "pro",
  "token_balance": 45000,
  "tokens_used": 5000,
  "tokens_limit": 50000,
  "percentage_used": 10.0,
  "max_concurrent_trainings": 5,
  "max_datasets": 50,
  "max_models": 25,
  "max_deployments": 10,
  "max_dataset_size_bytes": 524288000
}
```

### POST /api/v1/billing/webhook

Stripe webhook endpoint. Verifies HMAC-SHA256 signature via `Stripe-Signature` header.

Handled events: `checkout.session.completed`, `customer.subscription.updated`, `customer.subscription.deleted`

---

## MFA (Multi-Factor Authentication)

### POST /api/v1/mfa/setup

Generate a TOTP secret and provisioning URI for authenticator apps.

**Response (200):**
```json
{
  "secret": "JBSWY3DPEHPK3PXP",
  "provisioning_uri": "otpauth://totp/Prometheus:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Prometheus",
  "qr_code": "data:image/png;base64,..."
}
```

### POST /api/v1/mfa/verify

Verify a TOTP code and enable MFA for the account.

**Request:**
```json
{ "code": "482917" }
```

**Response (200):**
```json
{ "message": "MFA enabled successfully" }
```

### POST /api/v1/mfa/disable

Disable MFA for the account.

**Request:**
```json
{ "code": "482917" }
```

**Response (200):**
```json
{ "message": "MFA disabled" }
```

### POST /api/v1/auth/mfa/validate (Public)

Validate a TOTP code during login (before full session is granted).

**Request:**
```json
{
  "username": "admin",
  "code": "482917"
}
```

**Response (200):**
```json
{ "message": "MFA validated", "token": "opaque_bearer_token" }
```

---

## User Profile

### GET /api/v1/profile

Get the authenticated user's profile.

### GET /api/v1/profile/preferences

Get user preferences (notification settings, theme, etc.).

### PUT /api/v1/profile/preferences

Update user preferences.

**Request:**
```json
{
  "theme": "nexusedge-dark",
  "notifications_enabled": true,
  "email_notifications": true,
  "training_auto_stop": false,
  "default_architecture": "lstm_autoencoder",
  "timezone": "America/New_York"
}
```

### PUT /api/v1/auth/change-password

Change the authenticated user's password.

**Request:**
```json
{
  "current_password": "old_password",
  "new_password": "new_secure_password"
}
```

**Response (200):**
```json
{ "message": "Password changed successfully" }
```

---

## Push Notifications

### POST /api/v1/push/register

Register an Expo push token for the current user.

**Request:**
```json
{
  "token": "ExponentPushToken[...]",
  "platform": "ios",
  "device_name": "iPhone 15"
}
```

**Response (200):**
```json
{ "message": "Push token registered" }
```

Push notifications are sent automatically for:
- Training queued
- Training started (from queue)
- Training complete/failed/epoch milestones
- Deployment ready
- Security alerts
- Account and subscription changes

---

## Admin User Management

### GET /api/v1/admin/users

List all users with subscription and verification status.

### POST /api/v1/admin/users

Create a new user (admin only).

**Request:**
```json
{
  "username": "newuser",
  "email": "user@example.com",
  "password": "secure_password",
  "role": "operator"
}
```

### GET /api/v1/admin/users/:username

Get user details including subscription tier, MFA status, and preferences.

### PUT /api/v1/admin/users/:username

Update user (role, email, password, etc.).

### DELETE /api/v1/admin/users/:username

Delete user and cascade cleanup across all collections (subscriptions, preferences, MFA, push tokens, verification records).

---

## System

### GET /health

Liveness probe. No authentication required.

**Response (200):**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "aegis_db": "connected",
  "uptime_seconds": 3600
}
```

### GET /api/v1/system/metrics

System resource metrics.

**Response (200):**
```json
{
  "cpu_usage_percent": 23.5,
  "memory_used_mb": 512,
  "memory_total_mb": 8192,
  "disk_used_gb": 45.2,
  "disk_total_gb": 200.0,
  "active_training_runs": 1,
  "max_concurrent_trainings": 8,
  "total_models": 12,
  "total_deployments": 29
}
```

---

## Error Responses

All error responses follow this format:

```json
{
  "error": "Error description",
  "code": "ERROR_CODE",
  "details": "Additional context (optional)"
}
```

### Common HTTP Status Codes

| Code | Meaning |
|------|---------|
| 200 | Success |
| 201 | Created |
| 202 | Accepted (async operation started) |
| 204 | No Content (successful deletion) |
| 400 | Bad Request (invalid input) |
| 401 | Unauthorized (missing or invalid token) |
| 403 | Forbidden (insufficient role permissions) |
| 404 | Not Found |
| 429 | Too Many Requests (rate limit exceeded) |
| 502 | Bad Gateway (external data source connection failed) |
| 504 | Gateway Timeout (external data source query timed out) |
| 500 | Internal Server Error |

### Rate Limits

| Endpoint Category | Limit |
|-------------------|-------|
| Login | 30 requests/min per IP |
| General API | 1000 requests/min per user |
| Agent | 20 requests/min per user |
