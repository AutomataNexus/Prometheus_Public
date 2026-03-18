# prometheus-cli

Terminal interface for the Prometheus ML platform. Provides both a direct subcommand CLI and an interactive REPL with slash commands, a ratatui-based TUI training monitor, QR-code authentication, and OpenZL dataset compression.

## Modes of Operation

### Subcommand Mode

```bash
prometheus login
prometheus datasets
prometheus train <dataset_id> --arch lstm_autoencoder --epochs 100
prometheus monitor <run_id>
prometheus compress data.csv --dict prometheus.ozl-dict
```

### Interactive REPL

Run `prometheus` with no arguments to enter the REPL. Slash commands mirror the subcommands. Unslashed text is forwarded to the PrometheusForge AI agent.

```
prometheus> /datasets
prometheus> /train abc123 resnet
prometheus> /monitor
prometheus> What architecture works best for vibration data?
```

## Commands

| Command | Description |
|---------|-------------|
| `login` | QR-code + browser-verified authentication flow |
| `logout` | Clear stored credentials |
| `whoami` | Show authenticated user info |
| `datasets` / `dataset <id>` | List or inspect datasets |
| `upload <file>` | Upload a CSV dataset |
| `validate <id>` / `unlock <id>` | Validate dataset for training / unlock for editing |
| `train <dataset_id>` | Start training with architecture and hyperparameter flags |
| `training` / `status <id>` | List training runs / show run progress with bar |
| `queue` | Training queue capacity status |
| `stop <id>` | Send stop signal to a training run |
| `models` / `model <id>` | List or inspect trained models |
| `deploy <model_id>` | Deploy model to edge device (default target: armv7 musl) |
| `agent [message]` | Chat with PrometheusForge AI agent |
| `monitor [id]` | Launch TUI training dashboard |
| `compress <file>` | Compress dataset to .ozl format |
| `decompress <file>` | Decompress an .ozl file |
| `train-compressor <files...>` | Train an OpenZL dictionary on dataset files |
| `health` | Server health check |
| `config [key] [value]` | View or update CLI configuration |

### Supported Architectures

lstm_autoencoder, gru_predictor, rnn, sentinel, resnet, vgg, vit, bert, gpt2, conv1d, conv2d, nexus, phantom

## Authentication

Two-step flow:

1. User enters credentials in the terminal
2. CLI requests a verification session, displays a QR code and URL
3. User scans QR / opens URL in browser to confirm
4. CLI polls for browser verification (120s timeout), then saves the token

Credentials are stored at `~/.prometheus/credentials` with `0600` permissions.

## TUI Dashboard

Launched via `prometheus monitor`. Two tabs:

- **Dashboard** -- overview of all training runs (table with epoch progress, val loss, status) and models (list with F1 scores)
- **Monitor** -- focused view of a single training run with a live Braille-rendered loss curve chart (train + val loss), run metadata, and recent epoch metrics

Controls: `Tab` switch views, `j/k` or arrows to navigate, `Enter` to select, `r` to refresh, `q` to quit. Auto-refreshes every 3 seconds.

## OpenZL Compression

Custom dataset compression format wrapping zstd with trainable dictionaries.

### File Format (.ozl)

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | Magic `OZL\x01` |
| 4 | 1 | Version `0x01` |
| 5 | 1 | Flags (bit 0 = has dictionary) |
| 6 | 32 | SHA-256 of original data |
| 38 | 8 | Original size (u64 LE) |
| 46 | 8 | Dictionary ID (u64 LE) |
| 54 | ... | zstd-compressed payload |

### Dictionary Training

Train a zstd dictionary on representative CSV files, then use it for improved compression of similar datasets:

```bash
prometheus train-compressor train_*.csv --output prometheus.ozl-dict
prometheus compress new_data.csv --dict prometheus.ozl-dict
```

Compression level: zstd level 15. Dictionary max size: 112 KB. SHA-256 integrity verification on decompression.

## Configuration

Stored at `~/.prometheus/config.toml`:

| Key | Default | Description |
|-----|---------|-------------|
| `server_url` | `http://localhost:3030` | Prometheus server URL |
| `data_dir` | `~/.prometheus` | Local data directory |

Override server URL via `--server` flag or `PROMETHEUS_URL` env var.

## Dependencies

- `clap` -- argument parsing with derive macros
- `ratatui` / `crossterm` -- TUI rendering
- `reqwest` -- HTTP client
- `qrcode` -- terminal QR code generation
- `zstd` -- compression engine
- `sha2` -- integrity hashing
- `dialoguer` -- password input
- `open` -- browser launch for auth verification
- `toml` -- configuration file parsing
