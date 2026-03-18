# Prometheus — Data Ingestion Guide

## Overview

Prometheus accepts data from any domain — medical imaging, financial time-series, NLP corpora, genomics, satellite imagery, audio, industrial sensors, autonomous vehicles, and more. This guide covers all ingestion methods.

## Ingestion Methods

| Method | Best For | Protocol |
|--------|----------|----------|
| CSV Upload (Web UI) | Single files, manual upload | `POST /api/v1/datasets` multipart |
| CSV Upload (CLI) | Batch uploads from terminal | `prometheus upload <file>` |
| AegisBridge | Live edge controller data streams | Aegis-to-Aegis sync |
| External Sources | Existing databases (Postgres, InfluxDB, etc.) | `POST /api/v1/datasets/connect` |
| Bulk Ingest | Entire dataset collections | `tools/bulk_ingest/ingest.sh` |

---

## 1. CSV Upload (API)

Upload any CSV file directly:

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@heart_failure.csv" \
  -F "name=heart_failure_prediction" \
  -F "domain=medical" \
  -F "tags=medical,classification,heart"
```

Response:
```json
{
  "status": "created",
  "dataset_id": "ds_a1b2c3d4",
  "name": "heart_failure_prediction",
  "rows": 918,
  "columns": ["Age", "Sex", "ChestPainType", "RestingBP", "Cholesterol", ...],
  "file_size_bytes": 35942
}
```

**Supported file types:** CSV, TSV (comma or tab separated).

**Append-on-duplicate:** If a dataset with the same name already exists for the user, new rows are appended automatically instead of creating a duplicate.

---

## 2. AegisBridge — Edge Controller Data Streams

AegisBridge syncs data from edge Aegis-DB instances (running on Raspberry Pi or other edge controllers) to the cloud Prometheus Aegis-DB. This works for **any data type** — not just sensors.

### Architecture

```
Edge Controller (Pi)                    Prometheus (Cloud)
┌─────────────────────┐                ┌─────────────────────┐
│ Data Source          │                │                     │
│ (sensors, cameras,  │                │                     │
│  instruments, APIs) │                │                     │
│   │                 │                │                     │
│   v                 │                │                     │
│ Aegis-DB (edge)     │  AegisBridge   │ Aegis-DB (cloud)    │
│ (port 9090)         │◄══════════════►│ (port 9091)         │
│                     │                │                     │
└─────────────────────┘                └─────────────────────┘
```

### Connect an Edge Controller

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "aegis_bridge",
    "controller_ip": "100.124.76.93",
    "aegis_port": 9090,
    "collection": "sensor_readings",
    "limit": 10000
  }'
```

### Example: Edge Data Collection by Domain

#### Medical — Patient Monitoring

```python
# Edge device: Raspberry Pi connected to pulse oximeter, ECG, BP monitor
import requests, time, json

EDGE_AEGIS = "http://localhost:9090/api/v1"

while True:
    reading = {
        "timestamp": time.time(),
        "heart_rate": read_ecg_sensor(),        # BPM
        "spo2": read_pulse_oximeter(),           # Blood oxygen %
        "systolic_bp": read_bp_monitor()[0],     # mmHg
        "diastolic_bp": read_bp_monitor()[1],
        "respiratory_rate": read_resp_sensor(),
        "body_temp": read_temp_sensor(),         # Celsius
        "patient_id": "P-00142",
        "ward": "ICU-3"
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/vitals/documents", json={
        "id": f"vital_{int(time.time()*1000)}",
        "document": reading
    })
    time.sleep(1)

# Cloud: connect_source with collection="vitals"
```

#### Financial — Market Data Feed

```python
# Edge device: Low-latency market data collector
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

def on_tick(ticker, price, volume, bid, ask):
    tick = {
        "timestamp": time.time(),
        "ticker": ticker,
        "price": price,
        "volume": volume,
        "bid": bid,
        "ask": ask,
        "spread": ask - bid,
        "exchange": "NYSE"
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/market_ticks/documents", json={
        "id": f"tick_{ticker}_{int(time.time()*1000)}",
        "document": tick
    })

# Cloud: connect_source with collection="market_ticks"
```

#### Satellite — Ground Station Telemetry

```python
# Edge device: Satellite ground station receiver
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

def on_telemetry_frame(frame):
    record = {
        "timestamp": time.time(),
        "satellite_id": frame.norad_id,
        "latitude": frame.lat,
        "longitude": frame.lon,
        "altitude_km": frame.alt,
        "signal_strength_db": frame.rssi,
        "doppler_shift_hz": frame.doppler,
        "band_id": frame.spectral_band,
        "pixel_values": frame.pixel_data[:100],  # First 100 pixels
        "cloud_cover_pct": frame.cloud_mask_pct
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/sat_telemetry/documents", json={
        "id": f"sat_{frame.norad_id}_{int(time.time())}",
        "document": record
    })

# Cloud: connect_source with collection="sat_telemetry"
```

#### Autonomous Vehicles — Sensor Fusion

```python
# Edge device: Vehicle compute unit (Pi CM4 or Jetson)
import requests, time, base64

EDGE_AEGIS = "http://localhost:9090/api/v1"

def on_sensor_frame(lidar, camera, imu, gps):
    record = {
        "timestamp": time.time(),
        "gps_lat": gps.latitude,
        "gps_lon": gps.longitude,
        "speed_mps": gps.speed,
        "heading_deg": imu.heading,
        "accel_x": imu.accel[0],
        "accel_y": imu.accel[1],
        "accel_z": imu.accel[2],
        "gyro_x": imu.gyro[0],
        "gyro_y": imu.gyro[1],
        "gyro_z": imu.gyro[2],
        "lidar_points": len(lidar.points),
        "lidar_min_range": lidar.min_range,
        "lidar_max_range": lidar.max_range,
        "objects_detected": camera.detections,
        "lane_offset_m": camera.lane_offset,
        "vehicle_id": "AV-007"
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/av_sensor/documents", json={
        "id": f"av_{int(time.time()*1000)}",
        "document": record
    })

# Cloud: connect_source with collection="av_sensor"
```

#### Genomics — DNA Sequencer Output

```python
# Edge device: Pi connected to MinION nanopore sequencer
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

def on_read(read):
    record = {
        "timestamp": time.time(),
        "read_id": read.id,
        "sequence": read.sequence[:500],  # First 500 bases
        "quality_scores": read.quality[:500],
        "length": len(read.sequence),
        "mean_quality": read.mean_qscore,
        "channel": read.channel_id,
        "start_time": read.start_time,
        "barcode": read.barcode or "unclassified",
        "sample_id": "SAMPLE_042"
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/dna_reads/documents", json={
        "id": f"read_{read.id}",
        "document": record
    })

# Cloud: connect_source with collection="dna_reads"
```

#### Audio — Environmental Monitoring

```python
# Edge device: Pi with USB microphone array
import requests, time, numpy as np

EDGE_AEGIS = "http://localhost:9090/api/v1"

def on_audio_segment(audio_data, sr=16000):
    # Extract features (not raw audio)
    fft = np.fft.rfft(audio_data)
    magnitudes = np.abs(fft)

    record = {
        "timestamp": time.time(),
        "duration_sec": len(audio_data) / sr,
        "sample_rate": sr,
        "rms_energy": float(np.sqrt(np.mean(audio_data**2))),
        "peak_amplitude": float(np.max(np.abs(audio_data))),
        "spectral_centroid": float(np.sum(magnitudes * np.arange(len(magnitudes))) / np.sum(magnitudes)),
        "zero_crossing_rate": float(np.sum(np.diff(np.sign(audio_data)) != 0) / len(audio_data)),
        "mfcc_0": float(compute_mfcc(audio_data, sr)[0]),
        "mfcc_1": float(compute_mfcc(audio_data, sr)[1]),
        "location": "urban_park_station_7",
        "classification": classify_sound(audio_data)  # bird, traffic, rain, etc.
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/audio_features/documents", json={
        "id": f"audio_{int(time.time()*1000)}",
        "document": record
    })

# Cloud: connect_source with collection="audio_features"
```

#### NLP — Document Processing Pipeline

```python
# Edge device: Document scanner + OCR processor
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

def on_document_scanned(page_image, ocr_result):
    record = {
        "timestamp": time.time(),
        "document_id": ocr_result.doc_id,
        "page_number": ocr_result.page,
        "text": ocr_result.text[:2000],
        "language": ocr_result.detected_language,
        "confidence": ocr_result.confidence,
        "word_count": len(ocr_result.text.split()),
        "entities_found": ocr_result.entities,  # Named entities
        "sentiment_score": ocr_result.sentiment,
        "category": ocr_result.classification,
        "source": "scanner_lobby_east"
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/scanned_docs/documents", json={
        "id": f"doc_{ocr_result.doc_id}_p{ocr_result.page}",
        "document": record
    })

# Cloud: connect_source with collection="scanned_docs"
```

#### Industrial — Manufacturing Quality Control

```python
# Edge device: Pi connected to vibration sensors + thermal camera
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

def on_inspection(machine_id, vibration, thermal, output_quality):
    record = {
        "timestamp": time.time(),
        "machine_id": machine_id,
        "vibration_rms": vibration.rms,
        "vibration_peak": vibration.peak_freq,
        "vibration_crest_factor": vibration.crest_factor,
        "bearing_temp_c": thermal.bearing_temp,
        "motor_temp_c": thermal.motor_temp,
        "ambient_temp_c": thermal.ambient_temp,
        "spindle_speed_rpm": vibration.rpm,
        "power_draw_kw": read_power_meter(),
        "output_dimension_mm": output_quality.dimension,
        "surface_roughness_ra": output_quality.roughness,
        "defect_detected": output_quality.has_defect,
        "defect_type": output_quality.defect_class or "none",
        "production_line": "LINE-4B"
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/qc_inspections/documents", json={
        "id": f"qc_{machine_id}_{int(time.time()*1000)}",
        "document": record
    })

# Cloud: connect_source with collection="qc_inspections"
```

---

## 3. External Data Sources

Connect to 6 external database types. Data is normalized to CSV and stored as a Prometheus dataset.

### InfluxDB

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "influxdb",
    "host": "https://us-east-1.aws.cloud2.influxdata.com",
    "database": "weather_stations",
    "token": "your-influxdb-token",
    "query": "SELECT time, temperature, humidity, wind_speed, pressure FROM weather_data WHERE time > now() - 30d"
  }'
```

### PostgreSQL

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "postgresql",
    "host": "db.example.com",
    "port": 5432,
    "database": "clinical_trials",
    "username": "readonly",
    "password": "secret",
    "query": "SELECT patient_id, age, biomarker_a, biomarker_b, outcome FROM trial_data"
  }'
```

### MongoDB (Data API)

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "mongodb",
    "endpoint": "https://data.mongodb-api.com/app/data-xxxxx/endpoint/data/v1",
    "api_key": "your-mongodb-api-key",
    "database": "genomics",
    "collection": "dna_sequences",
    "filter": {"organism": "homo_sapiens"},
    "limit": 50000
  }'
```

### SQLite3

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "sqlite",
    "file_path": "/data/earthquake_catalog.sqlite3",
    "query": "SELECT latitude, longitude, depth_km, magnitude, event_time FROM quakes WHERE magnitude > 3.0"
  }'
```

### SpaceTimeDB

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "spacetimedb",
    "host": "https://spacetimedb.example.com",
    "database": "robotics_sim",
    "query": "SELECT timestamp, joint_angles, torques, end_effector_pos FROM robot_trajectories"
  }'
```

---

## 4. Bulk Ingestion Pipeline

For ingesting entire dataset collections (like the 28GB across 30 domains in `/opt/datasets/`):

```bash
# Ingest all domains
./tools/bulk_ingest/ingest.sh \
  --server https://prometheus.automatanexus.com \
  --token $TOKEN

# Ingest a single domain
./tools/bulk_ingest/ingest.sh \
  --server https://prometheus.automatanexus.com \
  --token $TOKEN \
  --domain finance

# Dry run (show what would happen)
./tools/bulk_ingest/ingest.sh \
  --server https://prometheus.automatanexus.com \
  --token $TOKEN \
  --dry-run

# Compress only (no upload)
./tools/bulk_ingest/ingest.sh --compress-only
```

### Processing Strategy by File Type

| File Type | Strategy | Expected Compression |
|-----------|----------|---------------------|
| CSV (text/numeric) | Direct upload or OZL compress | 75-90% |
| JSON/JSONL | Direct upload or OZL compress | 70-85% |
| Images (JPG/PNG) | Tar directory → OZL compress | 5-25% (already compressed) |
| Images (TIF/BMP) | Tar directory → OZL compress | 30-50% (uncompressed raster) |
| Audio (WAV) | Tar directory → OZL compress | 20-35% |
| Parquet | Direct upload | 5-10% (already columnar compressed) |
| Excel (XLSX) | Direct upload | 5-15% (already ZIP compressed) |

### Dictionary Training for Better Compression

For CSV-heavy domains, training a domain-specific dictionary improves compression:

```bash
# Train a dictionary on financial CSVs
prometheus train-compressor \
  /opt/datasets/finance/credit_fraud/creditcard.csv \
  /opt/datasets/stocks/world_prices/World-Stock-Prices-Dataset.csv \
  --output /opt/datasets/.dictionaries/finance.ozl-dict

# Compress with dictionary
prometheus compress /opt/datasets/finance/credit_fraud/creditcard.csv \
  --dict /opt/datasets/.dictionaries/finance.ozl-dict
```

---

## 5. OpenZL Compression

OpenZL (`.ozl`) is Prometheus's custom compression format wrapping zstd with integrity verification.

### Format Specification

```
[4 bytes]  Magic: "OZL\x01"
[1 byte]   Version: 0x01
[1 byte]   Flags: bit 0 = has dictionary
[32 bytes] SHA-256 hash of original data
[8 bytes]  Original size (u64 little-endian)
[8 bytes]  Dictionary ID (u64 little-endian, 0 if none)
[...]      zstd-compressed payload
```

### CLI Commands

```bash
# Compress a file
prometheus compress dataset.csv
# → dataset.csv.ozl

# Compress with custom output path
prometheus compress dataset.csv --output archive/dataset.ozl

# Compress with trained dictionary
prometheus compress dataset.csv --dict finance.ozl-dict

# Decompress
prometheus decompress dataset.csv.ozl
# → dataset.csv (original restored, SHA-256 verified)

# Train a compression dictionary
prometheus train-compressor file1.csv file2.csv file3.csv --output domain.ozl-dict
```

### Compression Ratios by Data Type

Tested on actual Prometheus datasets (zstd level 15):

| Dataset | Domain | Original | Compressed | Reduction |
|---------|--------|----------|------------|-----------|
| ECG heartbeat (556MB tar) | ecg | 569 MB | 63 MB | **89%** |
| Stock prices (96MB tar) | stocks | 98 MB | 29 MB | **80%** |
| NLP reviews (93MB tar) | nlp | 97 MB | 28 MB | **80%** |
| Akkadian text (1.6MB) | linguistics | 1.6 MB | 0.4 MB | **75%** |
| Credit fraud (144MB) | finance | 151 MB | 66 MB | **56%** |
| Climate data (28KB tar) | climate | 20 KB | 4 KB | **79%** |
| Medical CSV (780KB tar) | medical | 788 KB | 319 KB | **60%** |

**Achieving >86% compression:**
- Tar many similar files together (e.g., all CSVs in a domain)
- Train domain-specific dictionaries
- Numeric time-series data compresses best (ECG: 89%)
- Text data compresses well (NLP: 80%)
- Already-compressed formats (JPG, MP3) won't compress further

---

## 6. Dataset Validation & Locking

After ingestion, validate and lock datasets before training:

```bash
# Validate dataset
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/ds_a1b2c3d4/validate \
  -H "Authorization: Bearer $TOKEN"

# Response shows validation results:
# - Column width consistency
# - Missing value detection
# - Type inference per column
# - Auto-compresses with zstd on successful validation
```

Validated datasets are locked (frozen) and auto-compressed. Training refuses to start on unvalidated datasets.
