// ============================================================================
// File: main.rs
// Description: CLI entrypoint with interactive REPL, slash commands, and TUI launcher
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Prometheus CLI — terminal interface for the Prometheus ML platform.
//!
//! Features:
//! - Interactive REPL with slash commands
//! - QR-code and URL-based authentication via Aegis-DB
//! - Real-time TUI training monitor
//! - OpenZL dataset compression
//! - Sync with WASM UI sessions

mod api;
mod auth;
mod commands;
mod compression;
mod config;
mod theme;
mod tui;

use clap::{Parser, Subcommand};
use std::io::{self, Write, BufRead};

#[derive(Parser)]
#[command(
    name = "prometheus",
    about = "Prometheus ML Platform CLI",
    version,
    long_about = "AI-powered edge ML training orchestrator.\nInteractive mode: run without arguments for REPL with /commands."
)]
struct Cli {
    /// Server URL
    #[arg(long, env = "PROMETHEUS_URL", default_value = "http://localhost:3030")]
    server: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate via QR code or browser URL
    Login,
    /// Clear stored credentials
    Logout,
    /// Show current authenticated user
    Whoami,
    /// List datasets
    Datasets,
    /// Show dataset details
    Dataset {
        /// Dataset ID
        id: String,
    },
    /// Upload a CSV dataset
    Upload {
        /// Path to CSV file
        file: String,
        /// Dataset name
        #[arg(long)]
        name: Option<String>,
    },
    /// List trained models
    Models,
    /// Show model details
    Model {
        /// Model ID
        id: String,
    },
    /// Start a training run
    Train {
        /// Dataset ID to train on
        dataset_id: String,
        /// Architecture (lstm_autoencoder, gru_predictor, resnet, vgg, bert, etc.)
        #[arg(long, default_value = "lstm_autoencoder")]
        arch: String,
        /// Learning rate
        #[arg(long)]
        lr: Option<f64>,
        /// Number of epochs
        #[arg(long)]
        epochs: Option<u64>,
        /// Batch size
        #[arg(long)]
        batch_size: Option<u64>,
        /// Hidden dimension
        #[arg(long)]
        hidden_dim: Option<u64>,
    },
    /// List training runs
    Training,
    /// Show training run status
    TrainingStatus {
        /// Training run ID
        id: String,
    },
    /// Stop a training run
    Stop {
        /// Training run ID
        id: String,
    },
    /// Show training queue status
    Queue,
    /// Validate a dataset for training
    Validate {
        /// Dataset ID
        id: String,
    },
    /// Unlock a validated dataset
    Unlock {
        /// Dataset ID
        id: String,
    },
    /// Open TUI training monitor
    Monitor {
        /// Optional training run ID to focus on
        id: Option<String>,
    },
    /// Deploy a model to edge device
    Deploy {
        /// Model ID
        model_id: String,
        /// Target device address
        #[arg(long)]
        target: Option<String>,
    },
    /// Chat with PrometheusForge agent
    Agent {
        /// Message to send (omit for interactive chat)
        message: Option<String>,
    },
    /// Compress a dataset with OpenZL
    Compress {
        /// Input file path
        file: String,
        /// Output file path (default: <file>.ozl)
        #[arg(long)]
        output: Option<String>,
        /// Dictionary file for custom compression
        #[arg(long)]
        dict: Option<String>,
    },
    /// Decompress an OpenZL file
    Decompress {
        /// Input .ozl file
        file: String,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
    /// Train an OpenZL compression dictionary on a dataset
    TrainCompressor {
        /// Input dataset files (CSV)
        files: Vec<String>,
        /// Output dictionary file
        #[arg(long, default_value = "prometheus.ozl-dict")]
        output: String,
    },
    /// Check server health
    Health,
    /// Show or edit configuration
    Config {
        /// Key to get/set
        key: Option<String>,
        /// Value to set
        value: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut cfg = config::Config::load()?;
    if cli.server != "http://localhost:3030" {
        cfg.server_url = cli.server.clone();
    }

    match cli.command {
        Some(cmd) => run_command(cmd, &cfg).await,
        None => run_repl(&cfg).await,
    }
}

async fn run_command(cmd: Commands, cfg: &config::Config) -> anyhow::Result<()> {
    let client = api::ApiClient::new(&cfg.server_url, cfg.load_token());

    match cmd {
        Commands::Login => auth::login_flow(cfg).await,
        Commands::Logout => auth::logout(cfg),
        Commands::Whoami => commands::whoami(&client).await,
        Commands::Datasets => commands::list_datasets(&client).await,
        Commands::Dataset { id } => commands::get_dataset(&client, &id).await,
        Commands::Upload { file, name } => commands::upload_dataset(&client, &file, name.as_deref()).await,
        Commands::Models => commands::list_models(&client).await,
        Commands::Model { id } => commands::get_model(&client, &id).await,
        Commands::Train { dataset_id, arch, lr, epochs, batch_size, hidden_dim } => {
            commands::start_training(&client, &dataset_id, &arch, lr, epochs, batch_size, hidden_dim).await
        }
        Commands::Training => commands::list_training(&client).await,
        Commands::TrainingStatus { id } => commands::training_status(&client, &id).await,
        Commands::Stop { id } => commands::stop_training(&client, &id).await,
        Commands::Queue => commands::queue_status(&client).await,
        Commands::Validate { id } => commands::validate_dataset(&client, &id).await,
        Commands::Unlock { id } => commands::unlock_dataset(&client, &id).await,
        Commands::Monitor { id } => tui::run_tui(cfg, id).await,
        Commands::Deploy { model_id, target } => commands::deploy(&client, &model_id, target.as_deref()).await,
        Commands::Agent { message } => commands::agent_chat(&client, message.as_deref()).await,
        Commands::Compress { file, output, dict } => {
            compression::compress_file(&file, output.as_deref(), dict.as_deref())
        }
        Commands::Decompress { file, output } => {
            compression::decompress_file(&file, output.as_deref())
        }
        Commands::TrainCompressor { files, output } => {
            compression::train_dictionary(&files, &output)
        }
        Commands::Health => commands::health(&client).await,
        Commands::Config { key, value } => commands::config_cmd(cfg, key.as_deref(), value.as_deref()),
    }
}

async fn run_repl(cfg: &config::Config) -> anyhow::Result<()> {
    theme::print_banner();

    let client = api::ApiClient::new(&cfg.server_url, cfg.load_token());

    // Check auth status
    if client.token().is_none() {
        theme::print_warning("Not authenticated. Run /login to sign in.");
    } else {
        match client.get("/api/v1/auth/session").await {
            Ok(resp) => {
                if resp.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
                    let user = resp.get("user")
                        .and_then(|u| u.get("username"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    theme::print_success(&format!("Authenticated as {user}"));
                } else {
                    theme::print_warning("Session expired. Run /login to re-authenticate.");
                }
            }
            Err(_) => theme::print_warning("Cannot reach server. Check /config server_url"),
        }
    }

    println!();
    theme::print_info("Type /help for available commands, /quit to exit.");
    println!();

    let stdin = io::stdin();
    loop {
        theme::print_prompt();
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break; // EOF
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            "/quit" | "/exit" | "/q" => {
                theme::print_info("Goodbye.");
                break;
            }
            "/help" | "/h" => print_repl_help(),
            "/login" => { let _ = auth::login_flow(cfg).await; }
            "/logout" => { let _ = auth::logout(cfg); }
            "/whoami" => { let _ = commands::whoami(&client).await; }
            "/datasets" | "/ds" => { let _ = commands::list_datasets(&client).await; }
            "/models" | "/mdl" => { let _ = commands::list_models(&client).await; }
            "/training" | "/tr" => { let _ = commands::list_training(&client).await; }
            "/health" => { let _ = commands::health(&client).await; }
            "/monitor" => { let _ = tui::run_tui(cfg, None).await; }
            "/config" => { let _ = commands::config_cmd(cfg, None, None); }
            "/version" => println!("prometheus-cli {}", env!("CARGO_PKG_VERSION")),
            cmd if cmd.starts_with("/dataset ") => {
                let id = cmd.strip_prefix("/dataset ").unwrap().trim();
                let _ = commands::get_dataset(&client, id).await;
            }
            cmd if cmd.starts_with("/model ") => {
                let id = cmd.strip_prefix("/model ").unwrap().trim();
                let _ = commands::get_model(&client, id).await;
            }
            cmd if cmd.starts_with("/train ") => {
                let parts: Vec<&str> = cmd.strip_prefix("/train ").unwrap().trim().split_whitespace().collect();
                if parts.is_empty() {
                    theme::print_error("Usage: /train <dataset_id> [architecture]");
                } else {
                    let arch = parts.get(1).copied().unwrap_or("lstm_autoencoder");
                    let _ = commands::start_training(&client, parts[0], arch, None, None, None, None).await;
                }
            }
            cmd if cmd.starts_with("/stop ") => {
                let id = cmd.strip_prefix("/stop ").unwrap().trim();
                let _ = commands::stop_training(&client, id).await;
            }
            "/queue" => { let _ = commands::queue_status(&client).await; }
            cmd if cmd.starts_with("/validate ") => {
                let id = cmd.strip_prefix("/validate ").unwrap().trim();
                let _ = commands::validate_dataset(&client, id).await;
            }
            cmd if cmd.starts_with("/unlock ") => {
                let id = cmd.strip_prefix("/unlock ").unwrap().trim();
                let _ = commands::unlock_dataset(&client, id).await;
            }
            cmd if cmd.starts_with("/status ") => {
                let id = cmd.strip_prefix("/status ").unwrap().trim();
                let _ = commands::training_status(&client, id).await;
            }
            cmd if cmd.starts_with("/deploy ") => {
                let id = cmd.strip_prefix("/deploy ").unwrap().trim();
                let _ = commands::deploy(&client, id, None).await;
            }
            cmd if cmd.starts_with("/upload ") => {
                let path = cmd.strip_prefix("/upload ").unwrap().trim();
                let _ = commands::upload_dataset(&client, path, None).await;
            }
            cmd if cmd.starts_with("/monitor ") => {
                let id = cmd.strip_prefix("/monitor ").unwrap().trim();
                let _ = tui::run_tui(cfg, Some(id.to_string())).await;
            }
            cmd if cmd.starts_with("/compress ") => {
                let path = cmd.strip_prefix("/compress ").unwrap().trim();
                let _ = compression::compress_file(path, None, None);
            }
            cmd if cmd.starts_with("/decompress ") => {
                let path = cmd.strip_prefix("/decompress ").unwrap().trim();
                let _ = compression::decompress_file(path, None);
            }
            cmd if cmd.starts_with("/agent ") => {
                let msg = cmd.strip_prefix("/agent ").unwrap().trim();
                let _ = commands::agent_chat(&client, Some(msg)).await;
            }
            cmd if cmd.starts_with("/delete dataset ") => {
                let id = cmd.strip_prefix("/delete dataset ").unwrap().trim();
                match client.delete(&format!("/api/v1/datasets/{id}")).await {
                    Ok(_) => theme::print_success(&format!("Dataset {id} deleted")),
                    Err(e) => theme::print_error(&format!("Delete failed: {e}")),
                }
            }
            cmd if cmd.starts_with("/delete model ") => {
                let id = cmd.strip_prefix("/delete model ").unwrap().trim();
                match client.delete(&format!("/api/v1/models/{id}")).await {
                    Ok(_) => theme::print_success(&format!("Model {id} deleted")),
                    Err(e) => theme::print_error(&format!("Delete failed: {e}")),
                }
            }
            cmd if cmd.starts_with("/") => {
                theme::print_error(&format!("Unknown command: {cmd}. Type /help for commands."));
            }
            // Non-slash input: send to PrometheusForge agent
            msg => {
                let _ = commands::agent_chat(&client, Some(msg)).await;
            }
        }
        println!();
    }

    Ok(())
}

fn print_repl_help() {
    println!("{}", theme::styled_header("Prometheus CLI Commands"));
    println!();
    let cmds = [
        ("/login",                  "Authenticate via QR code / browser URL"),
        ("/logout",                 "Clear stored credentials"),
        ("/whoami",                 "Show current user"),
        ("/datasets, /ds",          "List datasets"),
        ("/dataset <id>",           "Show dataset details"),
        ("/upload <file>",          "Upload a CSV dataset"),
        ("/models, /mdl",           "List trained models"),
        ("/model <id>",             "Show model details"),
        ("/validate <ds_id>",       "Validate dataset for training"),
        ("/unlock <ds_id>",         "Unlock a validated dataset"),
        ("/train <ds_id> [arch]",   "Start training (13 architectures)"),
        ("/training, /tr",          "List training runs"),
        ("/queue",                  "Show training queue status"),
        ("/status <id>",            "Show training run progress"),
        ("/stop <id>",              "Stop a training run"),
        ("/monitor [id]",           "Open TUI training monitor"),
        ("/delete dataset <id>",     "Delete a dataset"),
        ("/delete model <id>",       "Delete a model"),
        ("/deploy <model_id>",      "Deploy model to edge device"),
        ("/agent <message>",        "Chat with PrometheusForge agent"),
        ("/compress <file>",        "Compress dataset with OpenZL"),
        ("/decompress <file>",      "Decompress an .ozl file"),
        ("/health",                 "Check server status"),
        ("/config [key] [value]",   "View or update configuration"),
        ("/version",                "Show CLI version"),
        ("/quit, /exit",            "Exit REPL"),
    ];
    for (cmd, desc) in cmds {
        println!("  \x1b[38;2;20;184;166m{:<26}\x1b[0m {}", cmd, desc);
    }
    println!();
    println!("  Architectures: lstm_autoencoder, gru_predictor, rnn, sentinel,");
    println!("                 resnet, vgg, vit, bert, gpt2, conv1d, conv2d, nexus, phantom");
    println!();
    println!("  Type any text without / to chat with PrometheusForge agent.");
}
