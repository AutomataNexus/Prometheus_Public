# prometheus-reports

PDF report generation library for the Prometheus platform. Renders branded, multi-section documents via headless Chrome, producing two report types: training summaries and deployment certificates.

## Report Types

### Training Report

Generated after a model training run completes. Sections:

- **Overview** -- model name, architecture, dataset, equipment type, epochs, training time
- **Hyperparameters** -- sorted key-value table of all training parameters
- **Final Metrics** -- accuracy, precision, recall, F1 as visual progress bars; final loss as a callout badge
- **Training Loss Curve** -- inline SVG polyline chart with axis labels and grid lines, rendered directly in the HTML (no external charting library)
- **Recommendations** -- numbered list of actionable suggestions

### Deployment Certificate

Single-page attestation document for model deployments. Includes:

- **Model artefact hash** -- SHA-256 displayed in a monospace callout box
- **Deployment details** -- model name, target device, IP/hostname, architecture, binary size, quantization type, deployer, timestamp
- **Attestation statement** -- formal text confirming integrity verification
- **Signature lines** -- deploying engineer and platform verification
- **Certificate ID** -- UUID for audit tracking

## Rendering Pipeline

```
ReportData ──> build_html() ──> render_html_to_pdf() ──> PDF on disk
                    │                    │
                    │                    ├── Launch headless Chrome
                    │                    ├── Navigate to data:text/html;base64,... URI
                    │                    ├── print_to_pdf()
                    │                    └── Write bytes to output_dir
                    │
                    ├── HTML/CSS template with inline styles
                    ├── Inline SVG charts (no JS dependencies)
                    └── Base64-encoded logo embedding
```

HTML is encoded as a base64 data URI, eliminating the need for a temporary file server. Logo images (PNG, SVG, JPEG) are read from disk and inlined as data URIs.

## Usage

```rust
use prometheus_reports::{
    generate_training_report, generate_deployment_cert,
    TrainingReportData, DeploymentCertData, ReportConfig,
};

let config = ReportConfig::default();

// Training report
let path = generate_training_report(&training_data, &config)?;

// Deployment certificate
let path = generate_deployment_cert(&cert_data, &config)?;
```

## Configuration

`ReportConfig` controls output directory, Chrome binary path, and branding:

| Field | Default | Description |
|-------|---------|-------------|
| `output_dir` | `/tmp/prometheus-reports` | Where PDFs are written |
| `chrome_path` | None (auto-detect) | Path to Chrome/Chromium binary |
| `branding.company_name` | `AutomataNexus` | Header/footer company name |
| `branding.primary_color` | `#14b8a6` | Theme primary (teal) |
| `branding.accent_color` | `#C4A484` | Theme accent (tan) |
| `branding.logo_path` | None | Path to logo image for embedding |

## Dependencies

- `headless_chrome` -- browser automation for PDF rendering
- `base64` -- image and HTML encoding
- `uuid` -- unique report/certificate filenames
- `chrono` -- timestamp formatting
- `thiserror` -- error type definitions
