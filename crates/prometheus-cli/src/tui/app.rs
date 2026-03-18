// ============================================================================
// File: app.rs
// Description: TUI application state management with tabs, data, and HTTP client
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! TUI application state.

use serde_json::Value;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Dashboard,
    Datasets,
    Models,
    Monitor,
    Agent,
    Convert,
    Quantize,
    Deploy,
}

pub struct App {
    pub current_tab: Tab,
    pub server_url: String,
    pub token: Option<String>,
    pub focus_run_id: Option<String>,

    // Data
    pub training_runs: Vec<Value>,
    pub models: Vec<Value>,
    pub datasets: Vec<Value>,
    pub agent_messages: Vec<(String, String)>, // (role, content)
    pub selected_run: Option<Value>,
    pub selected_index: usize,

    // Status
    pub last_error: Option<String>,
    pub last_refresh: String,
    pub is_loading: bool,

    // HTTP client
    client: reqwest::Client,
}

impl App {
    pub fn new(server_url: String, token: Option<String>, focus_run_id: Option<String>) -> Self {
        Self {
            current_tab: if focus_run_id.is_some() { Tab::Monitor } else { Tab::Dashboard },
            server_url,
            token,
            focus_run_id,
            training_runs: Vec::new(),
            models: Vec::new(),
            datasets: Vec::new(),
            agent_messages: vec![("assistant".into(), "I'm PrometheusForge. Ask me about your data, architectures, or training.".into())],
            selected_run: None,
            selected_index: 0,
            last_error: None,
            last_refresh: String::new(),
            is_loading: false,
            client: reqwest::Client::new(),
        }
    }

    pub async fn refresh(&mut self) {
        self.is_loading = true;
        self.last_error = None;

        // Fetch training runs
        match self.api_get("/api/v1/training").await {
            Ok(data) => {
                self.training_runs = data.as_array().cloned().unwrap_or_default();
            }
            Err(e) => self.last_error = Some(e.to_string()),
        }

        // Fetch datasets
        match self.api_get("/api/v1/datasets").await {
            Ok(data) => {
                self.datasets = data.as_array().cloned().unwrap_or_default();
            }
            Err(_) => {}
        }

        // Fetch models
        match self.api_get("/api/v1/models").await {
            Ok(data) => {
                self.models = data.as_array().cloned().unwrap_or_default();
            }
            Err(e) => {
                if self.last_error.is_none() {
                    self.last_error = Some(e.to_string());
                }
            }
        }

        // If we have a focused run, fetch its details
        if let Some(ref run_id) = self.focus_run_id {
            let path = format!("/api/v1/training/{run_id}");
            match self.api_get(&path).await {
                Ok(data) => self.selected_run = Some(data),
                Err(_) => {}
            }
        } else if !self.training_runs.is_empty() && self.selected_run.is_none() {
            // Auto-select the first running training
            let running = self.training_runs.iter()
                .find(|r| r.get("status").and_then(|v| v.as_str()) == Some("running"))
                .or_else(|| self.training_runs.first());
            if let Some(run) = running {
                if let Some(id) = run.get("id").and_then(|v| v.as_str()) {
                    let path = format!("/api/v1/training/{id}");
                    if let Ok(data) = self.api_get(&path).await {
                        self.selected_run = Some(data);
                    }
                }
            }
        }

        self.last_refresh = chrono::Local::now().format("%H:%M:%S").to_string();
        self.is_loading = false;
    }

    async fn api_get(&self, path: &str) -> anyhow::Result<Value> {
        let url = format!("{}{}", self.server_url, path);
        let mut req = self.client.get(&url);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("API error: {}", resp.status());
        }
        Ok(resp.json().await?)
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Dashboard => Tab::Datasets,
            Tab::Datasets => Tab::Models,
            Tab::Models => Tab::Monitor,
            Tab::Monitor => Tab::Agent,
            Tab::Agent => Tab::Convert,
            Tab::Convert => Tab::Quantize,
            Tab::Quantize => Tab::Deploy,
            Tab::Deploy => Tab::Dashboard,
        };
    }

    pub fn next_item(&mut self) {
        if !self.training_runs.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.training_runs.len();
        }
    }

    pub fn previous_item(&mut self) {
        if !self.training_runs.is_empty() {
            self.selected_index = self.selected_index.checked_sub(1)
                .unwrap_or(self.training_runs.len() - 1);
        }
    }

    pub fn select_item(&mut self) {
        if let Some(run) = self.training_runs.get(self.selected_index) {
            self.selected_run = Some(run.clone());
            self.focus_run_id = run.get("id").and_then(|v| v.as_str()).map(String::from);
            self.current_tab = Tab::Monitor;
        }
    }
}
