// ============================================================================
// File: model_detail.rs
// Description: Model detail page with metadata, export options, and delete actions
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
use leptos_router::hooks::use_params_map;
use crate::components::*;
use crate::icons;

#[component]
pub fn ModelDetailPage() -> impl IntoView {
    let params = use_params_map();
    let model = RwSignal::new(None::<serde_json::Value>);
    let show_delete = RwSignal::new(false);
    let show_retrain = RwSignal::new(false);
    let editing_name = RwSignal::new(false);
    let edit_name_value = RwSignal::new(String::new());
    let retrain_epochs = RwSignal::new("100".to_string());
    let retrain_lr = RwSignal::new("0.001".to_string());
    let retrain_batch = RwSignal::new("64".to_string());
    let retrain_hidden = RwSignal::new("64".to_string());
    let retrain_seq = RwSignal::new("60".to_string());
    let retrain_layers = RwSignal::new("2".to_string());
    let converting_onnx = RwSignal::new(false);
    let converting_hef = RwSignal::new(false);
    let convert_error = RwSignal::new(None::<String>);
    let convert_success = RwSignal::new(None::<String>);

    {
        let model = model;
        leptos::task::spawn_local(async move {
            let model_id = params.get_untracked().get("id").unwrap_or_default();
            if let Ok(resp) = crate::api::auth_get(&format!("/api/v1/models/{model_id}"))
                .send()
                .await
            {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    model.set(Some(data));
                }
            }
        });
    }

    let on_download = move |_| {
        let model_id = params.get().get("id").unwrap_or_default();
        leptos::task::spawn_local(async move {
            use wasm_bindgen::JsCast;
            let token = crate::api::get_token().unwrap_or_default();
            let url = format!("/api/v1/models/{model_id}/download");

            let opts = web_sys::RequestInit::new();
            opts.set_method("GET");
            let headers = web_sys::Headers::new().unwrap();
            let _ = headers.set("Authorization", &format!("Bearer {token}"));
            opts.set_headers(&headers);

            let window = web_sys::window().unwrap();
            let resp_promise = window.fetch_with_str_and_init(&url, &opts);
            if let Ok(resp_val) = wasm_bindgen_futures::JsFuture::from(resp_promise).await {
                let resp: web_sys::Response = resp_val.unchecked_into();
                if resp.ok() {
                    if let Ok(blob_promise) = resp.blob() {
                        if let Ok(blob_val) = wasm_bindgen_futures::JsFuture::from(blob_promise).await {
                            let blob: web_sys::Blob = blob_val.unchecked_into();
                            if let Ok(obj_url) = web_sys::Url::create_object_url_with_blob(&blob) {
                                let document = window.document().unwrap();
                                let a: web_sys::HtmlAnchorElement = document.create_element("a").unwrap().unchecked_into();
                                a.set_href(&obj_url);
                                a.set_download(&format!("{model_id}.axonml"));
                                let _ = document.body().unwrap().append_child(&a);
                                a.click();
                                let _ = a.remove();
                                let _ = web_sys::Url::revoke_object_url(&obj_url);
                            }
                        }
                    }
                }
            }
        });
    };

    let do_convert = move |format: &'static str| {
        let model_id = params.get().get("id").unwrap_or_default();
        let is_onnx = format == "onnx";
        if is_onnx {
            converting_onnx.set(true);
        } else {
            converting_hef.set(true);
        }
        convert_error.set(None);
        convert_success.set(None);

        leptos::task::spawn_local(async move {
            let resp = crate::api::auth_post(&format!(
                "/api/v1/models/{model_id}/convert?format={format}"
            ))
            .send()
            .await;

            if is_onnx {
                converting_onnx.set(false);
            } else {
                converting_hef.set(false);
            }

            match resp {
                Ok(r) if r.status() == 200 => {
                    if let Ok(data) = r.json::<serde_json::Value>().await {
                        let label = format.to_uppercase();
                        let size = data.get("file_size").and_then(|v| v.as_u64()).unwrap_or(0);
                        let size_kb = size as f64 / 1024.0;
                        let msg = data.get("message").and_then(|v| v.as_str()).unwrap_or("");
                        let status_str = data.get("status").and_then(|v| v.as_str()).unwrap_or("converted");
                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                            if status_str == "partial" {
                                crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success,
                                    &format!("{label}: HAR created ({size_kb:.1} KB). {msg}"));
                            } else {
                                crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success,
                                    &format!("{label} conversion complete ({size_kb:.1} KB)"));
                            }
                        }
                    }
                }
                Ok(r) => {
                    let status = r.status();
                    let text = r.text().await.unwrap_or_default();
                    // Try to parse JSON error
                    let msg = serde_json::from_str::<serde_json::Value>(&text)
                        .ok()
                        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
                        .unwrap_or(text);
                    convert_error.set(Some(format!("Conversion failed (HTTP {status}): {msg}")));
                }
                Err(e) => {
                    convert_error.set(Some(format!("Request failed: {e}")));
                }
            }
        });
    };

    let on_convert_onnx = move |_| do_convert("onnx");
    let on_convert_hef = move |_| do_convert("hef");

    let on_delete = move |_| {
        let model_id = params.get().get("id").unwrap_or_default();
        show_delete.set(false);
        leptos::task::spawn_local(async move {
            let _ = crate::api::auth_delete(&format!("/api/v1/models/{model_id}"))
                .send()
                .await;
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_href("/models");
            }
        });
    };

    view! {
        <div>
            <Show
                when=move || model.get().is_some()
                fallback=|| view! { <PageLoader /> }
            >
                {move || {
                    let m = model.get().unwrap();
                    let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("Model").to_string();
                    let arch = m.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let domain = m.get("domain").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let params_count = m.get("parameters").and_then(|v| v.as_u64()).unwrap_or(0);
                    let input_features = m.get("input_features").and_then(|v| v.as_u64()).unwrap_or(0);
                    let hidden_dim = m.get("hidden_dim").and_then(|v| v.as_u64()).unwrap_or(0);
                    let bottleneck = m.get("bottleneck_dim").and_then(|v| v.as_u64()).unwrap_or(0);
                    let num_layers = m.get("num_layers").and_then(|v| v.as_u64()).unwrap_or(0);
                    let seq_len = m.get("sequence_length").and_then(|v| v.as_u64()).unwrap_or(0);
                    let quantized = m.get("quantized").and_then(|v| v.as_bool()).unwrap_or(false);
                    let model_id_str = m.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let dataset_id_str = m.get("dataset_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let arch_for_retrain = arch.clone();

                    let metrics = m.get("metrics").cloned().unwrap_or(serde_json::json!({}));
                    let precision = metrics.get("precision").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let recall = metrics.get("recall").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let f1 = metrics.get("f1").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let val_loss = metrics.get("val_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);

                    view! {
                        <div>
                            <div class="flex-between mb-8">
                                <div>
                                    <div style="display:flex;align-items:center;gap:8px;">
                                        {move || if editing_name.get() {
                                            let name_clone = edit_name_value.get_untracked();
                                            view! {
                                                <input
                                                    class="input-field"
                                                    type="text"
                                                    style="font-size:1.4rem;font-weight:700;padding:4px 8px;max-width:400px;"
                                                    prop:value=move || edit_name_value.get()
                                                    on:input=move |ev| edit_name_value.set(event_target_value(&ev))
                                                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                        if ev.key() == "Enter" {
                                                            editing_name.set(false);
                                                            let new_name = edit_name_value.get_untracked();
                                                            let mid = params.get_untracked().get("id").unwrap_or_default();
                                                            leptos::task::spawn_local(async move {
                                                                let _ = crate::api::auth_put(&format!("/api/v1/models/{mid}"))
                                                                    .header("Content-Type", "application/json")
                                                                    .body(serde_json::json!({"name": new_name}).to_string())
                                                                    .unwrap()
                                                                    .send()
                                                                    .await;
                                                            });
                                                        } else if ev.key() == "Escape" {
                                                            editing_name.set(false);
                                                        }
                                                    }
                                                    on:blur=move |_| {
                                                        editing_name.set(false);
                                                        let new_name = edit_name_value.get_untracked();
                                                        let mid = params.get_untracked().get("id").unwrap_or_default();
                                                        leptos::task::spawn_local(async move {
                                                            let _ = crate::api::auth_put(&format!("/api/v1/models/{mid}"))
                                                                .header("Content-Type", "application/json")
                                                                .body(serde_json::json!({"name": new_name}).to_string())
                                                                .unwrap()
                                                                .send()
                                                                .await;
                                                        });
                                                    }
                                                />
                                            }.into_any()
                                        } else {
                                            let n = name.clone();
                                            view! {
                                                <h1 class="page-title" style="cursor:pointer;" on:click=move |_| {
                                                    edit_name_value.set(n.clone());
                                                    editing_name.set(true);
                                                }>{name.clone()}</h1>
                                            }.into_any()
                                        }}
                                        <span
                                            style="cursor:pointer;color:#9ca3af;font-size:14px;"
                                            title="Click to rename"
                                            on:click=move |_| {
                                                if !editing_name.get() {
                                                    if let Some(ref m) = model.get() {
                                                        let n = m.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                        edit_name_value.set(n);
                                                    }
                                                    editing_name.set(true);
                                                }
                                            }
                                        >"\u{270E}"</span>
                                    </div>
                                    <p class="page-subtitle">{format!("{arch} \u{2022} {domain}")}</p>
                                </div>
                                <div style="display: flex; gap: 8px;">
                                    <button class="btn btn-primary" on:click=on_download>
                                        {icons::icon_download()}
                                        " Download .axonml"
                                    </button>
                                    <button
                                        class="btn btn-secondary"
                                        on:click=move |_| {
                                            // Pre-fill hyperparams from model metadata
                                            if let Some(ref m) = model.get() {
                                                retrain_hidden.set(m.get("hidden_dim").and_then(|v| v.as_u64()).unwrap_or(64).to_string());
                                                retrain_layers.set(m.get("num_layers").and_then(|v| v.as_u64()).unwrap_or(2).to_string());
                                                retrain_seq.set(m.get("sequence_length").and_then(|v| v.as_u64()).unwrap_or(60).to_string());
                                                retrain_batch.set(m.get("batch_size").and_then(|v| v.as_u64()).unwrap_or(64).to_string());
                                            }
                                            show_retrain.set(true);
                                        }
                                    >
                                        {icons::icon_refresh()}
                                        " Retrain"
                                    </button>
                                    <button
                                        class="btn btn-sm"
                                        style="background:#FAF8F5;border:1px solid #E8D4C4;color:#374151;"
                                        on:click=move |_| {
                                            let mid = params.get().get("id").unwrap_or_default();
                                            leptos::task::spawn_local(async move {
                                                match crate::api::auth_post(&format!("/api/v1/evaluations/{mid}/gradient"))
                                                    .send()
                                                    .await
                                                {
                                                    Ok(resp) if resp.ok() => {
                                                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                                                            crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success, "Evaluation complete. View results on the Evaluation page.");
                                                        }
                                                    }
                                                    _ => {
                                                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                                                            crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Error, "Evaluation failed");
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    >
                                        {icons::icon_chart()}
                                        " Evaluate"
                                    </button>
                                    <button class="btn btn-danger btn-sm" on:click=move |_| show_delete.set(true)>
                                        {icons::icon_trash()}
                                    </button>
                                </div>
                            </div>

                            // Metrics
                            <div class="metric-grid mb-8">
                                <MetricCard label="Precision" value=Signal::derive(move || format!("{precision:.3}")) tooltip="Of all positive predictions, how many were correct. Higher is better. 1.0 = no false positives." />
                                <MetricCard label="Recall" value=Signal::derive(move || format!("{recall:.3}")) tooltip="Of all actual positives, how many were found. Higher is better. 1.0 = no missed positives." />
                                <MetricCard label="F1 Score" value=Signal::derive(move || format!("{f1:.3}")) tooltip="Harmonic mean of precision and recall. Balances both. 1.0 = perfect, 0.0 = worst." />
                                <MetricCard label="Val Loss" value=Signal::derive(move || format!("{val_loss:.4}")) tooltip="Validation loss on held-out data the model never saw during training. Lower is better." />
                            </div>

                            // Architecture
                            <Card title="Architecture" class="mb-8">
                                <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px;">
                                    <div>
                                        <div style="display:flex;align-items:center;gap:4px;"><span class="text-xs text-muted">"Parameters"</span><InfoTip text="Total number of trainable weights in the model. More parameters = more capacity but slower training and larger file size." /></div>
                                        <div class="text-bold">{format!("{params_count}")}</div>
                                    </div>
                                    <div>
                                        <div style="display:flex;align-items:center;gap:4px;"><span class="text-xs text-muted">"Input Features"</span><InfoTip text="Number of input columns/features the model expects. Must match dataset column count (minus label/timestamp)." /></div>
                                        <div class="text-bold">{format!("{input_features}")}</div>
                                    </div>
                                    <div>
                                        <div style="display:flex;align-items:center;gap:4px;"><span class="text-xs text-muted">"Hidden Dim"</span><InfoTip text="Size of the hidden layers. Larger = more capacity to learn complex patterns, but risk of overfitting on small datasets." /></div>
                                        <div class="text-bold">{format!("{hidden_dim}")}</div>
                                    </div>
                                    <div>
                                        <div style="display:flex;align-items:center;gap:4px;"><span class="text-xs text-muted">"Bottleneck"</span><InfoTip text="Compressed representation size in autoencoder models. Smaller = more compression, forces model to learn essential patterns." /></div>
                                        <div class="text-bold">{format!("{bottleneck}")}</div>
                                    </div>
                                    <div>
                                        <div style="display:flex;align-items:center;gap:4px;"><span class="text-xs text-muted">"Layers"</span><InfoTip text="Number of stacked neural network layers. Deeper models learn more abstract features but are harder to train." /></div>
                                        <div class="text-bold">{format!("{num_layers}")}</div>
                                    </div>
                                    <div>
                                        <div style="display:flex;align-items:center;gap:4px;"><span class="text-xs text-muted">"Sequence Length"</span><InfoTip text="Number of time steps in each training window. Longer sequences capture longer-term dependencies but use more memory." /></div>
                                        <div class="text-bold">{format!("{seq_len}")}</div>
                                    </div>
                                </div>
                                <div class="mt-4">
                                    <Badge status=if quantized { BadgeStatus::Ready } else { BadgeStatus::Pending } />
                                    <span class="text-sm" style="margin-left: 8px;">
                                        {if quantized { "INT8 Quantized" } else { "Full Precision (f32)" }}
                                    </span>
                                </div>
                            </Card>

                            // Export & Conversion
                            <Card title="Export & Conversion" class="mb-8">
                                <p class="text-sm text-muted mb-4">
                                    "Convert your trained model to industry-standard formats for deployment on edge accelerators."
                                </p>

                                // Error banner only (success uses toast)
                                {move || convert_error.get().map(|msg| view! {
                                    <div style="background: rgba(239,68,68,0.1); border: 1px solid #ef4444; border-radius: 8px; padding: 12px 16px; margin-bottom: 16px; color: #ef4444; font-size: 14px;">
                                        {msg}
                                    </div>
                                })}

                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
                                    // ONNX
                                    <div style="border: 1px solid #374151; border-radius: 12px; padding: 20px;">
                                        <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 12px;">
                                            <div style="width: 40px; height: 40px; border-radius: 8px; background: rgba(20,184,166,0.1); display: flex; align-items: center; justify-content: center; color: #14b8a6; font-weight: 800; font-size: 11px;">
                                                "ONNX"
                                            </div>
                                            <div>
                                                <div class="text-bold" style="font-size: 15px;">"ONNX Format"</div>
                                                <div class="text-xs text-muted">"Open Neural Network Exchange"</div>
                                            </div>
                                        </div>
                                        <p class="text-xs text-muted mb-4">
                                            "Universal model format compatible with ONNX Runtime, TensorRT, OpenVINO, Hailo DFC, and more."
                                        </p>
                                        <button
                                            class="btn btn-primary"
                                            style="width: 100%;"
                                            on:click=on_convert_onnx
                                            disabled=move || converting_onnx.get()
                                        >
                                            {move || if converting_onnx.get() {
                                                "Converting...".to_string()
                                            } else {
                                                "Convert to ONNX".to_string()
                                            }}
                                        </button>
                                    </div>

                                    // HEF
                                    <div style="border: 1px solid #374151; border-radius: 12px; padding: 20px;">
                                        <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 12px;">
                                            <div style="width: 40px; height: 40px; border-radius: 8px; background: rgba(212,165,116,0.12); display: flex; align-items: center; justify-content: center; color: #D4A574; font-weight: 800; font-size: 12px;">
                                                "HEF"
                                            </div>
                                            <div>
                                                <div class="text-bold" style="font-size: 15px;">"Hailo HEF"</div>
                                                <div class="text-xs text-muted">"Hailo Execution Format"</div>
                                            </div>
                                        </div>
                                        <p class="text-xs text-muted mb-4">
                                            "Optimized binary for Hailo-8/8L AI accelerators. Requires Hailo DFC SDK on the server."
                                        </p>
                                        <button
                                            class="btn btn-secondary"
                                            style="width: 100%; border-color: #D4A574; color: #D4A574;"
                                            on:click=on_convert_hef
                                            disabled=move || converting_hef.get()
                                        >
                                            {move || if converting_hef.get() {
                                                "Converting...".to_string()
                                            } else {
                                                "Convert to HEF".to_string()
                                            }}
                                        </button>
                                    </div>
                                </div>
                            </Card>

                            <ConfirmModal
                                title="Delete Model"
                                message="Are you sure you want to delete this model? This action cannot be undone.".to_string()
                                show=show_delete.read_only()
                                on_confirm=Callback::new(on_delete)
                                on_cancel=Callback::new(move |_| show_delete.set(false))
                                confirm_text="Delete"
                                danger=true
                            />
                        </div>
                    }
                }}
            </Show>

            // Retrain Modal
            <Show when=move || show_retrain.get()>
                <div class="modal-backdrop" on:click=move |_| show_retrain.set(false)>
                    <div class="modal" on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation() style="max-width: 500px;">
                        <h2 class="modal-title">"Retrain from Checkpoint"</h2>
                        <p class="text-sm text-muted mb-4">"Resume training with existing weights. Adjust hyperparameters below."</p>
                        <div style="display:grid;gap:12px;margin-bottom:16px;">
                            <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;">
                                <div>
                                    <label class="input-label">"Epochs"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || retrain_epochs.get()
                                        on:input=move |ev| retrain_epochs.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Batch Size"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || retrain_batch.get()
                                        on:input=move |ev| retrain_batch.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Learning Rate"</label>
                                    <input class="input-field" type="number" step="0.0001"
                                        prop:value=move || retrain_lr.get()
                                        on:input=move |ev| retrain_lr.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Hidden Dim"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || retrain_hidden.get()
                                        on:input=move |ev| retrain_hidden.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Sequence Length"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || retrain_seq.get()
                                        on:input=move |ev| retrain_seq.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"Layers"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || retrain_layers.get()
                                        on:input=move |ev| retrain_layers.set(event_target_value(&ev)) />
                                </div>
                            </div>
                        </div>
                        <div class="modal-actions">
                            <button class="btn btn-ghost" on:click=move |_| show_retrain.set(false)>"Cancel"</button>
                            <button class="btn btn-primary" on:click=move |_| {
                                show_retrain.set(false);
                                if let Some(ref m) = model.get() {
                                    let mid = m.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let did = m.get("dataset_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let ar = m.get("architecture").and_then(|v| v.as_str()).unwrap_or("lstm_autoencoder").to_string();
                                    let epochs: u64 = retrain_epochs.get_untracked().parse().unwrap_or(100);
                                    let batch: u64 = retrain_batch.get_untracked().parse().unwrap_or(64);
                                    let lr: f64 = retrain_lr.get_untracked().parse().unwrap_or(0.001);
                                    let hidden: u64 = retrain_hidden.get_untracked().parse().unwrap_or(64);
                                    let seq_len: u64 = retrain_seq.get_untracked().parse().unwrap_or(60);
                                    let layers: u64 = retrain_layers.get_untracked().parse().unwrap_or(2);
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
                                        match crate::api::auth_post("/api/v1/training/start")
                                            .header("Content-Type", "application/json")
                                            .body(body.to_string())
                                            .unwrap()
                                            .send()
                                            .await
                                        {
                                            Ok(resp) if resp.ok() => {
                                                if let Some(window) = web_sys::window() {
                                                    let _ = window.location().set_href("/training");
                                                }
                                            }
                                            _ => {
                                                if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                                                    crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Error, "Retrain failed");
                                                }
                                            }
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
