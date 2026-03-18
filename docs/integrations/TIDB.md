# Prometheus + TiDB Integration

Connect your TiDB (MySQL-compatible) database to Prometheus for ML model training.

## Overview

TiDB is a distributed SQL database compatible with the MySQL protocol. Prometheus connects to TiDB using the same SQL integration path as PostgreSQL — either through Aegis-DB's federated query engine or a direct REST API.

## Prerequisites

- TiDB cluster (TiDB Cloud or self-hosted)
- Connection string or REST API endpoint
- Read-only user credentials (recommended)
- Network connectivity from Prometheus server to your TiDB endpoint

## Quick Start

### Web UI

1. Go to **Datasets** page
2. Click **Connect Source**
3. Select **TiDB** from the dropdown
4. Fill in:
   - **Host**: Your TiDB hostname
   - **Port**: `4000` (TiDB default)
   - **Database**: Database name
   - **Query**: SQL SELECT query
5. Click **Connect**

### API

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "tidb",
    "connection_string": "mysql://readonly:password@tidb.example.com:4000/analytics",
    "query": "SELECT user_id, session_duration, pages_viewed, conversion, device_type FROM user_sessions",
    "limit": 100000
  }'
```

## Configuration Reference

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `source_type` | Yes | — | Must be `"tidb"` |
| `connection_string` | * | — | MySQL-compatible DSN (e.g., `mysql://user:pass@host:4000/db`) |
| `api_url` | * | — | TiDB REST API or HTTP SQL proxy URL |
| `query` | Yes | — | SQL SELECT query |
| `limit` | No | `10000` | Maximum rows to import |

*Provide either `connection_string` or `api_url`.

## How It Works

1. Prometheus sends your query to Aegis-DB's federated query engine (or directly to your REST endpoint)
2. Aegis-DB connects to TiDB using the MySQL wire protocol
3. Query results are returned as JSON rows
4. Rows are normalized to CSV and stored as a Prometheus dataset with computed statistics

## Example: Real-Time Analytics

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "tidb",
    "connection_string": "mysql://analyst:readonly@tidb-cloud.example.com:4000/game_analytics",
    "query": "SELECT player_id, level, score, play_time_minutes, purchases, churn_label FROM player_metrics WHERE play_time_minutes > 10",
    "limit": 200000
  }'
```

## Example: Supply Chain Data

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "tidb",
    "connection_string": "mysql://scm_reader:readonly@tidb.logistics.com:4000/supply_chain",
    "query": "SELECT order_date, product_sku, warehouse, quantity, lead_time_days, delay_flag FROM orders WHERE order_date >= '\''2024-01-01'\''",
    "limit": 150000
  }'
```

## Security

- Connection strings are validated by the Shield security engine
- SQL queries pass through a SQL firewall (only SELECT allowed)
- SSRF protection on hostnames
- Credentials used only for the import request, not stored persistently

## TiDB Cloud

For TiDB Cloud Serverless, use the public endpoint from your cluster's connection settings:

```bash
"connection_string": "mysql://user:pass@gateway01.us-east-1.prod.aws.tidbcloud.com:4000/mydb?tls=true"
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Connection refused | Check TiDB is running on the specified port (default 4000) |
| Access denied | Verify credentials and that the user has SELECT privileges |
| SQL blocked by firewall | Only SELECT queries are allowed |
| Timeout | Add WHERE clauses to reduce the result set |
