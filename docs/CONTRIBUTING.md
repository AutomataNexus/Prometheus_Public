# Contributing to Prometheus

Thank you for your interest in contributing to Prometheus. This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

Be respectful. Treat all contributors with courtesy and professionalism. We are building tools that protect the buildings where people live and work -- bring that same care to how you interact with others.

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Node.js 20+ (for Tailwind CSS and E2E tests)
- Python 3.10+ (for the Gradient agent)
- Chromium (for PDF generation and Puppeteer tests)
- Aegis-DB running on port 9091

### Clone and Build

```bash
git clone https://github.com/AutomataNexus/Prometheus.git
cd Prometheus

# Install WASM target
rustup target add wasm32-unknown-unknown

# Build all crates
cargo build
```

### Run Locally

```bash
# Terminal 1: Start Aegis-DB
cd /opt/Aegis-DB && cargo run --release

# Terminal 2: Start Prometheus server
cd /opt/Prometheus && cargo run --bin prometheus-server
```

The server will be available at `http://localhost:3030`.

### Run Tests

```bash
# Rust unit tests
cargo test

# Rust integration tests (requires running server)
cargo test --test api_tests
cargo test --test auth_tests
cargo test --test training_tests -- --test-threads=1

# Playwright E2E tests
cd tests/e2e
npm install
npx playwright install --with-deps
npx playwright test

# Puppeteer PDF tests
cd tests/puppeteer
npm install
npm test
```

## Project Structure

```
/opt/Prometheus/
  Cargo.toml                # Workspace root
  crates/
    prometheus-server/      # Axum HTTP server, REST API, WebSocket
    prometheus-ui/          # Leptos WASM reactive UI
    prometheus-training/    # AxonML training pipeline orchestrator
    prometheus-edge/        # Edge inference daemon for Raspberry Pi
    prometheus-reports/     # PDF report generation
    prometheus-agent/       # Gradient AI ADK agent (Python)
  tests/
    e2e/                    # Playwright E2E browser tests
    puppeteer/              # Puppeteer PDF generation tests
    integration/            # Rust integration tests
  docs/                     # Documentation
  tailwind.config.js        # Tailwind CSS configuration
  input.css                 # Tailwind input with NexusEdge components
  Dockerfile                # Multi-stage production build
  docker-compose.yml        # Development and production orchestration
```

## Coding Standards

### Rust

- Follow standard Rust conventions and `rustfmt` formatting
- Run `cargo fmt` before every commit
- Run `cargo clippy` and fix all warnings
- All public functions and types must have doc comments (`///`)
- Use `thiserror` for error types in library crates
- Use `anyhow` for error handling in binary crates and tests
- Prefer `async/await` for I/O-bound operations
- Use `tracing` for structured logging (not `println!` or `log`)
- Keep functions under 50 lines where practical

### TypeScript (E2E Tests)

- Use TypeScript for all test files
- Follow Playwright best practices:
  - Prefer `data-testid` selectors over CSS/XPath
  - Use `getByRole`, `getByLabel`, `getByText` for accessible selectors
  - Keep tests independent -- each test should set up its own state
- Use page object patterns for complex test flows
- Add meaningful test descriptions

### Python (Gradient Agent)

- Follow PEP 8 style guidelines
- Use type hints for all function signatures
- Use `async/await` for I/O operations
- Document all agent tools and capabilities

### CSS / Tailwind

- Use Tailwind utility classes when possible
- Define reusable component classes in `input.css` using `@layer components`
- Follow the NexusEdge design system color palette (see `tailwind.config.js`)
- Use the predefined component classes (`btn-primary`, `prometheus-card`, etc.)

## Pull Request Process

1. **Fork** the repository
2. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/my-feature
   ```
3. **Make your changes** with clear, atomic commits
4. **Add or update tests** for any new functionality
5. **Run the full test suite** and ensure it passes:
   ```bash
   cargo fmt --check
   cargo clippy
   cargo test
   ```
6. **Submit a pull request** with a clear description

### Commit Message Format

Use conventional commit style:

```
feat: add boiler anomaly detection architecture
fix: resolve WebSocket disconnect during long training runs
docs: update API reference for deployment endpoints
test: add E2E tests for model comparison feature
refactor: extract training pipeline into separate module
chore: update Tailwind CSS to v4.1
```

### PR Description Template

```markdown
## Summary
Brief description of what changed and why.

## Changes
- Changed X to do Y
- Added Z for W

## Test plan
- [ ] Unit tests pass
- [ ] E2E tests pass
- [ ] Manual testing steps (describe)

## Screenshots
(For UI changes)
```

## Architecture Decisions

Major architectural changes should be discussed in a GitHub issue before implementation. This includes:

- New model architectures for `prometheus-training`
- Aegis-DB schema changes
- API endpoint additions or breaking modifications
- New external service integrations
- Changes to the edge deployment pipeline
- Changes to the Gradient agent tools or capabilities

## Adding a New Model Architecture

1. Create a new file in `crates/prometheus-training/src/architectures/`
2. Implement the `TrainableModel` trait using AxonML nn layers:
   - `forward(&self, input: &Variable) -> Variable` -- define the forward pass using AxonML layers (Linear, LSTM, GRU, RNN, Conv1d, Conv2d, BatchNorm2d, TransformerEncoder, TransformerDecoder, ResidualBlock, MultiHeadAttention, CrossAttention, Sequential, etc.)
   - `parameters(&self) -> Vec<Parameter>` -- return all trainable parameters for the autograd optimizer
   - Training uses AxonML's autograd engine for real backpropagation (single forward + backward pass), not numerical gradients
   - Use AxonML's built-in loss functions (MSELoss, BCELoss, CrossEntropyLoss) and optimizers (Adam, AdamW with weight_decay)
3. Register the architecture in `crates/prometheus-training/src/architectures/mod.rs`
4. Add the architecture option to the API validation in `prometheus-server`
5. Update the UI to display the new option
6. Add integration tests for the new architecture
7. Update documentation

## Reporting Issues

When reporting bugs, include:

- **Steps to reproduce** the issue
- **Expected behavior** vs. **actual behavior**
- **Environment details** (OS, Rust version, browser, etc.)
- **Relevant log output** (use `RUST_LOG=debug` for detailed logs)
- **Screenshots** for UI issues

## Security

If you discover a security vulnerability, please report it privately. Do NOT open a public GitHub issue. Contact the maintainers directly at security@automatanexus.com.

## License

Contributions are dual-licensed under MIT and Apache 2.0, consistent with the project license. By submitting a pull request, you agree to license your contributions under these terms.
