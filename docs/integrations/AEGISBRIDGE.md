# Prometheus + AegisBridge Integration

Connect edge controllers running Aegis-DB to stream live data into Prometheus.

## Overview

AegisBridge is the native/default data ingestion path for Prometheus. Each edge controller (Raspberry Pi, Jetson, or any device) runs Aegis-DB locally. AegisBridge continuously syncs data from the edge Aegis-DB to the cloud Prometheus Aegis-DB instance. This is the lowest-latency, most tightly integrated way to get data into Prometheus.

## Architecture

```
Edge Controller (Pi/Jetson)             Prometheus (Cloud)
+-------------------------+            +-------------------------+
| Data Source              |            |                         |
| (sensors, cameras,      |            |                         |
|  instruments, APIs)     |            |                         |
|   |                     |            |                         |
|   v                     |            |                         |
| Aegis-DB (edge)         | AegisBridge| Aegis-DB (cloud)       |
| port 9090               |<==========>| port 9091               |
+-------------------------+            +-------------------------+
```

## Prerequisites

- Edge device running Aegis-DB (port 9090)
- Data collection daemon writing to the edge Aegis-DB
- Network connectivity between edge and Prometheus server (Tailscale, VPN, or direct)

## Quick Start

### Web UI

1. Go to **Datasets** page
2. Click **Connect Source**
3. Select **AegisBridge (Edge Controller)** from the dropdown
4. Fill in:
   - **Controller IP**: The edge device's IP address (e.g., `100.124.76.93`)
   - **Port**: Aegis-DB port (default `9090`)
   - **Collection**: The Aegis-DB collection name on the edge device
5. Click **Connect**

### API

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "aegis_bridge",
    "controller_ip": "100.124.76.93",
    "aegis_port": 9090,
    "collection": "sensor_readings",
    "limit": 50000
  }'
```

## Configuration Reference

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `source_type` | Yes | — | Must be `"aegis_bridge"` |
| `controller_ip` | Yes | — | IP address of the edge controller |
| `aegis_port` | No | `9090` | Aegis-DB HTTP port on the edge device |
| `collection` | No | `"hardware_metrics"` | Aegis-DB collection name |
| `limit` | No | `10000` | Maximum documents to import |

## How It Works

1. Prometheus connects to the edge Aegis-DB:
   ```
   GET http://{controller_ip}:{aegis_port}/api/v1/documents/collections/{collection}/documents?limit={limit}
   ```
2. Edge Aegis-DB returns JSON documents from the collection
3. Prometheus flattens documents to CSV (union of all keys across documents)
4. CSV is stored as a new Prometheus dataset with auto-computed statistics
5. The dataset appears in your Datasets list with source = `aegis_bridge`

## Setting Up the Edge Device

### 1. Install Aegis-DB on the Edge

```bash
# On Raspberry Pi or edge device
cd /opt
git clone https://github.com/AutomataNexus/Aegis-DB.git
cd Aegis-DB
cargo build --release
./target/release/aegis-db --port 9090
```

### 2. Write Data to Edge Aegis-DB

Your data collection daemon writes documents to the edge Aegis-DB:

```python
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

def collect_and_store(reading):
    """Write a single reading to edge Aegis-DB."""
    doc_id = f"reading_{int(time.time() * 1000)}"
    requests.post(
        f"{EDGE_AEGIS}/documents/collections/sensor_readings/documents",
        json={
            "id": doc_id,
            "document": reading
        }
    )
```

### 3. Connect from Prometheus

Once data is flowing into the edge Aegis-DB, connect from the Prometheus UI or API.

## Edge Data Collection Examples

### Medical — Patient Vitals Monitor

```python
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

while True:
    reading = {
        "timestamp": time.time(),
        "heart_rate": read_ecg(),
        "spo2": read_pulse_oximeter(),
        "systolic_bp": read_bp()[0],
        "diastolic_bp": read_bp()[1],
        "respiratory_rate": read_resp(),
        "body_temp": read_temp(),
        "patient_id": "P-00142"
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/vitals/documents", json={
        "id": f"vital_{int(time.time()*1000)}",
        "document": reading
    })
    time.sleep(1)
```

### Finance — Market Data Collector

```python
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

def on_tick(ticker, price, volume, bid, ask):
    requests.post(f"{EDGE_AEGIS}/documents/collections/market_ticks/documents", json={
        "id": f"tick_{ticker}_{int(time.time()*1000)}",
        "document": {
            "timestamp": time.time(),
            "ticker": ticker,
            "price": price,
            "volume": volume,
            "bid": bid,
            "ask": ask,
            "spread": ask - bid
        }
    })
```

### Industrial — Vibration & Thermal Monitoring

```python
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

while True:
    reading = {
        "timestamp": time.time(),
        "machine_id": "CNC-04",
        "vibration_rms": read_accelerometer_rms(),
        "vibration_peak_freq": read_fft_peak(),
        "bearing_temp_c": read_thermal_camera("bearing"),
        "motor_temp_c": read_thermal_camera("motor"),
        "spindle_rpm": read_tachometer(),
        "power_kw": read_power_meter()
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/machine_health/documents", json={
        "id": f"mh_{int(time.time()*1000)}",
        "document": reading
    })
    time.sleep(0.5)
```

### Satellite — Ground Station Receiver

```python
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

def on_telemetry_frame(frame):
    requests.post(f"{EDGE_AEGIS}/documents/collections/sat_telemetry/documents", json={
        "id": f"sat_{frame.norad_id}_{int(time.time())}",
        "document": {
            "timestamp": time.time(),
            "satellite_id": frame.norad_id,
            "latitude": frame.lat,
            "longitude": frame.lon,
            "altitude_km": frame.alt,
            "signal_strength_db": frame.rssi,
            "doppler_shift_hz": frame.doppler,
            "band_id": frame.band
        }
    })
```

### Agriculture — Soil & Climate Sensors

```python
import requests, time

EDGE_AEGIS = "http://localhost:9090/api/v1"

while True:
    reading = {
        "timestamp": time.time(),
        "soil_moisture": read_soil_moisture(),
        "soil_temp_c": read_soil_temp(),
        "soil_ph": read_soil_ph(),
        "air_temp_c": read_air_temp(),
        "humidity_pct": read_humidity(),
        "light_lux": read_light_sensor(),
        "rainfall_mm": read_rain_gauge(),
        "field_zone": "ZONE-A3"
    }
    requests.post(f"{EDGE_AEGIS}/documents/collections/field_readings/documents", json={
        "id": f"field_{int(time.time()*1000)}",
        "document": reading
    })
    time.sleep(60)
```

## Ingestion Keys

For automated edge-to-cloud data push (without user auth tokens), use **Ingestion Keys**:

1. Go to **Datasets** page
2. Find the **Ingestion Keys** panel
3. Click **Generate Key**
4. Use the key on your edge device:

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets \
  -H "X-Ingestion-Key: ink_abc123..." \
  -F "file=@readings.csv" \
  -F "name=edge_controller_readings" \
  -F "domain=industrial"
```

## Security

- Controller IP is validated against SSRF attacks (blocks localhost, private ranges depending on config)
- Edge Aegis-DB should be on a private network (Tailscale, WireGuard, VPN)
- Ingestion keys provide scoped, revocable access for automated uploads
- All data transfer happens server-side

## Networking

| Component | Port | Protocol |
|-----------|------|----------|
| Edge Aegis-DB | 9090 | HTTP |
| Cloud Aegis-DB | 9091 | HTTP |
| Prometheus Server | 3030 | HTTP + WebSocket |

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Connection refused | Check edge device IP and port; verify Aegis-DB is running |
| SSRF blocked | Private IPs may be blocked; use Tailscale IPs (100.x.x.x) |
| Empty collection | Verify your data daemon is writing to the correct collection name |
| Network unreachable | Ensure VPN/Tailscale is connected between cloud and edge |
| Timeout (30s) | Reduce the `limit` parameter or check network latency |
