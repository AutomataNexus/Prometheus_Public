# Prometheus -- Deployment Guide

## Quick Start (Docker Compose)

The fastest way to deploy Prometheus is with Docker Compose:

```bash
cd /opt/Prometheus

# Set environment variables
export DIGITALOCEAN_API_TOKEN=dop_v1_your_token
export GRADIENT_MODEL_ACCESS_KEY=your_key
export GRADIENT_AGENT_ID=your_agent_id
export AEGIS_DB_PASSWORD=your_secure_password

# Build and start
docker compose up -d --build

# Verify services
docker compose ps
curl http://localhost:3030/health
curl http://localhost:9091/health
```

Prometheus will be available at `http://localhost:3030`.

## Manual Build and Deploy

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Node.js 20+ (for Tailwind CSS)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- trunk: `cargo install trunk`
- Aegis-DB running on port 9091
- Chromium (for PDF report generation)

### Step 1: Build the WASM UI

```bash
cd /opt/Prometheus
rustup target add wasm32-unknown-unknown
cargo install trunk

cd crates/prometheus-ui
trunk build --release
```

### Step 2: Build Tailwind CSS

```bash
cd /opt/Prometheus
npm install -g tailwindcss
npx tailwindcss -i input.css -o static/css/prometheus.css --minify
```

### Step 3: Compile the Server

```bash
cd /opt/Prometheus
cargo build --release --bin prometheus-server
```

### Step 4: Start Aegis-DB

```bash
cd /opt/Aegis-DB
cargo build --release --bin aegis-server
./target/release/aegis-server --port 9091
```

### Step 5: Run the Server

```bash
cd /opt/Prometheus
./target/release/prometheus-server \
    --port 3030 \
    --aegis-url http://localhost:9091 \
    --gradient-api-key $GRADIENT_MODEL_ACCESS_KEY
```

## Production Deployment (DigitalOcean Droplet)

### PM2 Process Management

```bash
# Install PM2 if not already installed
npm install -g pm2

# Start Prometheus server
pm2 start ./target/release/prometheus-server \
    --name prometheus \
    --interpreter none \
    -- --port 3030 --aegis-url http://localhost:9091

# Save process list
pm2 save --force

# Set up startup script
pm2 startup
```

### Nginx Reverse Proxy

Create `/etc/nginx/sites-available/prometheus`:

```nginx
server {
    listen 443 ssl http2;
    server_name prometheus.automatanexus.com;

    ssl_certificate /etc/letsencrypt/live/prometheus.automatanexus.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/prometheus.automatanexus.com/privkey.pem;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Gzip compression
    gzip on;
    gzip_types text/plain text/css application/json application/javascript application/wasm;
    gzip_min_length 1000;

    # HTTP requests
    location / {
        proxy_pass http://127.0.0.1:3030;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket connections
    location /ws/ {
        proxy_pass http://127.0.0.1:3030;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_read_timeout 86400;
    }

    # Static WASM files (long cache)
    location /static/ {
        proxy_pass http://127.0.0.1:3030;
        expires 30d;
        add_header Cache-Control "public, immutable";
    }
}

# HTTP redirect
server {
    listen 80;
    server_name prometheus.automatanexus.com;
    return 301 https://$server_name$request_uri;
}
```

Enable the site:
```bash
sudo ln -s /etc/nginx/sites-available/prometheus /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### TLS Certificate

```bash
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d prometheus.automatanexus.com
```

## Edge Deployment (Raspberry Pi)

### Cross-Compile the Edge Daemon

```bash
# Install cross-compilation tool
cargo install cross

# Add ARM target
rustup target add armv7-unknown-linux-musleabihf

# Cross-compile
cross build --release --target armv7-unknown-linux-musleabihf --bin prometheus-edge
```

### Deploy to Raspberry Pi

```bash
# Copy binary to the Pi
scp target/armv7-unknown-linux-musleabihf/release/prometheus-edge \
    pi@192.168.1.100:/opt/prometheus/

# Copy model file
scp models/warren-ahu1-aether.axonml pi@192.168.1.100:/opt/prometheus/models/

# SSH to the Pi and start
ssh pi@192.168.1.100
cd /opt/prometheus
./prometheus-edge --model models/warren-ahu1-aether.axonml --port 6200
```

### PM2 on Raspberry Pi

```bash
pm2 start ./prometheus-edge \
    --name axonml-inference \
    --interpreter none \
    -- --model models/warren-ahu1-aether.axonml --port 6200
pm2 save --force
pm2 startup
```

### Verify Edge Deployment

```bash
curl http://192.168.1.100:6200/health
# Expected: {"status": "ok", "model": "warren-ahu1-aether", "uptime": 42}
```

## Deploy the Gradient Agent

See [GRADIENT_SETUP.md](./GRADIENT_SETUP.md) for detailed instructions.

```bash
cd /opt/Prometheus/crates/prometheus-agent
pip install -r requirements.txt
cp .env.example .env
# Edit .env with credentials
gradient agent deploy
```

## Stripe Setup

Prometheus uses Stripe for subscription billing. Configure the following:

1. **Create Products in Stripe Dashboard:**
   - `Prometheus Pro` -- $49.00/month recurring
   - `Prometheus Enterprise` -- $199.00/month recurring

2. **Create a Webhook Endpoint:**
   - URL: `https://your-domain/api/v1/billing/webhook`
   - Events: `checkout.session.completed`, `customer.subscription.updated`, `customer.subscription.deleted`

3. **Set Environment Variables:**
   - `STRIPE_SECRET_KEY` -- from Stripe Dashboard > Developers > API Keys
   - `STRIPE_WEBHOOK_SECRET` -- from Stripe Dashboard > Developers > Webhooks (signing secret)
   - `STRIPE_PRO_PRICE_ID` -- Price ID for the Pro product (starts with `price_`)
   - `STRIPE_ENTERPRISE_PRICE_ID` -- Price ID for the Enterprise product

## Environment Variables Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `PROMETHEUS_PORT` | No | `3030` | Server listen port |
| `PROMETHEUS_HOST` | No | `0.0.0.0` | Server bind address |
| `AEGIS_DB_URL` | Yes | `http://localhost:9091` | Aegis-DB connection URL |
| `AEGIS_DB_USERNAME` | No | `admin` | Aegis-DB admin username |
| `AEGIS_DB_PASSWORD` | Yes | -- | Aegis-DB admin password |
| `DIGITALOCEAN_API_TOKEN` | Yes | -- | DigitalOcean API token |
| `GRADIENT_MODEL_ACCESS_KEY` | Yes | -- | Gradient model access key |
| `GRADIENT_AGENT_ID` | Yes | -- | Deployed Gradient agent ID |
| `STRIPE_SECRET_KEY` | No | -- | Stripe API secret key |
| `STRIPE_WEBHOOK_SECRET` | No | -- | Stripe webhook signing secret |
| `STRIPE_PRO_PRICE_ID` | No | -- | Stripe Price ID for Pro tier |
| `STRIPE_ENTERPRISE_PRICE_ID` | No | -- | Stripe Price ID for Enterprise tier |
| `RESEND_API_KEY` | No | -- | Resend transactional email API key |
| `CROSS_COMPILE_TARGET` | No | `armv7-unknown-linux-musleabihf` | ARM cross-compile target |
| `PUPPETEER_EXECUTABLE_PATH` | No | Auto-detected | Path to Chromium binary |
| `PROMETHEUS_MAX_TRAININGS` | No | CPU core count | Max concurrent training runs server-wide |
| `PROMETHEUS_PUBLIC_URL` | No | -- | Public-facing URL for email links |
| `RUST_LOG` | No | `info` | Logging level |

## Health Checks

| Endpoint | Service | Port |
|----------|---------|------|
| `GET /health` | Prometheus server | 3030 |
| `GET /health` | Aegis-DB | 9091 |
| `GET /health` | Edge inference daemon | 6200 |
| `GET /health` | NexusEdge hardware daemon | 6100 |

## Backup and Recovery

### Aegis-DB Data

```bash
# Backup
docker compose exec aegis-db aegis-backup /var/lib/aegis /backup/aegis-$(date +%Y%m%d).tar.gz

# Restore
docker compose exec aegis-db aegis-restore /backup/aegis-20260306.tar.gz /var/lib/aegis
```

### Model Artifacts

```bash
# Backup model files
tar -czf models-backup-$(date +%Y%m%d).tar.gz /app/models/
```
