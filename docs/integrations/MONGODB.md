# Prometheus + MongoDB Integration

Connect your MongoDB collections to Prometheus for ML model training.

## Overview

Prometheus connects to MongoDB via the **MongoDB Data API** (Atlas App Services). Documents are fetched, flattened to tabular format, normalized to CSV, and stored as a Prometheus dataset. Works with MongoDB Atlas, self-hosted MongoDB with Data API enabled, or any MongoDB REST proxy.

## Prerequisites

- MongoDB Atlas cluster with Data API enabled, or self-hosted MongoDB with a REST interface
- Data API key or application ID
- Network connectivity from Prometheus server to your MongoDB endpoint

## Quick Start

### Web UI

1. Go to **Datasets** page
2. Click **Connect Source**
3. Select **MongoDB (Data API)** from the dropdown
4. Fill in:
   - **Data API Endpoint**: Your MongoDB Data API URL
   - **Token / API Key**: Your Data API key
   - **Database**: Database name
   - **Collection**: Collection name
5. Click **Connect**

### API

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "mongodb",
    "api_url": "https://data.mongodb-api.com/app/data-xxxxx/endpoint/data/v1",
    "api_key": "your-mongodb-data-api-key",
    "database": "production",
    "collection": "sensor_readings",
    "limit": 50000
  }'
```

## Configuration Reference

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `source_type` | Yes | — | Must be `"mongodb"` |
| `api_url` | Yes | — | MongoDB Data API endpoint URL |
| `api_key` | No | — | Data API key (sent as `api-key` header) |
| `database` | Yes | — | Database name |
| `collection` | Yes | — | Collection name |
| `filter` | No | `{}` | MongoDB query filter (JSON object) |
| `limit` | No | `10000` | Maximum documents to import |

## How It Works

1. Prometheus sends a `find` request to the MongoDB Data API:
   ```
   POST {api_url}/action/find
   Headers: { "api-key": "{api_key}" }
   Body: {
     "dataSource": "{database}",
     "database": "{database}",
     "collection": "{collection}",
     "filter": {filter},
     "limit": {limit},
     "sort": { "timestamp": -1 }
   }
   ```
2. MongoDB returns a `{ "documents": [...] }` response
3. Prometheus flattens each document into a tabular row:
   - Top-level fields become columns
   - `_id` fields are excluded
   - Nested objects are flattened or stringified
   - Values with commas/quotes are properly escaped
4. The resulting CSV is stored as a Prometheus dataset

## Example: Genomics Data

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "mongodb",
    "api_url": "https://data.mongodb-api.com/app/genomics-app/endpoint/data/v1",
    "api_key": "genome-api-key-xxx",
    "database": "genome_db",
    "collection": "variants",
    "filter": {"chromosome": "chr1", "quality": {"$gte": 30}},
    "limit": 100000
  }'
```

## Example: E-commerce Reviews (NLP)

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "mongodb",
    "api_url": "https://data.mongodb-api.com/app/store-app/endpoint/data/v1",
    "api_key": "store-api-key-xxx",
    "database": "ecommerce",
    "collection": "reviews",
    "filter": {"rating": {"$exists": true}},
    "limit": 50000
  }'
```

## Example: IoT Device Logs

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "mongodb",
    "api_url": "https://data.mongodb-api.com/app/iot-app/endpoint/data/v1",
    "api_key": "iot-api-key-xxx",
    "database": "iot_platform",
    "collection": "device_metrics",
    "filter": {"device_type": "temperature_sensor", "timestamp": {"$gte": {"$date": "2024-01-01T00:00:00Z"}}},
    "limit": 200000
  }'
```

## Enabling MongoDB Data API

### Atlas (Cloud)

1. Go to **Atlas App Services** in your MongoDB Atlas project
2. Click **Create a New App** (or use existing)
3. Enable **Data API** under the app's services
4. Create an **API Key** under Authentication > API Keys
5. Note your **Data API URL** from the App Services dashboard

### Self-Hosted

For self-hosted MongoDB, you can use:
- **MongoDB Atlas Device Sync** with Data API
- **mongo-express** REST API
- A custom REST proxy (Express.js + Mongoose)

## Security

- API URL is validated against SSRF attacks
- API keys are sent only in the import request and not stored
- The `_id` field is automatically excluded from imported data
- All data transfer is server-side (credentials never reach the browser)

## Data Flattening

MongoDB documents are hierarchical (nested objects, arrays). Prometheus flattens them:

| MongoDB Document | CSV Columns |
|-----------------|-------------|
| `{"temp": 23.5, "loc": "room1"}` | `temp`, `loc` |
| `{"readings": [1,2,3]}` | `readings` (stringified as `"[1,2,3]"`) |
| `{"meta": {"unit": "C"}}` | `meta` (stringified as `"{"unit":"C"}"`) |

For best results, store data in flat documents with scalar values.

## Troubleshooting

| Issue | Solution |
|-------|----------|
| 401 Unauthorized | Check your API key is correct and has read permissions |
| "documents" missing | Verify your Data API URL ends with `/endpoint/data/v1` |
| Empty results | Check your filter matches existing documents |
| Connection timeout | Verify the API URL is reachable from Prometheus server |
| Nested data not useful | Flatten documents in MongoDB before importing, or use aggregation pipeline |
