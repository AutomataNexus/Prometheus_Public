// ============================================================================
// File: training.rs
// Description: Training jobs listing page with new training run creation
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use leptos::callback::Callback;
use crate::components::*;
use crate::icons;

#[component]
pub fn TrainingPage() -> impl IntoView {
    let training_runs = RwSignal::new(Vec::<serde_json::Value>::new());
    let show_start = RwSignal::new(false);
    let selected_dataset = RwSignal::new(String::new());
    let datasets = RwSignal::new(Vec::<serde_json::Value>::new());

    // Hyperparameter signals (editable by user)
    let hp_architecture = RwSignal::new("lstm_autoencoder".to_string());
    let hp_epochs = RwSignal::new("100".to_string());
    let hp_batch_size = RwSignal::new("64".to_string());
    let hp_learning_rate = RwSignal::new("0.001".to_string());
    let hp_hidden_dim = RwSignal::new("64".to_string());
    let hp_sequence_length = RwSignal::new("60".to_string());
    let hp_num_layers = RwSignal::new("2".to_string());
    let resume_from_model = RwSignal::new(String::new());
    let models = RwSignal::new(Vec::<serde_json::Value>::new());

    // Fetch training runs and datasets on mount
    {
        let training_runs = training_runs;
        let datasets = datasets;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/training").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    training_runs.set(data);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/datasets").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    datasets.set(data);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/models").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    models.set(data);
                }
            }
        });
    }

    // Auto-refresh every 3 seconds while training is active
    {
        let training_runs = training_runs;
        leptos::task::spawn_local(async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(3000).await;
                if let Ok(resp) = crate::api::auth_get("/api/v1/training").send().await {
                    if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                        training_runs.set(data);
                    }
                }
            }
        });
    }

    let active_runs = Signal::derive(move || {
        training_runs.get().iter().filter(|r| {
            r.get("status").and_then(|s| s.as_str()) == Some("running")
        }).cloned().collect::<Vec<_>>()
    });

    let table_columns = vec![
        Column { key: "id".into(), label: "Run ID".into(), sortable: true },
        Column { key: "architecture".into(), label: "Architecture".into(), sortable: true },
        Column { key: "dataset".into(), label: "Dataset".into(), sortable: false },
        Column { key: "status".into(), label: "Status".into(), sortable: true },
        Column { key: "epoch".into(), label: "Epoch".into(), sortable: false },
        Column { key: "loss".into(), label: "Best Loss".into(), sortable: true },
        Column { key: "time".into(), label: "Duration".into(), sortable: true },
    ];

    let table_rows: Signal<Vec<Vec<String>>> = Signal::derive(move || {
        training_runs.get().iter().map(|r| {
            vec![
                r.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                r.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                r.get("dataset_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                r.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string(),
                format!("{}/{}",
                    r.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0),
                    r.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(100)
                ),
                r.get("best_val_loss").and_then(|v| v.as_f64()).map(|v| format!("{v:.4}")).unwrap_or_default(),
                r.get("training_time_seconds").and_then(|v| v.as_u64()).map(|s| format_duration(s)).unwrap_or_default(),
            ]
        }).collect()
    });

    let on_row_click = Callback::new(move |idx: usize| {
        let runs = training_runs.get();
        if let Some(run) = runs.get(idx) {
            if let Some(id) = run.get("id").and_then(|v| v.as_str()) {
                if let Some(window) = web_sys::window() {
                    let _ = window.location().set_href(&format!("/training/{id}"));
                }
            }
        }
    });

    let on_start_training = move |_| {
        let ds_id = selected_dataset.get();
        if ds_id.is_empty() {
            return;
        }
        show_start.set(false);
        let arch = hp_architecture.get_untracked();
        let epochs: u64 = hp_epochs.get_untracked().parse().unwrap_or(100);
        let batch: u64 = hp_batch_size.get_untracked().parse().unwrap_or(64);
        let lr: f64 = hp_learning_rate.get_untracked().parse().unwrap_or(0.001);
        let hidden: u64 = hp_hidden_dim.get_untracked().parse().unwrap_or(64);
        let seq_len: u64 = hp_sequence_length.get_untracked().parse().unwrap_or(60);
        let layers: u64 = hp_num_layers.get_untracked().parse().unwrap_or(2);
        let resume = resume_from_model.get_untracked();
        leptos::task::spawn_local(async move {
            let mut body = serde_json::json!({
                "dataset_id": ds_id,
                "architecture": arch,
                "hyperparameters": {
                    "epochs": epochs,
                    "batch_size": batch,
                    "learning_rate": lr,
                    "hidden_dim": hidden,
                    "sequence_length": seq_len,
                    "num_layers": layers,
                }
            });
            if !resume.is_empty() {
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("resume_from_model".into(), serde_json::json!(resume));
                }
            }
            if let Ok(resp) = crate::api::auth_post("/api/v1/training/start")
                .header("Content-Type", "application/json")
                .body(body.to_string())
                .unwrap()
                .send()
                .await
            {
                if let Ok(run) = resp.json::<serde_json::Value>().await {
                    if let Some(id) = run.get("id").and_then(|v| v.as_str()) {
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().set_href(&format!("/training/{id}"));
                        }
                    }
                }
            }
        });
    };

    let clearing = RwSignal::new(false);

    let has_clearable = Signal::derive(move || {
        training_runs.get().iter().any(|r| {
            matches!(
                r.get("status").and_then(|s| s.as_str()),
                Some("completed") | Some("failed") | Some("cancelled") | Some("stopped")
            )
        })
    });

    let on_clear = move |_| {
        clearing.set(true);
        let training_runs = training_runs;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_post("/api/v1/training/clear")
                .send()
                .await
            {
                let _ = resp.json::<serde_json::Value>().await;
            }
            // Refresh the list
            if let Ok(resp) = crate::api::auth_get("/api/v1/training").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    training_runs.set(data);
                }
            }
            clearing.set(false);
        });
    };

    view! {
        <div>
            <div class="flex-between mb-8">
                <div>
                    <h1 class="page-title">"Training"</h1>
                    <p class="page-subtitle">"Manage AxonML training jobs"</p>
                </div>
                <div style="display: flex; gap: 8px;">
                    <Show when=move || has_clearable.get()>
                        <button
                            class="btn btn-ghost btn-sm"
                            style="color: #ef4444; border: 1px solid #ef4444;"
                            on:click=on_clear
                            disabled=move || clearing.get()
                        >
                            {move || if clearing.get() { "Clearing..." } else { "Clear Training" }}
                        </button>
                    </Show>
                    <button class="btn btn-primary" on:click=move |_| show_start.set(true)>
                        {icons::icon_play()}
                        " Start Training"
                    </button>
                </div>
            </div>

            // Active Training Runs
            <Show when=move || !active_runs.get().is_empty()>
                <div class="mb-8">
                    <h2 class="text-bold mb-4">"Active Training"</h2>
                    <div class="grid-2">
                        {move || active_runs.get().into_iter().map(|run| {
                            let id = run.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let arch = run.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let current = run.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0);
                            let total = run.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(100);
                            let pct = if total > 0 { current as f64 / total as f64 * 100.0 } else { 0.0 };
                            let run_id = id.clone();

                            view! {
                                <a href=format!("/training/{run_id}") style="text-decoration: none;">
                                    <Card>
                                        <div class="flex-between mb-4">
                                            <span class="text-bold">{id}</span>
                                            <Badge status=BadgeStatus::Training />
                                        </div>
                                        <p class="text-sm text-muted mb-4">{arch}</p>
                                        <div class="flex-between mb-4">
                                            <span class="text-sm">{format!("Epoch {current}/{total}")}</span>
                                            <span class="text-sm text-muted">{format!("{pct:.0}%")}</span>
                                        </div>
                                        <div class="progress-bar">
                                            <div class="progress-bar-fill" style=format!("width: {pct}%")></div>
                                        </div>
                                    </Card>
                                </a>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </Show>

            // Training History
            <Card title="Training History">
                <DataTable
                    columns=table_columns
                    rows=table_rows
                    on_row_click=on_row_click
                    empty_message="No training runs yet. Start your first training job."
                />
            </Card>

            // Start Training Modal
            <Show when=move || show_start.get()>
                <div class="modal-backdrop" on:click=move |_| show_start.set(false)>
                    <div class="modal" on:click=move |ev| ev.stop_propagation() style="max-width: 600px;">
                        <h2 class="modal-title">"Start Training"</h2>
                        <p class="text-sm text-muted mb-4">"Select a dataset and configure hyperparameters."</p>
                        <div style="display:grid;gap:12px;margin-bottom:16px;">
                            <div>
                                <label class="input-label">"Dataset"</label>
                                <select
                                    class="input-field"
                                    style="width: 100%;"
                                    prop:value=move || selected_dataset.get()
                                    on:change=move |ev| selected_dataset.set(event_target_value(&ev))
                                >
                                    <option value="">"Select a dataset..."</option>
                                    {move || datasets.get().into_iter().map(|ds| {
                                        let id = ds.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let name = ds.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let id_val = id.clone();
                                        view! { <option value=id_val>{name}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                            <div>
                                <label class="input-label">"Resume from Existing Model"</label>
                                <select
                                    class="input-field"
                                    style="width: 100%;"
                                    prop:value=move || resume_from_model.get()
                                    on:change=move |ev| resume_from_model.set(event_target_value(&ev))
                                >
                                    <option value="">"Start fresh (no checkpoint)"</option>
                                    {move || models.get().into_iter().filter(|m| {
                                        m.get("status").and_then(|v| v.as_str()).unwrap_or("") == "ready"
                                    }).map(|m| {
                                        let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let id_val = id.clone();
                                        view! { <option value=id_val>{format!("Resume: {name}")}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                            <div>
                                <label class="input-label">"Architecture"</label>
                                <select
                                    class="input-field"
                                    style="width: 100%;"
                                    prop:value=move || hp_architecture.get()
                                    on:change=move |ev| hp_architecture.set(event_target_value(&ev))
                                >
                                    <option value="lstm_autoencoder">"LSTM Autoencoder"</option>
                                    <option value="gru_predictor">"GRU Predictor"</option>
                                    <option value="rnn">"RNN"</option>
                                    <option value="sentinel">"Sentinel (MLP)"</option>
                                    <option value="res_net">"ResNet"</option>
                                    <option value="vgg">"VGG"</option>
                                    <option value="vi_t">"Vision Transformer"</option>
                                    <option value="bert">"BERT"</option>
                                    <option value="gpt2">"GPT-2"</option>
                                    <option value="nexus">"Nexus (Multi-modal)"</option>
                                    <option value="phantom">"Phantom (Edge)"</option>
                                    <option value="conv1d">"Conv1D"</option>
                                    <option value="conv2d">"Conv2D"</option>
                                </select>
                            </div>
                            <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;">
                                <div>
                                    <label class="input-label">"Epochs"</label>
                                    <input class="input-field" type="number" prop:value=move || hp_epochs.get()
                                        on:input=move |ev| hp_epochs.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Batch Size"</label>
                                    <input class="input-field" type="number" prop:value=move || hp_batch_size.get()
                                        on:input=move |ev| hp_batch_size.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Learning Rate"</label>
                                    <input class="input-field" type="number" step="0.0001" prop:value=move || hp_learning_rate.get()
                                        on:input=move |ev| hp_learning_rate.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Hidden Dim"</label>
                                    <input class="input-field" type="number" prop:value=move || hp_hidden_dim.get()
                                        on:input=move |ev| hp_hidden_dim.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Sequence Length"</label>
                                    <input class="input-field" type="number" prop:value=move || hp_sequence_length.get()
                                        on:input=move |ev| hp_sequence_length.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Layers"</label>
                                    <input class="input-field" type="number" prop:value=move || hp_num_layers.get()
                                        on:input=move |ev| hp_num_layers.set(event_target_value(&ev)) />
                                </div>
                            </div>
                        </div>
                        <div class="modal-actions">
                            <button class="btn btn-ghost" on:click=move |_| show_start.set(false)>"Cancel"</button>
                            <button class="btn btn-primary" on:click=on_start_training>
                                {icons::icon_play()}
                                " Start Training"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
