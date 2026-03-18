# Prometheus + SpaceTimeDB Integration

Connect your SpaceTimeDB modules to Prometheus for ML model training.

## Overview

SpaceTimeDB is a serverless database designed for real-time applications (multiplayer games, simulations, collaborative tools). Prometheus connects via SpaceTimeDB's HTTP SQL endpoint to query table data, normalize it to CSV, and store it as a training dataset.

## Prerequisites

- SpaceTimeDB instance (Cloud or self-hosted)
- Database name / module address
- Network connectivity from Prometheus server to your SpaceTimeDB endpoint

## Quick Start

### Web UI

1. Go to **Datasets** page
2. Click **Connect Source**
3. Select **SpaceTimeDB** from the dropdown
4. Fill in:
   - **Host**: SpaceTimeDB endpoint URL (e.g., `https://mainnet.spacetimedb.com`)
   - **Database**: Module name or address
   - **Query**: SQL query to pull data
5. Click **Connect**

### API

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "spacetimedb",
    "host": "https://mainnet.spacetimedb.com",
    "database": "my-game-module",
    "query": "SELECT * FROM player_stats",
    "limit": 50000
  }'
```

## Configuration Reference

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `source_type` | Yes | — | Must be `"spacetimedb"` |
| `host` | Yes | — | SpaceTimeDB endpoint URL |
| `database` | Yes | — | Module name or database address |
| `query` | Yes | — | SQL query to execute |
| `limit` | No | `10000` | Maximum rows to import |

## How It Works

1. Prometheus sends a SQL query to SpaceTimeDB's HTTP SQL endpoint:
   ```
   POST {host}/database/{database}/sql
   Body: "{query} LIMIT {limit}"
   ```
2. SpaceTimeDB returns JSON rows
3. Prometheus normalizes the JSON to CSV with computed statistics
4. The CSV is stored as a Prometheus dataset

## Example: Game Analytics

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "spacetimedb",
    "host": "https://mainnet.spacetimedb.com",
    "database": "battle-royale-stats",
    "query": "SELECT player_id, matches_played, kills, deaths, win_rate, avg_placement, playtime_hours FROM player_stats WHERE matches_played > 10",
    "limit": 100000
  }'
```

## Example: Robotics Simulation

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "spacetimedb",
    "host": "https://testnet.spacetimedb.com",
    "database": "robot-sim-v2",
    "query": "SELECT timestamp, joint_1, joint_2, joint_3, joint_4, end_effector_x, end_effector_y, end_effector_z, torque, success FROM trajectories",
    "limit": 200000
  }'
```

## Example: Real-Time Collaboration Metrics

```bash
curl -X POST https://prometheus.automatanexus.com/api/v1/datasets/connect \
  -H "Authorization: Bearer $PROMETHEUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "spacetimedb",
    "host": "https://mainnet.spacetimedb.com",
    "database": "collab-editor",
    "query": "SELECT session_id, user_count, edits_per_minute, latency_ms, conflict_rate, session_duration_min FROM session_metrics",
    "limit": 50000
  }'
```

## Use Cases

SpaceTimeDB is particularly useful for:
- **Game telemetry** — Player behavior, match outcomes, churn prediction
- **Simulation data** — Physics, robotics, autonomous systems
- **Real-time app metrics** — Collaboration tools, live dashboards
- **Multiplayer game balancing** — Weapon stats, economy tuning

## Security

- SpaceTimeDB endpoint URL is validated against SSRF
- SQL queries pass through the Shield SQL firewall (only SELECT allowed)
- All data transfer is server-side

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Connection refused | Verify the SpaceTimeDB host URL is correct |
| Module not found | Check the database/module name or address |
| Empty results | Verify the table exists and has data |
| SQL syntax error | SpaceTimeDB uses standard SQL — check column names |
| Timeout | Add WHERE clauses to reduce the result set |
