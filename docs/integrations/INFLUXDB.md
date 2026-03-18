# Prometheus + InfluxDB Integration

Connect your InfluxDB time-series data to Prometheus for ML model training.

## Overview

Prometheus connects to InfluxDB via the **v3 SQL API** (`/api/v3/query_sql`). Data is queried, normalized to CSV, and stored as a Prometheus dataset in Aegis-DB. Supports InfluxDB Cloud, InfluxDB OSS 2.x+, and InfluxDB 3.0.

## Prerequisites

- InfluxDB instance with HTTP API access
- API token with read permissions on your bucket/database
- Network connectivity from Prometheus server to your InfluxDB endpoint

## Quick Start

### Web UI

1. Go to **Datasets** page
2. Click **Connect Source**
3. Select **InfluxDB** from the dropdown
4. Fill in:
   - **Host**: Your InfluxDB URL (e.g., `https://us-east-1.aws.cloud2.influxdata.com`)
   - **Database**: Your database/bucket name
   - **Token / API Key**: Your InfluxDB API token
   - **Query**: SQL query to pull data
5. Click **Connect**

### API

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "influxdb",
    "url": "https://us-east-1.aws.cloud2.influxdata.com",
    "database": "my_bucket",
    "measurement": "sensor_readings",
    "limit": 50000
  }'
```

### CLI

```bash
prometheus agent "connect my InfluxDB at https://influx.example.com, database=metrics, measurement=cpu_usage"
```

## Configuration Reference

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `source_type` | Yes | — | Must be `"influxdb"` |
| `url` | Yes | — | InfluxDB HTTP endpoint URL |
| `database` | No | `"NexusEdge"` | Database/bucket name |
| `measurement` | No | `"ProcessingEngineCommands"` | Measurement/table to query |
| `limit` | No | `10000` | Maximum rows to import |

## How It Works

1. Prometheus sends a SQL query to InfluxDB's v3 SQL API:
   ```
   POST {url}/api/v3/query_sql
   Body: { "q": "SELECT * FROM \"{measurement}\" ORDER BY time DESC LIMIT {limit}", "db": "{database}" }
   ```
2. InfluxDB returns JSON rows
3. Prometheus normalizes the JSON to CSV (flattening nested fields, escaping values)
4. The CSV is stored as a new Prometheus dataset with computed statistics
5. Dataset appears in your Datasets list, ready for validation and training

## Example: Weather Station Data

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "influxdb",
    "url": "https://us-east-1.aws.cloud2.influxdata.com",
    "database": "weather_stations",
    "measurement": "ambient",
    "limit": 100000
  }'
```

This imports up to 100,000 rows from the `ambient` measurement, including columns like `temperature`, `humidity`, `wind_speed`, `pressure`, `timestamp`.

## Example: Industrial IoT Metrics

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "influxdb",
    "url": "http://influxdb.factory.local:8086",
    "database": "production_line",
    "measurement": "machine_metrics",
    "limit": 200000
  }'
```

## Security

- InfluxDB URL is validated against SSRF attacks before any request is made
- API tokens are used only for the import request and not stored persistently
- All data transfer happens server-side (your InfluxDB credentials never reach the browser)

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Connection timeout | Verify InfluxDB URL is reachable from the Prometheus server |
| 401 Unauthorized | Check your API token has read permissions |
| No data returned | Verify the measurement name and database exist |
| "Unsupported source_type" | Ensure `source_type` is exactly `"influxdb"` |

## Supported InfluxDB Versions

| Version | Support | API |
|---------|---------|-----|
| InfluxDB 3.0 | Full | v3 SQL API |
| InfluxDB Cloud (Serverless) | Full | v3 SQL API |
| InfluxDB 2.x | Via Flux-to-SQL | v3 SQL API (if enabled) |
| InfluxDB 1.x | Not supported | Requires v3 API |
