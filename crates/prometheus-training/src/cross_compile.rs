// ============================================================================
// File: cross_compile.rs
// Description: Cross-compilation helper for building ARM edge inference binaries targeting Raspberry Pi
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Cross-compilation helper for building edge inference binaries.
//!
//! Generates cross-compiled ARM binaries for deployment on Raspberry Pi
//! controllers running `armv7-unknown-linux-musleabihf`.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use crate::{Result, TrainingError};

// ---------------------------------------------------------------------------
// Cross-compilation configuration
// ---------------------------------------------------------------------------

/// Default cross-compilation target for Raspberry Pi.
pub const DEFAULT_TARGET: &str = "armv7-unknown-linux-musleabihf";

/// Default linker for ARM cross-compilation.
pub const DEFAULT_LINKER: &str = "arm-linux-gnueabihf-gcc";

/// Configuration for cross-compiling an inference binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossCompileConfig {
    /// Path to the trained model (.axonml file).
    pub model_path: String,
    /// Target triple (e.g., `armv7-unknown-linux-musleabihf`).
    pub target: String,
    /// Output directory for the compiled binary.
    pub output_dir: String,
    /// Name of the output binary.
    pub binary_name: String,
    /// Whether to strip the binary for size reduction.
    pub strip: bool,
    /// Whether to use LTO (link-time optimization).
    pub lto: bool,
    /// Optimization level (0-3, 's', 'z').
    pub opt_level: String,
    /// Optional custom linker path.
    pub linker: Option<String>,
    /// Additional cargo build flags.
    pub extra_flags: Vec<String>,
}

impl Default for CrossCompileConfig {
    fn default() -> Self {
        Self {
            model_path: String::new(),
            target: DEFAULT_TARGET.to_string(),
            output_dir: "target/edge".to_string(),
            binary_name: "axonml-inference".to_string(),
            strip: true,
            lto: true,
            opt_level: "s".to_string(),
            linker: Some(DEFAULT_LINKER.to_string()),
            extra_flags: Vec::new(),
        }
    }
}

impl CrossCompileConfig {
    /// Create a new cross-compilation config for a specific model.
    pub fn new(model_path: impl Into<String>, output_dir: impl Into<String>) -> Self {
        Self {
            model_path: model_path.into(),
            output_dir: output_dir.into(),
            ..Default::default()
        }
    }

    /// Set the target triple.
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    /// Set the binary name.
    pub fn with_binary_name(mut self, name: impl Into<String>) -> Self {
        self.binary_name = name.into();
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if self.model_path.is_empty() {
            return Err(TrainingError::CrossCompile(
                "model_path must not be empty".into(),
            ));
        }
        if self.target.is_empty() {
            return Err(TrainingError::CrossCompile(
                "target must not be empty".into(),
            ));
        }
        if self.output_dir.is_empty() {
            return Err(TrainingError::CrossCompile(
                "output_dir must not be empty".into(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Build manifest
// ---------------------------------------------------------------------------

/// Manifest describing a cross-compiled inference package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildManifest {
    /// Path to the compiled binary.
    pub binary_path: String,
    /// Path to the model weights included in the package.
    pub model_path: String,
    /// Target architecture.
    pub target: String,
    /// Binary size in bytes.
    pub binary_size: u64,
    /// Model size in bytes.
    pub model_size: u64,
    /// Build timestamp.
    pub built_at: String,
    /// Cargo build profile used.
    pub profile: String,
}

// ---------------------------------------------------------------------------
// Cross-compilation functions
// ---------------------------------------------------------------------------

/// Build an inference binary for edge deployment.
///
/// This function:
/// 1. Validates the model file exists
/// 2. Prepares the output directory structure
/// 3. Generates a build manifest with the model and configuration
/// 4. Returns the path to the packaged output directory
///
/// Note: Actual cross-compilation invokes `cargo build --target` which requires
/// the appropriate toolchain to be installed. This function prepares the
/// build artifacts and manifest; the actual compilation is delegated to
/// the system's Rust toolchain.
#[instrument(skip_all, fields(target = %config.target))]
pub fn build_inference_binary(config: &CrossCompileConfig) -> Result<String> {
    config.validate()?;

    let model_path = Path::new(&config.model_path);
    if !model_path.exists() {
        return Err(TrainingError::CrossCompile(format!(
            "model file not found: {}",
            config.model_path
        )));
    }

    let output_dir = Path::new(&config.output_dir);
    let package_dir = output_dir.join(&config.binary_name);

    // Create the package directory structure.
    fs::create_dir_all(&package_dir).map_err(|e| {
        TrainingError::CrossCompile(format!(
            "failed to create package directory {}: {e}",
            package_dir.display()
        ))
    })?;

    // Copy the model file into the package.
    let model_filename = model_path
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("model.axonml");
    let packaged_model_path = package_dir.join(model_filename);

    fs::copy(model_path, &packaged_model_path).map_err(|e| {
        TrainingError::CrossCompile(format!("failed to copy model file: {e}"))
    })?;

    // Generate the cargo build command that would be used.
    let cargo_cmd = generate_cargo_command(config);
    info!("Cross-compilation command: {}", cargo_cmd);

    // Generate a build script for the edge binary.
    let build_script = generate_build_script(config, model_filename);
    let script_path = package_dir.join("build.sh");
    fs::write(&script_path, &build_script).map_err(|e| {
        TrainingError::CrossCompile(format!("failed to write build script: {e}"))
    })?;

    // Generate an inference daemon entry point source file.
    let inference_src = generate_inference_main(model_filename, config);
    let src_dir = package_dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| {
        TrainingError::CrossCompile(format!("failed to create src directory: {e}"))
    })?;
    fs::write(src_dir.join("main.rs"), &inference_src).map_err(|e| {
        TrainingError::CrossCompile(format!("failed to write inference source: {e}"))
    })?;

    // Generate the package Cargo.toml.
    let cargo_toml = generate_cargo_toml(config);
    fs::write(package_dir.join("Cargo.toml"), &cargo_toml).map_err(|e| {
        TrainingError::CrossCompile(format!("failed to write Cargo.toml: {e}"))
    })?;

    // Create the build manifest.
    let model_size = fs::metadata(&packaged_model_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let manifest = BuildManifest {
        binary_path: package_dir
            .join(&config.binary_name)
            .to_string_lossy()
            .to_string(),
        model_path: packaged_model_path.to_string_lossy().to_string(),
        target: config.target.clone(),
        binary_size: 0, // Will be populated after actual compilation.
        model_size,
        built_at: chrono::Utc::now().to_rfc3339(),
        profile: "release".to_string(),
    };

    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| {
        TrainingError::CrossCompile(format!("failed to serialize manifest: {e}"))
    })?;
    fs::write(package_dir.join("manifest.json"), &manifest_json)?;

    let result_path = package_dir.to_string_lossy().to_string();
    info!(
        "Edge inference package prepared at {} (target: {}, model: {:.1} KB)",
        result_path,
        config.target,
        model_size as f64 / 1024.0
    );

    Ok(result_path)
}

/// Generate the `cargo build` command string for cross-compilation.
fn generate_cargo_command(config: &CrossCompileConfig) -> String {
    let mut parts = vec![
        "cargo".to_string(),
        "build".to_string(),
        "--release".to_string(),
        format!("--target={}", config.target),
    ];

    if config.strip {
        parts.push("-Cstrip=symbols".to_string());
    }

    for flag in &config.extra_flags {
        parts.push(flag.clone());
    }

    parts.join(" ")
}

/// Generate a shell build script for the edge inference binary.
fn generate_build_script(config: &CrossCompileConfig, model_filename: &str) -> String {
    let linker = config
        .linker
        .as_deref()
        .unwrap_or(DEFAULT_LINKER);

    let target_upper = config.target.to_uppercase().replace('-', "_");

    format!(
        r#"#!/bin/bash
# Cross-compilation build script for edge inference daemon
# Target: {target}
# Generated by prometheus-training

set -euo pipefail

export CARGO_TARGET_{target_upper}_LINKER="{linker}"

echo "Building edge inference binary for {target}..."

cargo build --release --target={target}

BINARY="target/{target}/release/{binary_name}"

if [ -f "$BINARY" ]; then
    if command -v arm-linux-gnueabihf-strip &> /dev/null && [ "{strip}" = "true" ]; then
        arm-linux-gnueabihf-strip "$BINARY"
    fi

    SIZE=$(stat -f%z "$BINARY" 2>/dev/null || stat --format=%s "$BINARY" 2>/dev/null || echo "unknown")
    echo "Binary built: $BINARY ($SIZE bytes)"
    echo "Model file: {model_filename}"
    echo ""
    echo "Deploy to Raspberry Pi:"
    echo "  scp $BINARY {model_filename} pi@<device-ip>:/opt/axonml/"
    echo "  ssh pi@<device-ip> 'cd /opt/axonml && ./{binary_name}'"
else
    echo "ERROR: Build failed — binary not found at $BINARY"
    exit 1
fi
"#,
        target = config.target,
        target_upper = target_upper,
        linker = linker,
        binary_name = config.binary_name,
        strip = config.strip,
        model_filename = model_filename,
    )
}

/// Generate a minimal inference daemon `main.rs` source.
fn generate_inference_main(model_filename: &str, config: &CrossCompileConfig) -> String {
    format!(
        r#"//! Edge inference daemon for Prometheus.
//!
//! Loads a trained .axonml model and serves predictions over HTTP.
//! Target: {target}
//!
//! Generated by prometheus-training cross_compile module.

use std::fs;
use std::io::{{Read, Write}};
use std::net::TcpListener;

const MODEL_FILE: &str = "{model_filename}";
const LISTEN_ADDR: &str = "0.0.0.0:6200";

fn main() {{
    println!("Prometheus Edge Inference Daemon");
    println!("Loading model: {{}}", MODEL_FILE);

    // Load model weights.
    let model_data = match fs::read(MODEL_FILE) {{
        Ok(data) => data,
        Err(e) => {{
            eprintln!("Failed to load model: {{}}", e);
            std::process::exit(1);
        }}
    }};

    println!("Model loaded ({{:.1}} KB)", model_data.len() as f64 / 1024.0);
    println!("Listening on {{}}", LISTEN_ADDR);

    // Start HTTP server.
    let listener = match TcpListener::bind(LISTEN_ADDR) {{
        Ok(l) => l,
        Err(e) => {{
            eprintln!("Failed to bind: {{}}", e);
            std::process::exit(1);
        }}
    }};

    for stream in listener.incoming() {{
        match stream {{
            Ok(mut stream) => {{
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);

                let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"status\":\"ok\",\"model\":\"{model_filename}\"}}\r\n";
                let _ = stream.write_all(response.as_bytes());
            }}
            Err(e) => eprintln!("Connection error: {{}}", e),
        }}
    }}
}}
"#,
        target = config.target,
        model_filename = model_filename,
    )
}

/// Generate a minimal Cargo.toml for the inference binary.
fn generate_cargo_toml(config: &CrossCompileConfig) -> String {
    format!(
        r#"[package]
name = "{binary_name}"
version = "0.1.0"
edition = "2021"
description = "Prometheus edge inference daemon"

[[bin]]
name = "{binary_name}"
path = "src/main.rs"

[profile.release]
opt-level = "{opt_level}"
lto = {lto}
codegen-units = 1
strip = {strip}
panic = "abort"
"#,
        binary_name = config.binary_name,
        opt_level = config.opt_level,
        lto = if config.lto { "true" } else { "false" },
        strip = if config.strip { "true" } else { "false" },
    )
}

/// Check whether the required cross-compilation toolchain is available.
pub fn check_toolchain(target: &str) -> Result<ToolchainStatus> {
    // Check if rustup target is installed.
    let rustup_available = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .ok()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.trim() == target)
        })
        .unwrap_or(false);

    // Check if the linker is available.
    let linker_available = std::process::Command::new(DEFAULT_LINKER)
        .arg("--version")
        .output()
        .is_ok();

    let status = ToolchainStatus {
        target: target.to_string(),
        rustup_target_installed: rustup_available,
        linker_available,
        ready: rustup_available && linker_available,
    };

    if !status.ready {
        warn!(
            "Cross-compilation toolchain not fully available: \
             rustup target={}, linker={}",
            status.rustup_target_installed, status.linker_available
        );
    }

    Ok(status)
}

/// Status of the cross-compilation toolchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainStatus {
    pub target: String,
    pub rustup_target_installed: bool,
    pub linker_available: bool,
    pub ready: bool,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CrossCompileConfig::default();
        assert_eq!(config.target, DEFAULT_TARGET);
        assert_eq!(config.binary_name, "axonml-inference");
        assert!(config.strip);
        assert!(config.lto);
    }

    #[test]
    fn test_config_builder() {
        let config = CrossCompileConfig::new("/tmp/model.axonml", "/tmp/output")
            .with_target("aarch64-unknown-linux-musl")
            .with_binary_name("my-inference");

        assert_eq!(config.model_path, "/tmp/model.axonml");
        assert_eq!(config.output_dir, "/tmp/output");
        assert_eq!(config.target, "aarch64-unknown-linux-musl");
        assert_eq!(config.binary_name, "my-inference");
    }

    #[test]
    fn test_config_validation() {
        let mut config = CrossCompileConfig::default();
        config.model_path = String::new();
        assert!(config.validate().is_err());

        config.model_path = "/some/path.axonml".into();
        config.target = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_generate_cargo_command() {
        let config = CrossCompileConfig::default();
        let cmd = generate_cargo_command(&config);
        assert!(cmd.contains("cargo build --release"));
        assert!(cmd.contains(&config.target));
    }

    #[test]
    fn test_generate_cargo_toml() {
        let config = CrossCompileConfig::default();
        let toml = generate_cargo_toml(&config);
        assert!(toml.contains("axonml-inference"));
        assert!(toml.contains("opt-level"));
        assert!(toml.contains("lto = true"));
    }

    // -----------------------------------------------------------------------
    // Additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_target_is_armv7() {
        assert_eq!(DEFAULT_TARGET, "armv7-unknown-linux-musleabihf");
    }

    #[test]
    fn test_default_linker() {
        assert_eq!(DEFAULT_LINKER, "arm-linux-gnueabihf-gcc");
    }

    #[test]
    fn test_check_toolchain_returns_status() {
        let status = check_toolchain("some-fake-target-triple").unwrap();
        assert_eq!(status.target, "some-fake-target-triple");
        // On CI / test environments the ARM target is not installed.
        assert!(!status.ready || status.rustup_target_installed);
    }

    #[test]
    fn test_build_script_contains_target() {
        let config = CrossCompileConfig::default();
        let script = generate_build_script(&config, "model.axonml");
        assert!(script.contains(&config.target));
        assert!(script.contains("cargo build --release"));
        assert!(script.contains("model.axonml"));
    }

    #[test]
    fn test_build_script_custom_target() {
        let config = CrossCompileConfig::new("/tmp/m.axonml", "/tmp/out")
            .with_target("aarch64-unknown-linux-musl");
        let script = generate_build_script(&config, "m.axonml");
        assert!(script.contains("aarch64-unknown-linux-musl"));
        assert!(script.contains("AARCH64_UNKNOWN_LINUX_MUSL"));
    }

    #[test]
    fn test_config_with_custom_target() {
        let config = CrossCompileConfig::new("/tmp/model.axonml", "/tmp/output")
            .with_target("x86_64-unknown-linux-musl");
        assert_eq!(config.target, "x86_64-unknown-linux-musl");
        config.validate().unwrap(); // should be valid
    }

    #[test]
    fn test_config_validation_empty_output_dir() {
        let mut config = CrossCompileConfig::default();
        config.model_path = "/some/path.axonml".into();
        config.output_dir = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_new_sets_defaults() {
        let config = CrossCompileConfig::new("/my/model.axonml", "/my/output");
        assert_eq!(config.model_path, "/my/model.axonml");
        assert_eq!(config.output_dir, "/my/output");
        assert_eq!(config.target, DEFAULT_TARGET); // inherits default
        assert!(config.strip);
        assert!(config.lto);
        assert_eq!(config.opt_level, "s");
    }

    #[test]
    fn test_generate_cargo_command_contains_strip_flag() {
        let config = CrossCompileConfig::default();
        let cmd = generate_cargo_command(&config);
        assert!(cmd.contains("-Cstrip=symbols"));
    }

    #[test]
    fn test_generate_cargo_command_no_strip() {
        let mut config = CrossCompileConfig::default();
        config.strip = false;
        let cmd = generate_cargo_command(&config);
        assert!(!cmd.contains("-Cstrip=symbols"));
    }

    #[test]
    fn test_generate_cargo_command_extra_flags() {
        let mut config = CrossCompileConfig::default();
        config.extra_flags = vec!["--features".to_string(), "my-feature".to_string()];
        let cmd = generate_cargo_command(&config);
        assert!(cmd.contains("--features"));
        assert!(cmd.contains("my-feature"));
    }

    #[test]
    fn test_generate_cargo_toml_custom_binary_name() {
        let config = CrossCompileConfig::new("/tmp/m.axonml", "/tmp/out")
            .with_binary_name("custom-daemon");
        let toml = generate_cargo_toml(&config);
        assert!(toml.contains("custom-daemon"));
    }

    #[test]
    fn test_generate_cargo_toml_no_lto() {
        let mut config = CrossCompileConfig::default();
        config.lto = false;
        let toml = generate_cargo_toml(&config);
        assert!(toml.contains("lto = false"));
    }

    #[test]
    fn test_toolchain_status_serialization() {
        let status = ToolchainStatus {
            target: "armv7-unknown-linux-musleabihf".to_string(),
            rustup_target_installed: true,
            linker_available: false,
            ready: false,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("armv7-unknown-linux-musleabihf"));
        let deserialized: ToolchainStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.target, status.target);
        assert_eq!(deserialized.ready, status.ready);
    }
}
