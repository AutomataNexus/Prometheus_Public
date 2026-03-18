// ============================================================================
// File: training_detail.rs
// Description: Training run detail page with live loss curves and run metadata
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use leptos_router::hooks::use_params_map;
use crate::components::*;
use crate::components::chart::DataPoint;
use crate::icons;

#[component]
pub fn TrainingDetailPage() -> impl IntoView {
    let params = use_params_map();
    let training_run = RwSignal::new(None::<serde_json::Value>);
    let loss_data = RwSignal::new(Vec::<DataPoint>::new());
    let val_loss_data = RwSignal::new(Vec::<DataPoint>::new());
    let show_retrain = RwSignal::new(false);
    let rt_epochs = RwSignal::new("100".to_string());
    let rt_lr = RwSignal::new("0.001".to_string());
    let rt_batch = RwSignal::new("64".to_string());
    let rt_hidden = RwSignal::new("64".to_string());
    let rt_seq = RwSignal::new("60".to_string());
    let rt_layers = RwSignal::new("2".to_string());

    // Update loss chart data from training run
    let update_charts = move |data: &serde_json::Value| {
        if let Some(metrics) = data.get("epoch_metrics").and_then(|m| m.as_array()) {
            let train_loss: Vec<DataPoint> = metrics.iter().enumerate().map(|(i, m)| {
                DataPoint {
                    x: i as f64,
                    y: m.get("train_loss").and_then(|v| v.as_f64()).unwrap_or(0.0),
                }
            }).collect();
            let vl: Vec<DataPoint> = metrics.iter().enumerate().map(|(i, m)| {
                DataPoint {
                    x: i as f64,
                    y: m.get("val_loss").and_then(|v| v.as_f64()).unwrap_or(0.0),
                }
            }).collect();
            loss_data.set(train_loss);
            val_loss_data.set(vl);
        }
    };

    // Initial fetch + WebSocket for live updates
    {
        let training_run = training_run;
        leptos::task::spawn_local(async move {
            let run_id = params.get_untracked().get("id").unwrap_or_default();

            // Initial fetch
            if let Ok(resp) = crate::api::auth_get(&format!("/api/v1/training/{run_id}"))
                .send()
                .await
            {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    update_charts(&data);
                    let status = data.get("status").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    training_run.set(Some(data));

                    // Connect WebSocket only if training is still running
                    if status == "running" {
                        connect_training_ws(&run_id, training_run, loss_data, val_loss_data).await;
                    }
                }
            }
        });
    }

    let is_running = move || {
        training_run.get()
            .and_then(|r| r.get("status").and_then(|s| s.as_str()).map(|s| s == "running"))
            .unwrap_or(false)
    };

    let on_stop = move |_| {
        let run_id = params.get().get("id").unwrap_or_default();
        leptos::task::spawn_local(async move {
            let _ = crate::api::auth_post(&format!("/api/v1/training/{run_id}/stop"))
                .send()
                .await;
        });
    };

    view! {
        <div>
            <Show
                when=move || training_run.get().is_some()
                fallback=|| view! { <PageLoader /> }
            >
                {move || {
                    let run = match training_run.get() {
                        Some(r) => r,
                        None => return view! { <PageLoader /> }.into_any(),
                    };
                    let id = run.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let arch = run.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let status = run.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string();
                    let current_epoch = run.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0);
                    let total_epochs = run.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(100).max(1);
                    let best_loss = run.get("best_val_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let lr = run.get("hyperparameters").and_then(|h| h.get("learning_rate")).and_then(|v| v.as_f64()).unwrap_or(0.001);

                    view! {
                        <div>
                            <div class="flex-between mb-8">
                                <div>
                                    <h1 class="page-title">{format!("Training: {id}")}</h1>
                                    <p class="page-subtitle">{format!("{arch} \u{2022} Epoch {current_epoch}/{total_epochs}")}</p>
                                </div>
                                <div style="display:flex;gap:8px;">
                                    <Show when=is_running>
                                        <button class="btn btn-danger" on:click=on_stop>
                                            {icons::icon_stop()}
                                            " Stop Training"
                                        </button>
                                    </Show>
                                    <Show when=move || {
                                        let s = training_run.get().and_then(|r| r.get("status").and_then(|s| s.as_str()).map(String::from)).unwrap_or_default();
                                        s == "completed" || s == "failed"
                                    }>
                                        <button
                                            class="btn btn-secondary"
                                            on:click=move |_| {
                                                if let Some(ref r) = training_run.get() {
                                                    let hp = r.get("hyperparameters").cloned().unwrap_or(serde_json::json!({}));
                                                    rt_epochs.set(hp.get("epochs").and_then(|v| v.as_u64()).unwrap_or(100).to_string());
                                                    rt_lr.set(hp.get("learning_rate").and_then(|v| v.as_f64()).unwrap_or(0.001).to_string());
                                                    rt_batch.set(hp.get("batch_size").and_then(|v| v.as_u64()).unwrap_or(64).to_string());
                                                    rt_hidden.set(hp.get("hidden_dim").and_then(|v| v.as_u64()).unwrap_or(64).to_string());
                                                    rt_seq.set(hp.get("sequence_length").and_then(|v| v.as_u64()).unwrap_or(60).to_string());
                                                    rt_layers.set(hp.get("num_layers").and_then(|v| v.as_u64()).unwrap_or(2).to_string());
                                                }
                                                show_retrain.set(true);
                                            }
                                        >
                                            {icons::icon_refresh()}
                                            " Retrain from Checkpoint"
                                        </button>
                                    </Show>
                                </div>
                            </div>

                            // Progress
                            <Card title="Progress" class="mb-8">
                                <div class="flex-between mb-4">
                                    <Badge status=badge::status_to_badge(&status) />
                                    <span class="text-sm text-muted">
                                        {format!("{:.0}%", current_epoch as f64 / total_epochs as f64 * 100.0)}
                                    </span>
                                </div>
                                <div class="progress-bar" style="height: 12px;">
                                    <div class="progress-bar-fill" style=format!(
                                        "width: {}%", current_epoch as f64 / total_epochs as f64 * 100.0
                                    )></div>
                                </div>
                            </Card>

                            // Metrics
                            <div class="metric-grid mb-8">
                                <MetricCard label="Best Val Loss" value=Signal::derive(move || format!("{best_loss:.4}")) tooltip="Lowest validation loss achieved so far. Measures how well the model generalizes to unseen data. Lower is better." />
                                <MetricCard label="Current Epoch" value=Signal::derive(move || format!("{current_epoch}/{total_epochs}")) tooltip="Training progress. One epoch = one complete pass through the entire training dataset." />
                                <MetricCard label="Learning Rate" value=Signal::derive(move || format!("{lr:.1e}")) tooltip="Step size for gradient descent. Too high = unstable training. Too low = slow convergence. Typical: 0.001." />
                                <MetricCard label="Architecture" value=Signal::derive({
                                    let arch = arch.clone();
                                    move || arch.clone()
                                }) />
                            </div>

                            // Loss Charts
                            <div class="grid-2">
                                <Card title="Training Loss">
                                    <LineChart
                                        data=Signal::derive(move || loss_data.get())
                                        color="#14b8a6"
                                        x_label="Epoch"
                                        y_label="Loss"
                                    />
                                </Card>
                                <Card title="Validation Loss">
                                    <LineChart
                                        data=Signal::derive(move || val_loss_data.get())
                                        color="#C2714F"
                                        x_label="Epoch"
                                        y_label="Loss"
                                    />
                                </Card>
                            </div>

                            // Hyperparameters
                            <div class="mt-8">
                                <Card title="Hyperparameters">
                                    {move || {
                                        let run = match training_run.get() {
                                            Some(r) => r,
                                            None => return view! { <span>"Loading..."</span> }.into_any(),
                                        };
                                        let hp = run.get("hyperparameters").cloned().unwrap_or(serde_json::json!({}));
                                        let json_str = serde_json::to_string_pretty(&hp).unwrap_or_default();
                                        view! { <CodeBlock code=json_str language="json" /> }.into_any()
                                    }}
                                </Card>
                            </div>
                        </div>
                    }.into_any()
                }}
            </Show>

            // Retrain Modal
            <Show when=move || show_retrain.get()>
                <div class="modal-backdrop" on:click=move |_| show_retrain.set(false)>
                    <div class="modal" on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation() style="max-width: 500px;">
                        <h2 class="modal-title">"Retrain from Checkpoint"</h2>
                        <p class="text-sm text-muted mb-4">"Adjust hyperparameters and resume training from existing weights."</p>
                        <div style="display:grid;gap:12px;margin-bottom:16px;">
                            <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;">
                                <div>
                                    <label class="input-label">"Epochs"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || rt_epochs.get()
                                        on:input=move |ev| rt_epochs.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Batch Size"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || rt_batch.get()
                                        on:input=move |ev| rt_batch.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Learning Rate"</label>
                                    <input class="input-field" type="number" step="0.0001"
                                        prop:value=move || rt_lr.get()
                                        on:input=move |ev| rt_lr.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Hidden Dim"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || rt_hidden.get()
                                        on:input=move |ev| rt_hidden.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Sequence Length"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || rt_seq.get()
                                        on:input=move |ev| rt_seq.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Layers"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || rt_layers.get()
                                        on:input=move |ev| rt_layers.set(event_target_value(&ev)) />
                                </div>
                            </div>
                        </div>
                        <div class="modal-actions">
                            <button class="btn btn-ghost" on:click=move |_| show_retrain.set(false)>"Cancel"</button>
                            <button class="btn btn-primary" on:click=move |_| {
                                show_retrain.set(false);
                                if let Some(ref r) = training_run.get() {
                                    let mid = r.get("model_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let did = r.get("dataset_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let ar = r.get("architecture").and_then(|v| v.as_str()).unwrap_or("lstm_autoencoder").to_string();
                                    let epochs: u64 = rt_epochs.get_untracked().parse().unwrap_or(100);
                                    let batch: u64 = rt_batch.get_untracked().parse().unwrap_or(64);
                                    let lr: f64 = rt_lr.get_untracked().parse().unwrap_or(0.001);
                                    let hidden: u64 = rt_hidden.get_untracked().parse().unwrap_or(64);
                                    let seq_len: u64 = rt_seq.get_untracked().parse().unwrap_or(60);
                                    let layers: u64 = rt_layers.get_untracked().parse().unwrap_or(2);
                                    leptos::task::spawn_local(async move {
                                        let body = serde_json::json!({
                                            "dataset_id": did,
                                            "architecture": ar,
                                            "resume_from_model": mid,
                                            "hyperparameters": {
                                                "epochs": epochs,
                                                "batch_size": batch,
                                                "learning_rate": lr,
                                                "hidden_dim": hidden,
                                                "sequence_length": seq_len,
                                                "num_layers": layers,
                                            }
                                        });
                                        let _ = crate::api::auth_post("/api/v1/training/start")
                                            .header("Content-Type", "application/json")
                                            .body(body.to_string())
                                            .unwrap()
                                            .send()
                                            .await;
                                        if let Some(window) = web_sys::window() {
                                            let _ = window.location().set_href("/training");
                                        }
                                    });
                                }
                            }>
                                {icons::icon_play()}
                                " Start Retrain"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Connect to the training WebSocket for live epoch updates.
/// Receives epoch_update and training_complete messages and updates signals.
async fn connect_training_ws(
    run_id: &str,
    training_run: RwSignal<Option<serde_json::Value>>,
    loss_data: RwSignal<Vec<DataPoint>>,
    val_loss_data: RwSignal<Vec<DataPoint>>,
) {
    use wasm_bindgen::prelude::*;
    use web_sys::{MessageEvent, WebSocket};

    let location = web_sys::window().unwrap().location();
    let protocol = if location.protocol().unwrap_or_default() == "https:" { "wss" } else { "ws" };
    let host = location.host().unwrap_or_else(|_| "localhost:3030".into());
    let ws_url = format!("{protocol}://{host}/ws/training/{run_id}");

    let ws = match WebSocket::new(&ws_url) {
        Ok(ws) => ws,
        Err(_) => return,
    };

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Ok(text) = event.data().dyn_into::<js_sys::JsString>() {
            let text: String = text.into();
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&text) {
                let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match msg_type {
                    "epoch_update" | "training_complete" => {
                        if let Some(mut run) = training_run.get() {
                            if let Some(obj) = run.as_object_mut() {
                                if let Some(epoch) = msg.get("current_epoch") {
                                    obj.insert("current_epoch".into(), epoch.clone());
                                }
                                if let Some(loss) = msg.get("best_val_loss") {
                                    obj.insert("best_val_loss".into(), loss.clone());
                                }
                                let metrics_key = if msg_type == "training_complete" { "final_metrics" } else { "epoch_metrics" };
                                if let Some(metrics) = msg.get(metrics_key) {
                                    obj.insert("epoch_metrics".into(), metrics.clone());
                                }
                                if let Some(status) = msg.get("status") {
                                    obj.insert("status".into(), status.clone());
                                }
                            }

                            // Update charts from epoch_metrics
                            if let Some(metrics) = run.get("epoch_metrics").and_then(|m| m.as_array()) {
                                let tl: Vec<DataPoint> = metrics.iter().enumerate().map(|(i, m)| {
                                    DataPoint {
                                        x: i as f64,
                                        y: m.get("train_loss").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                    }
                                }).collect();
                                let vl: Vec<DataPoint> = metrics.iter().enumerate().map(|(i, m)| {
                                    DataPoint {
                                        x: i as f64,
                                        y: m.get("val_loss").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                    }
                                }).collect();
                                loss_data.set(tl);
                                val_loss_data.set(vl);
                            }

                            training_run.set(Some(run));
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
}
