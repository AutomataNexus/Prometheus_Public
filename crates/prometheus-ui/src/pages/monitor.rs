// ============================================================================
// File: monitor.rs
// Description: Real-time training monitor page with loss charts and queue status
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use crate::components::*;
use crate::components::chart::DataPoint;
use crate::icons;

#[component]
pub fn MonitorPage() -> impl IntoView {
    let training_runs = RwSignal::new(Vec::<serde_json::Value>::new());
    let queue_info = RwSignal::new(None::<serde_json::Value>);
    let selected_run = RwSignal::new(None::<String>);
    let auto_refresh = RwSignal::new(true);

    // Initial fetch
    {
        let training_runs = training_runs;
        let queue_info = queue_info;
        leptos::task::spawn_local(async move {
            fetch_all(training_runs, queue_info).await;
        });
    }

    // Auto-refresh every 5 seconds
    {
        let training_runs = training_runs;
        let queue_info = queue_info;
        leptos::task::spawn_local(async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(2000).await;
                if auto_refresh.get_untracked() {
                    fetch_all(training_runs, queue_info).await;
                }
            }
        });
    }

    let active_runs = Signal::derive(move || {
        training_runs.get().iter().filter(|r| {
            matches!(
                r.get("status").and_then(|s| s.as_str()),
                Some("running") | Some("queued")
            )
        }).cloned().collect::<Vec<_>>()
    });

    let completed_runs = Signal::derive(move || {
        training_runs.get().iter().filter(|r| {
            matches!(
                r.get("status").and_then(|s| s.as_str()),
                Some("completed") | Some("failed") | Some("cancelled")
            )
        }).cloned().collect::<Vec<_>>()
    });

    let total_active = Signal::derive(move || {
        training_runs.get().iter().filter(|r| {
            r.get("status").and_then(|s| s.as_str()) == Some("running")
        }).count()
    });

    let total_queued = Signal::derive(move || {
        training_runs.get().iter().filter(|r| {
            r.get("status").and_then(|s| s.as_str()) == Some("queued")
        }).count()
    });

    view! {
        <div>
            <div class="flex-between mb-8">
                <div>
                    <h1 class="page-title">"Training Monitor"</h1>
                    <p class="page-subtitle">"Real-time training dashboard"</p>
                </div>
                <div style="display: flex; align-items: center; gap: 12px;">
                    // Auto-refresh toggle
                    <label style="display: flex; align-items: center; gap: 6px; cursor: pointer; font-size: 0.85rem; color: #6b7280;">
                        <input
                            type="checkbox"
                            prop:checked=move || auto_refresh.get()
                            on:change=move |ev| auto_refresh.set(event_target_checked(&ev))
                            style="accent-color: #14b8a6;"
                        />
                        "Auto-refresh"
                    </label>
                    // Manual refresh
                    <button
                        class="btn btn-ghost btn-sm"
                        on:click=move |_| {
                            let training_runs = training_runs;
                            let queue_info = queue_info;
                            leptos::task::spawn_local(async move {
                                fetch_all(training_runs, queue_info).await;
                            });
                        }
                    >
                        {icons::icon_activity()}
                        " Refresh"
                    </button>
                </div>
            </div>

            // ── Server Capacity Bar ────────────────────────────────
            {move || {
                let qi = queue_info.get();
                let active = qi.as_ref().and_then(|q| q.get("active_trainings").and_then(|v| v.as_u64())).unwrap_or(0);
                let max = qi.as_ref().and_then(|q| q.get("max_concurrent").and_then(|v| v.as_u64())).unwrap_or(1);
                let queued = qi.as_ref().and_then(|q| q.get("queued").and_then(|v| v.as_u64())).unwrap_or(0);
                let pct = if max > 0 { (active as f64 / max as f64 * 100.0).min(100.0) } else { 0.0 };
                let bar_color = if pct >= 100.0 { "#ef4444" } else if pct >= 75.0 { "#f59e0b" } else { "#14b8a6" };

                view! {
                    <div class="prometheus-card" style="padding: 16px 20px; margin-bottom: 24px;">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                            <span style="font-weight: 600; font-size: 0.9rem; color: #374151;">"Server Capacity"</span>
                            <div style="display: flex; gap: 16px; font-size: 0.8rem;">
                                <span style="color: #14b8a6;" title="Currently running training jobs">
                                    {format!("{active} running")}
                                </span>
                                <span
                                    style=move || if { queued > 0 } { "color: #f59e0b;" } else { "color: #9ca3af;" }
                                    title="Training jobs waiting for a slot"
                                >
                                    {format!("{queued} queued")}
                                </span>
                                <span style="color: #6b7280;" title="Maximum concurrent training runs (PROMETHEUS_MAX_TRAININGS)">
                                    {format!("max {max}")}
                                </span>
                            </div>
                        </div>
                        <div style="height: 8px; background: #f3f0ec; border-radius: 4px; overflow: hidden;">
                            <div style=format!(
                                "height: 100%; width: {pct:.0}%; background: {bar_color}; border-radius: 4px; transition: width 0.5s ease;"
                            )></div>
                        </div>
                    </div>
                }
            }}

            // ── Metric Cards ──────────────────────────────────
            <div class="metric-grid mb-8">
                <MetricCard label="Running" value=Signal::derive(move || total_active.get().to_string()) tooltip="Training jobs currently executing on the server. Limited by max concurrent trainings." />
                <MetricCard label="Queued" value=Signal::derive(move || total_queued.get().to_string()) tooltip="Jobs waiting for a training slot to open. Will start automatically when capacity is available." />
                <MetricCard label="Total Runs" value=Signal::derive(move || training_runs.get().len().to_string()) tooltip="Total number of training runs across all statuses (running, queued, completed, failed)." />
                <MetricCard label="Completed" value=Signal::derive(move || {
                    training_runs.get().iter().filter(|r| {
                        r.get("status").and_then(|s| s.as_str()) == Some("completed")
                    }).count().to_string()
                }) />
            </div>

            // ── Active / Queued Runs ──────────────────────────────────
            <Show when=move || !active_runs.get().is_empty()>
                <h2 style="font-size: 1.1rem; font-weight: 600; color: #111827; margin-bottom: 16px;">"Active &amp; Queued Runs"</h2>
                <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 32px;">
                    {move || active_runs.get().into_iter().map(|run| {
                        view! { <RunCard run=run.clone() selected_run=selected_run training_runs=training_runs /> }
                    }).collect_view()}
                </div>
            </Show>

            // ── Selected Run Detail ──────────────────────────────────
            {move || {
                let sel_id = selected_run.get();
                if sel_id.is_none() {
                    return view! { <div></div> }.into_any();
                }
                let sel_id = sel_id.unwrap();
                let run = training_runs.get().into_iter().find(|r| {
                    r.get("id").and_then(|v| v.as_str()) == Some(&sel_id)
                });
                if run.is_none() {
                    return view! { <div></div> }.into_any();
                }
                let run = run.unwrap();
                view! { <RunDetailPanel run=run /> }.into_any()
            }}

            // ── Recent Completed ──────────────────────────────────
            <Show when=move || !completed_runs.get().is_empty()>
                <h2 style="font-size: 1.1rem; font-weight: 600; color: #111827; margin: 32px 0 16px 0;">"Recent Completed"</h2>
                <div class="prometheus-card" style="overflow-x: auto;">
                    <table style="width: 100%; border-collapse: collapse; font-size: 0.85rem;">
                        <thead>
                            <tr style="border-bottom: 1px solid #E8D4C4; text-align: left;">
                                <th style="padding: 10px 12px; color: #6b7280; font-weight: 500;">"Run ID"</th>
                                <th style="padding: 10px 12px; color: #6b7280; font-weight: 500;"
                                    title="Neural network architecture used">"Architecture"</th>
                                <th style="padding: 10px 12px; color: #6b7280; font-weight: 500;">"Status"</th>
                                <th style="padding: 10px 12px; color: #6b7280; font-weight: 500;"
                                    title="Best validation loss achieved during training">"Best Val Loss"</th>
                                <th style="padding: 10px 12px; color: #6b7280; font-weight: 500;"
                                    title="Total training wall-clock time">"Duration"</th>
                                <th style="padding: 10px 12px; color: #6b7280; font-weight: 500;">"Epochs"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || completed_runs.get().into_iter().take(20).map(|r| {
                                let id = r.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let arch = r.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let best_loss = r.get("best_val_loss").and_then(|v| v.as_f64())
                                    .map(|v| format!("{v:.6}")).unwrap_or_else(|| "--".into());
                                let duration = r.get("training_time_seconds").and_then(|v| v.as_u64())
                                    .map(format_duration).unwrap_or_else(|| "--".into());
                                let epochs = format!("{}/{}",
                                    r.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0),
                                    r.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(0));
                                let status_color = match status.as_str() {
                                    "completed" => "#22c55e",
                                    "failed" => "#ef4444",
                                    "cancelled" => "#f59e0b",
                                    _ => "#6b7280",
                                };
                                let id_click = id.clone();
                                view! {
                                    <tr
                                        style="border-bottom: 1px solid #f3f0ec; cursor: pointer; transition: background 0.1s;"
                                        on:click=move |_| selected_run.set(Some(id_click.clone()))
                                    >
                                        <td style="padding: 10px 12px;">
                                            <span style="color: #14b8a6; font-weight: 500;">{id}</span>
                                        </td>
                                        <td style="padding: 10px 12px; color: #374151;">{arch}</td>
                                        <td style="padding: 10px 12px;">
                                            <span style=format!("color: {status_color}; font-weight: 500;")>{status}</span>
                                        </td>
                                        <td style="padding: 10px 12px; color: #374151; font-family: monospace;">{best_loss}</td>
                                        <td style="padding: 10px 12px; color: #6b7280;">{duration}</td>
                                        <td style="padding: 10px 12px; color: #6b7280;">{epochs}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            </Show>

            // ── Empty State ──────────────────────────────────
            <Show when=move || training_runs.get().is_empty()>
                <div class="prometheus-card" style="padding: 48px; text-align: center;">
                    {icons::icon_activity()}
                    <h3 style="margin-top: 16px; color: #374151;">"No Training Runs"</h3>
                    <p style="color: #9ca3af; margin-top: 8px;">"Start a training job from the Datasets or Training page to see it here."</p>
                </div>
            </Show>
        </div>
    }
}

/// Individual run card for active/queued runs
#[component]
fn RunCard(
    run: serde_json::Value,
    selected_run: RwSignal<Option<String>>,
    training_runs: RwSignal<Vec<serde_json::Value>>,
) -> impl IntoView {
    let _ = &training_runs;
    let id = run.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let arch = run.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let status = run.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string();
    let current_epoch = run.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0);
    let total_epochs = run.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(100);
    let best_loss = run.get("best_val_loss").and_then(|v| v.as_f64());
    let dataset_id = run.get("dataset_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let started_at = run.get("started_at").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let pct = if total_epochs > 0 { current_epoch as f64 / total_epochs as f64 * 100.0 } else { 0.0 };
    let is_queued = status == "queued";
    let _is_selected = {
        let id = id.clone();
        move || selected_run.get().as_deref() == Some(&id)
    };

    let border_style = {
        let id = id.clone();
        move || {
            if selected_run.get().as_deref() == Some(&id) {
                "border-left: 4px solid #14b8a6; padding: 16px 20px; cursor: pointer; transition: all 0.15s;"
            } else {
                "border-left: 4px solid transparent; padding: 16px 20px; cursor: pointer; transition: all 0.15s;"
            }
        }
    };

    // Mini sparkline from epoch_metrics
    let sparkline_path = {
        let metrics = run.get("epoch_metrics").and_then(|m| m.as_array()).cloned().unwrap_or_default();
        if metrics.len() < 2 {
            String::new()
        } else {
            let losses: Vec<f64> = metrics.iter()
                .filter_map(|m| m.get("val_loss").and_then(|v| v.as_f64()))
                .collect();
            if losses.is_empty() {
                String::new()
            } else {
                let y_min = losses.iter().cloned().fold(f64::INFINITY, f64::min);
                let y_max = losses.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let y_range = if (y_max - y_min).abs() < 1e-10 { 1.0 } else { y_max - y_min };
                let w = 120.0;
                let h = 32.0;
                losses.iter().enumerate().map(|(i, &v)| {
                    let px = i as f64 / (losses.len() - 1).max(1) as f64 * w;
                    let py = h - ((v - y_min) / y_range * h);
                    if i == 0 { format!("M{px:.1},{py:.1}") } else { format!("L{px:.1},{py:.1}") }
                }).collect::<Vec<_>>().join(" ")
            }
        }
    };

    let id_click = id.clone();
    let id_stop = id.clone();

    view! {
        <div
            class="prometheus-card"
            style=border_style
            on:click=move |_| {
                if selected_run.get().as_deref() == Some(&id_click) {
                    selected_run.set(None);
                } else {
                    selected_run.set(Some(id_click.clone()));
                }
            }
        >
            <div style="display: flex; justify-content: space-between; align-items: flex-start;">
                // Left: run info
                <div style="flex: 1;">
                    <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 6px;">
                        <span style="font-weight: 600; color: #14b8a6;">{id.clone()}</span>
                        <Badge status=badge::status_to_badge(&status) />
                    </div>
                    <div style="display: flex; gap: 16px; font-size: 0.8rem; color: #6b7280;">
                        <span title="Neural network architecture">{arch}</span>
                        <span title="Source dataset ID">{dataset_id}</span>
                        <span title="Training start time">{format_time_ago(&started_at)}</span>
                    </div>
                </div>

                // Middle: sparkline (if running)
                <div style=if is_queued { "display:none;" } else { "display: flex; align-items: center; margin: 0 24px;" }>
                    <svg width="120" height="32" viewBox="0 0 120 32">
                        <path d=sparkline_path.clone() fill="none" stroke="#14b8a6" stroke-width="1.5" />
                    </svg>
                </div>

                // Right: metrics
                <div style="text-align: right; min-width: 120px;">
                    <div style=if is_queued { "display:none;" } else { "" }>
                        <div style="font-size: 0.75rem; color: #6b7280;" title="Epoch progress">{format!("Epoch {current_epoch}/{total_epochs}")}</div>
                        <div style="font-size: 1.1rem; font-weight: 600; color: #111827; font-family: monospace;"
                            title="Best validation loss — lower is better. This is the minimum loss on the held-out validation set across all epochs."
                        >
                            {best_loss.map(|v| format!("{v:.6}")).unwrap_or_else(|| "--".into())}
                        </div>
                        <div style="font-size: 0.7rem; color: #9ca3af;" title="Best validation loss (val_loss)">"val_loss"</div>
                    </div>
                    <div style=if is_queued { "" } else { "display:none;" }>
                        <div style="font-size: 0.85rem; color: #f59e0b; font-weight: 500;">"Waiting for slot..."</div>
                    </div>
                </div>
            </div>

            // Progress bar (running only)
            <div style=if is_queued { "display:none;" } else { "margin-top: 10px;" }>
                <div style="display: flex; justify-content: space-between; margin-bottom: 4px;">
                    <span style="font-size: 0.7rem; color: #9ca3af;">{format!("{pct:.0}%")}</span>
                    // Stop button
                    <button
                        class="btn btn-ghost btn-sm"
                        style="font-size: 0.75rem; color: #ef4444; padding: 2px 8px;"
                        title="Stop this training run"
                        on:click=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            let id = id_stop.clone();
                            leptos::task::spawn_local(async move {
                                let _ = crate::api::auth_post(&format!("/api/v1/training/{id}/stop"))
                                    .send()
                                    .await;
                            });
                        }
                    >
                        {icons::icon_stop()}
                        " Stop"
                    </button>
                </div>
                <div style="height: 4px; background: #f3f0ec; border-radius: 2px; overflow: hidden;">
                    <div style=format!(
                        "height: 100%; width: {pct:.0}%; background: #14b8a6; border-radius: 2px; transition: width 0.5s ease;"
                    )></div>
                </div>
            </div>
        </div>
    }
}

/// Expanded detail panel for a selected run with full charts
#[component]
fn RunDetailPanel(run: serde_json::Value) -> impl IntoView {
    let id = run.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let arch = run.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let _status = run.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let _current_epoch = run.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0);
    let _total_epochs = run.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(100);
    let best_loss = run.get("best_val_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let training_secs = run.get("training_time_seconds").and_then(|v| v.as_u64()).unwrap_or(0);
    let lr = run.get("hyperparameters").and_then(|h| h.get("learning_rate")).and_then(|v| v.as_f64()).unwrap_or(0.001);
    let batch_size = run.get("hyperparameters").and_then(|h| h.get("batch_size")).and_then(|v| v.as_u64()).unwrap_or(64);
    let hidden_dim = run.get("hyperparameters").and_then(|h| h.get("hidden_dim")).and_then(|v| v.as_u64()).unwrap_or(64);

    // Extract epoch metrics for charts
    let metrics = run.get("epoch_metrics").and_then(|m| m.as_array()).cloned().unwrap_or_default();
    let train_loss_data: Vec<DataPoint> = metrics.iter().enumerate().map(|(i, m)| {
        DataPoint {
            x: i as f64,
            y: m.get("train_loss").and_then(|v| v.as_f64()).unwrap_or(0.0),
        }
    }).collect();
    let val_loss_data: Vec<DataPoint> = metrics.iter().enumerate().map(|(i, m)| {
        DataPoint {
            x: i as f64,
            y: m.get("val_loss").and_then(|v| v.as_f64()).unwrap_or(0.0),
        }
    }).collect();

    // Compute convergence stats
    let last_train = metrics.last().and_then(|m| m.get("train_loss").and_then(|v| v.as_f64())).unwrap_or(0.0);
    let last_val = metrics.last().and_then(|m| m.get("val_loss").and_then(|v| v.as_f64())).unwrap_or(0.0);
    let overfit_gap = (last_val - last_train).abs();
    let overfit_pct = if last_train > 1e-10 { overfit_gap / last_train * 100.0 } else { 0.0 };

    // Loss improvement rate
    let first_val = metrics.first().and_then(|m| m.get("val_loss").and_then(|v| v.as_f64())).unwrap_or(last_val);
    let improvement = if first_val > 1e-10 { (first_val - best_loss) / first_val * 100.0 } else { 0.0 };

    let id_link = id.clone();

    view! {
        <div class="prometheus-card" style="margin-bottom: 32px; padding: 24px;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h3 style="font-size: 1.05rem; font-weight: 600; color: #111827; margin: 0;">
                    {format!("Run Detail: {id}")}
                </h3>
                <a
                    href=format!("/training/{id_link}")
                    style="font-size: 0.8rem; color: #14b8a6; text-decoration: none;"
                >
                    "Open full detail \u{2192}"
                </a>
            </div>

            // Metric pills
            <div style="display: flex; flex-wrap: wrap; gap: 12px; margin-bottom: 20px;">
                <MetricPill
                    label="Architecture"
                    value=arch.clone()
                    tooltip="Neural network architecture type"
                />
                <MetricPill
                    label="Best Val Loss"
                    value=format!("{best_loss:.6}")
                    tooltip="Minimum validation loss across all epochs — lower indicates better generalization"
                />
                <MetricPill
                    label="LR"
                    value=format!("{lr:.1e}")
                    tooltip="Learning rate — controls the step size during gradient descent optimization"
                />
                <MetricPill
                    label="Batch Size"
                    value=batch_size.to_string()
                    tooltip="Number of samples per gradient update — larger batches give smoother gradients but use more memory"
                />
                <MetricPill
                    label="Hidden Dim"
                    value=hidden_dim.to_string()
                    tooltip="Hidden layer dimension — number of neurons in each hidden layer of the model"
                />
                <MetricPill
                    label="Duration"
                    value=format_duration(training_secs)
                    tooltip="Total wall-clock training time"
                />
                <MetricPill
                    label="Overfit Gap"
                    value=format!("{overfit_pct:.1}%")
                    tooltip="Difference between train and val loss as % of train loss — high values suggest overfitting"
                />
                <MetricPill
                    label="Improvement"
                    value=format!("{improvement:.1}%")
                    tooltip="Total validation loss improvement from first to best epoch"
                />
            </div>

            // Loss charts side-by-side
            <div class="grid-2">
                <div>
                    <h4 style="font-size: 0.85rem; font-weight: 500; color: #374151; margin-bottom: 8px;"
                        title="Training loss — computed on the training data subset. Should decrease monotonically."
                    >
                        "Train Loss"
                        <span style="font-size: 0.7rem; color: #9ca3af; margin-left: 4px;" title="Mean Squared Error or Cross-Entropy depending on architecture">"(MSE/CE)"</span>
                    </h4>
                    <LineChart
                        data=Signal::derive(move || train_loss_data.clone())
                        color="#14b8a6"
                        x_label="Epoch"
                        y_label="Loss"
                    />
                </div>
                <div>
                    <h4 style="font-size: 0.85rem; font-weight: 500; color: #374151; margin-bottom: 8px;"
                        title="Validation loss — computed on held-out data the model never sees during training. The true measure of model quality."
                    >
                        "Val Loss"
                        <span style="font-size: 0.7rem; color: #9ca3af; margin-left: 4px;" title="Evaluated on the validation split (15% of data)">"(val split)"</span>
                    </h4>
                    <LineChart
                        data=Signal::derive(move || val_loss_data.clone())
                        color="#C2714F"
                        x_label="Epoch"
                        y_label="Loss"
                    />
                </div>
            </div>

            // Epoch Log Table (AxonML style)
            <div style="margin-top: 16px;">
                <h4 style="font-size: 0.85rem; font-weight: 500; color: #374151; margin-bottom: 8px;">"Epoch Log"</h4>
                <div style="max-height: 300px; overflow-y: auto;">
                    <table style="width:100%;border-collapse:collapse;font-family:'JetBrains Mono',monospace;font-size:0.78rem;">
                        <thead>
                            <tr style="border-bottom:2px solid #e8d4c4;color:#6b7280;text-align:left;">
                                <th style="padding:0.4rem">"Epoch"</th>
                                <th style="padding:0.4rem">"Train Loss"</th>
                                <th style="padding:0.4rem">"Val Loss"</th>
                                <th style="padding:0.4rem">"Best"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {metrics.iter().rev().take(20).map(|m| {
                                let epoch = m.get("epoch").and_then(|v| v.as_u64()).unwrap_or(0);
                                let tl = m.get("train_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let vl = m.get("val_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let is_best = (vl - best_loss).abs() < 1e-7;
                                view! {
                                    <tr style="border-bottom:1px solid rgba(232,212,196,0.3);">
                                        <td style="padding:0.3rem;color:#0d9488;font-weight:600">{epoch.to_string()}</td>
                                        <td style="padding:0.3rem">{format!("{tl:.6}")}</td>
                                        <td style="padding:0.3rem">{format!("{vl:.6}")}</td>
                                        <td style="padding:0.3rem;color:#c2714f;font-weight:600">{if is_best { "*" } else { "" }}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    }
}

/// Small metric pill with tooltip
#[component]
fn MetricPill(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(into)] tooltip: String,
) -> impl IntoView {
    view! {
        <div
            style="display: inline-flex; flex-direction: column; padding: 8px 14px; background: #FDFBF7; border: 1px solid #f3f0ec; border-radius: 8px; min-width: 80px;"
            title=tooltip
        >
            <span style="font-size: 0.65rem; color: #9ca3af; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 2px;">
                {label}
            </span>
            <span style="font-size: 0.9rem; font-weight: 600; color: #111827; font-family: monospace;">
                {value}
            </span>
        </div>
    }
}

async fn fetch_all(
    training_runs: RwSignal<Vec<serde_json::Value>>,
    queue_info: RwSignal<Option<serde_json::Value>>,
) {
    // Fetch training runs
    if let Ok(resp) = crate::api::auth_get("/api/v1/training").send().await {
        if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
            training_runs.set(data);
        }
    }
    // Fetch queue status
    if let Ok(resp) = crate::api::auth_get("/api/v1/training/queue").send().await {
        if let Ok(data) = resp.json::<serde_json::Value>().await {
            queue_info.set(Some(data));
        }
    }
}

fn format_duration(secs: u64) -> String {
    if secs == 0 {
        "--".into()
    } else if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

fn format_time_ago(iso: &str) -> String {
    if iso.is_empty() {
        return "--".into();
    }
    // Simple: just show the time portion
    if let Some(t_pos) = iso.find('T') {
        let time_part = &iso[t_pos + 1..];
        if let Some(dot_pos) = time_part.find('.') {
            return time_part[..dot_pos].to_string();
        }
        if time_part.len() >= 8 {
            return time_part[..8].to_string();
        }
    }
    iso.to_string()
}

fn event_target_checked(ev: &web_sys::Event) -> bool {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|el| el.checked())
        .unwrap_or(false)
}
