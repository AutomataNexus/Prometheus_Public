# Prometheus + SQLite Integration

Connect your SQLite databases to Prometheus for ML model training.

## Overview

Prometheus can import data from SQLite3 database files accessible on the server filesystem. The query is executed through Aegis-DB's federated query engine or a direct REST API, and results are normalized to CSV and stored as a Prometheus dataset.

## Prerequisites

- SQLite3 database file (`.sqlite3`, `.db`, `.sqlite`) accessible on the Prometheus server
- Read access to the file path
- Or: a REST API that wraps your SQLite database

## Quick Start

### Web UI

1. Go to **Datasets** page
2. Click **Connect Source**
3. Select **SQLite3** from the dropdown
4. Fill in:
   - **File Path**: Absolute path to the SQLite file on the server (e.g., `/data/earthquakes.sqlite3`)
   - **Query**: SQL SELECT query
5. Click **Connect**

### API (File Path)

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "sqlite",
    "connection_string": "/data/earthquake_catalog.sqlite3",
    "query": "SELECT latitude, longitude, depth_km, magnitude, event_time FROM quakes WHERE magnitude > 3.0",
    "limit": 100000
  }'
```

### API (REST API)

If your SQLite database is exposed via a REST API (e.g., Datasette, sqlite-web):

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "sqlite",
    "api_url": "https://datasette.example.com/mydb/query",
    "query": "SELECT * FROM measurements WHERE date >= '\''2024-01-01'\''",
    "limit": 50000
  }'
```

## Configuration Reference

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `source_type` | Yes | — | Must be `"sqlite"` |
| `connection_string` | * | — | Absolute file path to SQLite database |
| `api_url` | * | — | REST API URL (e.g., Datasette endpoint) |
| `query` | Yes | — | SQL SELECT query |
| `limit` | No | `10000` | Maximum rows to import |

*Provide either `connection_string` (file path) or `api_url` (REST).

## How It Works

### File Path Mode

1. Prometheus sends the file path and query to Aegis-DB's federated query engine
2. Aegis-DB opens the SQLite file, executes the SELECT query
3. Results are returned as JSON rows
4. Rows are normalized to CSV and stored as a dataset

### REST API Mode

1. Prometheus sends the query to your REST endpoint
2. Expects JSON array of row objects in the response
3. Normalizes to CSV and stores as a dataset

## Example: Earthquake Catalog

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "sqlite",
    "connection_string": "/opt/datasets/seismology/earthquake_catalog.sqlite3",
    "query": "SELECT latitude, longitude, depth, magnitude, mag_type, event_time, region FROM events WHERE magnitude >= 2.5 ORDER BY event_time DESC",
    "limit": 50000
  }'
```

## Example: Research Dataset

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "sqlite",
    "connection_string": "/home/researcher/experiments.db",
    "query": "SELECT trial_id, concentration, temperature, ph, yield_pct, catalyst FROM reactions",
    "limit": 10000
  }'
```

## Use Cases

SQLite is ideal for:
- **Kaggle datasets** downloaded as `.sqlite` files
- **Research databases** stored locally
- **Embedded device logs** synced from edge controllers
- **Application databases** from mobile or desktop apps
- **Pre-processed datasets** already in tabular form

## Security

- File paths are validated by the Shield security engine (no path traversal)
- SQL queries pass through a SQL firewall (only SELECT allowed)
- Only files on the Prometheus server filesystem are accessible
- REST API URLs are validated against SSRF

## Troubleshooting

| Issue | Solution |
|-------|----------|
| File not found | Verify the absolute path is correct and accessible |
| Permission denied | Ensure the Prometheus server process has read access |
| "database is locked" | Close other connections to the SQLite file |
| SQL blocked | Only SELECT queries are allowed |
| Empty results | Check your WHERE clause matches existing data |
