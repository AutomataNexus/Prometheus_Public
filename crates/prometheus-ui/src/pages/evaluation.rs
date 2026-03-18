// ============================================================================
// File: evaluation.rs
// Description: Model evaluation results page with metrics and comparison views
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use crate::components::*;

#[component]
pub fn EvaluationPage() -> impl IntoView {
    let evaluations = RwSignal::new(Vec::<serde_json::Value>::new());
    let selected_eval = RwSignal::new(None::<serde_json::Value>);
    let models = RwSignal::new(Vec::<serde_json::Value>::new());
    let evaluating = RwSignal::new(false);

    // Fetch evaluations and models on mount
    {
        let evaluations = evaluations;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/evaluations").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    if let Some(latest) = data.first().cloned() {
                        selected_eval.set(Some(latest));
                    }
                    evaluations.set(data);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/models").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    models.set(data);
                }
            }
        });
    }

    // Derive metrics from selected evaluation
    fn get_metric(eval: &Option<serde_json::Value>, key: &str, decimals: usize) -> String {
        eval.as_ref()
            .and_then(|e| e.get("model_metrics").or_else(|| e.get("gradient_metrics")))
            .and_then(|m| m.get(key).and_then(|v| v.as_f64()))
            .map(|v| format!("{v:.prec$}", prec = decimals))
            .unwrap_or_else(|| "--".into())
    }

    let accuracy = Signal::derive(move || get_metric(&selected_eval.get(), "accuracy", 3));
    let precision = Signal::derive(move || get_metric(&selected_eval.get(), "precision", 3));
    let recall = Signal::derive(move || get_metric(&selected_eval.get(), "recall", 3));
    let f1 = Signal::derive(move || get_metric(&selected_eval.get(), "f1", 3));
    let val_loss = Signal::derive(move || get_metric(&selected_eval.get(), "val_loss", 4));

    let quality_tier = Signal::derive(move || {
        let eval = selected_eval.get();
        eval.as_ref()
            .and_then(|e| e.get("assessment"))
            .and_then(|a| a.get("quality_tier"))
            .and_then(|v| v.as_str())
            .unwrap_or("--")
            .to_string()
    });

    let deploy_ready = Signal::derive(move || {
        let eval = selected_eval.get();
        eval.as_ref()
            .and_then(|e| e.get("assessment"))
            .and_then(|a| a.get("deploy_ready"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    });

    let recommendations = Signal::derive(move || {
        let eval = selected_eval.get();
        eval.as_ref()
            .and_then(|e| e.get("assessment"))
            .and_then(|a| a.get("recommendations"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
            .unwrap_or_default()
    });

    // Bar chart for metrics comparison
    let metric_labels = vec![
        "Precision".to_string(),
        "Recall".to_string(),
        "F1".to_string(),
    ];
    let metric_values = Signal::derive(move || {
        let eval = selected_eval.get();
        let metrics = eval.as_ref()
            .and_then(|e| e.get("model_metrics").or_else(|| e.get("gradient_metrics")));
        vec![
            metrics.and_then(|m| m.get("precision").and_then(|v| v.as_f64())).unwrap_or(0.0),
            metrics.and_then(|m| m.get("recall").and_then(|v| v.as_f64())).unwrap_or(0.0),
            metrics.and_then(|m| m.get("f1").and_then(|v| v.as_f64())).unwrap_or(0.0),
        ]
    });

    view! {
        <div>
            <h1 class="page-title">"Evaluation"</h1>
            <p class="page-subtitle">"Model evaluation metrics and performance analysis"</p>

            // Run Evaluation section
            <div class="prometheus-card" style="padding:16px 20px;margin-bottom:24px;">
                <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap;">
                    <span style="font-weight:600;font-size:0.9rem;color:#374151;">"Evaluate a Model"</span>
                    {move || models.get().into_iter().filter(|m| {
                        m.get("status").and_then(|v| v.as_str()).unwrap_or("") == "ready"
                    }).map(|m| {
                        let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("Model").to_string();
                        let short = if name.len() > 25 { format!("{}...", &name[..22]) } else { name };
                        let eval_id = id.clone();
                        view! {
                            <button
                                class="btn btn-sm"
                                style="background:#FAF8F5;border:1px solid #E8D4C4;color:#374151;font-size:0.75rem;"
                                disabled=move || evaluating.get()
                                on:click=move |_| {
                                    let mid = eval_id.clone();
                                    evaluating.set(true);
                                    leptos::task::spawn_local(async move {
                                        let _ = crate::api::auth_post(&format!("/api/v1/evaluations/{mid}/gradient"))
                                            .send().await;
                                        if let Ok(resp) = crate::api::auth_get("/api/v1/evaluations").send().await {
                                            if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                                                if let Some(latest) = data.first().cloned() {
                                                    selected_eval.set(Some(latest));
                                                }
                                                evaluations.set(data);
                                            }
                                        }
                                        evaluating.set(false);
                                    });
                                }
                            >
                                {short}
                            </button>
                        }
                    }).collect_view()}
                    {move || if evaluating.get() {
                        Some(view! { <span class="text-sm text-muted">"Evaluating..."</span> })
                    } else { None }}
                </div>
            </div>

            // Key Metrics
            <div class="metric-grid mb-8">
                <MetricCard label="Accuracy" value=accuracy tooltip="Percentage of all predictions that were correct. (TP + TN) / Total. Can be misleading on imbalanced data." />
                <MetricCard label="Precision" value=precision tooltip="Of all positive predictions, how many were correct. High precision = few false alarms." />
                <MetricCard label="Recall" value=recall tooltip="Of all actual positives, how many were found. High recall = few missed detections." />
                <MetricCard label="F1 Score" value=f1 tooltip="Harmonic mean of precision and recall. Best single metric for overall model quality. 1.0 = perfect." />
                <MetricCard label="Val Loss" value=val_loss tooltip="Loss computed on the validation set. Lower = better generalization. Compare to train loss to detect overfitting." />
                <MetricCard label="Quality" value=quality_tier tooltip="Overall quality assessment tier based on metrics thresholds. Excellent > Good > Fair > Poor." />
            </div>

            // Assessment
            <div class="grid-2 mb-8">
                <div class="prometheus-card" style="padding: 20px;">
                    <h3 style="font-size: 1rem; font-weight: 600; color: #374151; margin-bottom: 16px;">"Assessment"</h3>
                    <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 16px;">
                        <span class="text-sm">"Deploy Ready: "</span>
                        {move || {
                            if deploy_ready.get() {
                                view! { <Badge status=BadgeStatus::Online /> }.into_any()
                            } else {
                                view! { <Badge status=BadgeStatus::Offline /> }.into_any()
                            }
                        }}
                    </div>
                    <div>
                        <span class="text-xs text-muted">"Recommendations:"</span>
                        <ul style="margin-top: 8px; padding-left: 16px;">
                            {move || recommendations.get().into_iter().map(|rec| {
                                view! { <li class="text-sm" style="margin-bottom: 4px;">{rec}</li> }
                            }).collect_view()}
                        </ul>
                    </div>
                </div>

                // Metrics Bar Chart
                <div class="prometheus-card" style="padding: 20px;">
                    <h3 style="font-size: 1rem; font-weight: 600; color: #374151; margin-bottom: 16px;">"Metrics Overview"</h3>
                    <BarChart
                        labels=metric_labels.clone()
                        values=metric_values
                        height=200
                    />
                </div>
            </div>

            // Evaluation History
            <div class="prometheus-card" style="padding: 20px;">
                <h3 style="font-size: 1rem; font-weight: 600; color: #374151; margin-bottom: 16px;">"Evaluation History"</h3>
                <DataTable
                    columns=vec![
                        Column { key: "id".into(), label: "ID".into(), sortable: true },
                        Column { key: "model".into(), label: "Model".into(), sortable: false },
                        Column { key: "quality".into(), label: "Quality".into(), sortable: true },
                        Column { key: "source".into(), label: "Source".into(), sortable: true },
                    ]
                    rows=Signal::derive(move || {
                        evaluations.get().iter().map(|e| {
                            vec![
                                e.get("evaluation_id").or(e.get("id")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                e.get("model_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                e.get("assessment").and_then(|a| a.get("quality_tier")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                e.get("source").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            ]
                        }).collect()
                    })
                    empty_message="No evaluations yet. Run an evaluation from the Models page."
                />
            </div>
        </div>
    }
}
