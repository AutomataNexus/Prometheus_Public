# Prometheus + PostgreSQL Integration

Connect your PostgreSQL database to Prometheus for ML model training.

## Overview

Prometheus connects to PostgreSQL through either:
1. **Aegis-DB Federated Query** — Aegis-DB proxies the SQL query to your PostgreSQL instance
2. **Direct REST API** — If you run PostgREST or a similar HTTP gateway

Data is queried, normalized to CSV, and stored as a Prometheus dataset.

## Prerequisites

- PostgreSQL 12+ instance
- A connection string (DSN) or REST API endpoint
- Read-only user credentials (recommended)
- Network connectivity from Prometheus server to your database

## Quick Start

### Web UI

1. Go to **Datasets** page
2. Click **Connect Source**
3. Select **PostgreSQL** from the dropdown
4. Fill in:
   - **Host**: Your PostgreSQL hostname (e.g., `db.example.com`)
   - **Port**: `5432` (default)
   - **Database**: Database name
   - **Query**: SQL SELECT query to pull training data
5. Click **Connect**

### API (Connection String)

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "postgresql",
    "connection_string": "postgresql://readonly:password@db.example.com:5432/clinical_data",
    "query": "SELECT patient_id, age, biomarker_a, biomarker_b, outcome FROM trial_results",
    "limit": 50000
  }'
```

### API (REST API)

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "postgresql",
    "api_url": "https://postgrest.example.com/rpc/query",
    "query": "SELECT * FROM financial_transactions WHERE amount > 0",
    "limit": 100000
  }'
```

## Configuration Reference

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `source_type` | Yes | — | Must be `"postgresql"` |
| `connection_string` | * | — | PostgreSQL DSN (e.g., `postgresql://user:pass@host:5432/db`) |
| `api_url` | * | — | PostgREST or HTTP proxy URL |
| `query` | Yes | — | SQL SELECT query |
| `limit` | No | `10000` | Maximum rows to import |

*Provide either `connection_string` (Aegis-DB federated) or `api_url` (direct REST).

## How It Works

### Path 1: Aegis-DB Federated Query (preferred)

1. Prometheus sends your query to Aegis-DB's federated query endpoint
2. Aegis-DB connects to PostgreSQL using the provided DSN
3. Executes the SQL query with LIMIT appended
4. Returns JSON rows to Prometheus
5. Rows are normalized to CSV and stored as a dataset

### Path 2: Direct REST API

1. Prometheus sends the query directly to your HTTP endpoint
2. Expects a JSON array of row objects in the response
3. Normalizes to CSV and stores as a dataset

## Example: Medical Records

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "postgresql",
    "connection_string": "postgresql://readonly:s3cure@emr-db.hospital.internal:5432/patient_data",
    "query": "SELECT age, sex, bmi, blood_pressure, cholesterol, glucose, insulin, outcome FROM diabetes_screening WHERE screening_date >= '\''2024-01-01'\''",
    "limit": 25000
  }'
```

## Example: Financial Analytics

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "postgresql",
    "connection_string": "postgresql://analyst:readonly@analytics-db.corp.com:5432/transactions",
    "query": "SELECT transaction_id, amount, merchant_category, hour_of_day, day_of_week, is_fraud FROM credit_card_txns",
    "limit": 200000
  }'
```

## Example: E-commerce Product Data

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "postgresql",
    "connection_string": "postgresql://data_team:readonly@prod-replica.shop.com:5432/catalog",
    "query": "SELECT product_name, category, price, rating, review_count, description FROM products WHERE review_count > 10",
    "limit": 50000
  }'
```

## Security

- Connection strings are validated against injection attacks by the Shield security engine
- SQL queries pass through a SQL firewall that blocks DROP, DELETE, UPDATE, INSERT, ALTER
- Only SELECT queries are allowed
- Connection strings with suspicious patterns are rejected
- SSRF protection validates hostnames before connecting
- Credentials are used only for the import and not stored (only the source_type and query are saved in metadata)

## Best Practices

1. **Use a read-only database user** — Never connect with admin credentials
2. **Filter server-side** — Use WHERE clauses to pull only relevant data instead of dumping entire tables
3. **Add LIMIT** — Prometheus adds a LIMIT, but include one in your query for predictability
4. **Use column aliases** — Give columns meaningful names: `SELECT temp AS temperature, hum AS humidity`
5. **Avoid SELECT *** — List specific columns for cleaner datasets

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Connection refused | Check hostname, port, and firewall rules |
| Authentication failed | Verify username/password in connection string |
| SQL blocked | Shield firewall blocks non-SELECT queries. Use only SELECT |
| "connection_string or api_url required" | Provide one of the two connection methods |
| Timeout | Large queries may timeout at 30s. Add WHERE filters to reduce data |
