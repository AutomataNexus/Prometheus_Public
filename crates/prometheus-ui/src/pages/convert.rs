// ============================================================================
// File: convert.rs
// Description: Model format conversion page for ONNX and HEF export workflows
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
use crate::icons;

#[component]
pub fn ConvertPage() -> impl IntoView {
    let models = RwSignal::new(Vec::<serde_json::Value>::new());
    let selected_model = RwSignal::new(None::<String>);
    let converting_onnx = RwSignal::new(false);
    let converting_hef = RwSignal::new(false);
    let convert_error = RwSignal::new(None::<String>);
    let convert_success = RwSignal::new(None::<String>);
    let conversion_history = RwSignal::new(Vec::<ConversionEntry>::new());

    // Fetch models
    {
        let models = models;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/models").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    models.set(data);
                }
            }
        });
    }

    let do_convert = move |format: &'static str| {
        let model_id = match selected_model.get() {
            Some(id) => id,
            None => {
                convert_error.set(Some("Select a model first".into()));
                return;
            }
        };
        let is_onnx = format == "onnx";
        if is_onnx { converting_onnx.set(true); } else { converting_hef.set(true); }
        convert_error.set(None);
        convert_success.set(None);

        leptos::task::spawn_local(async move {
            let resp = crate::api::auth_post(&format!(
                "/api/v1/models/{model_id}/convert?format={format}"
            ))
            .send()
            .await;

            if is_onnx { converting_onnx.set(false); } else { converting_hef.set(false); }

            match resp {
                Ok(r) if r.status() == 200 => {
                    if let Ok(data) = r.json::<serde_json::Value>().await {
                        let label = format.to_uppercase();
                        let size = data.get("file_size").and_then(|v| v.as_u64()).unwrap_or(0);
                        let size_kb = size as f64 / 1024.0;
                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                            crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success,
                                &format!("{label} conversion complete ({size_kb:.1} KB)"));
                        }
                        // Add to history
                        conversion_history.update(|h| {
                            h.insert(0, ConversionEntry {
                                model_id: model_id.clone(),
                                format: label.clone(),
                                size_kb,
                                download_url: data.get("download_url")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            });
                        });
                        // No auto-redirect — user downloads from conversion history
                    }
                }
                Ok(r) => {
                    let status = r.status();
                    let text = r.text().await.unwrap_or_default();
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

    view! {
        <div>
            <h1 class="page-title">"Model Conversion"</h1>
            <p class="page-subtitle">"Convert trained models to ONNX and Hailo HEF formats for edge deployment"</p>

            // Error banner only (success uses toast)
            {move || convert_error.get().map(|msg| view! {
                <div style="background: rgba(239,68,68,0.08); border: 1px solid #ef4444; border-radius: 10px; padding: 14px 18px; margin-bottom: 20px; color: #ef4444; font-size: 14px;">
                    {msg}
                </div>
            })}

            // Model selector
            <Card title="Select Model" class="mb-8">
                <Show
                    when=move || !models.get().is_empty()
                    fallback=|| view! {
                        <p class="text-muted" style="padding: 24px; text-align: center;">
                            "No models available. Train a model first."
                        </p>
                    }
                >
                    <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px;">
                        {move || models.get().into_iter().filter(|m| {
                            let status = m.get("status").and_then(|v| v.as_str()).unwrap_or("");
                            status == "ready" || status == "completed"
                        }).map(|model| {
                            let id = model.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = model.get("name").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
                            let arch = model.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let file_size = model.get("file_size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                            let size_str = if file_size > 1_048_576 {
                                format!("{:.1} MB", file_size as f64 / 1_048_576.0)
                            } else {
                                format!("{:.1} KB", file_size as f64 / 1024.0)
                            };
                            let id_for_click = id.clone();
                            let id_for_check = id.clone();

                            view! {
                                <div
                                    style=move || {
                                        let is_selected = selected_model.get().as_deref() == Some(&id_for_check);
                                        if is_selected {
                                            "border: 2px solid #14b8a6; border-radius: 12px; padding: 16px; cursor: pointer; background: rgba(20,184,166,0.05); transition: all 0.15s ease;"
                                        } else {
                                            "border: 1px solid #e5e7eb; border-radius: 12px; padding: 16px; cursor: pointer; transition: all 0.15s ease;"
                                        }
                                    }
                                    on:click=move |_| selected_model.set(Some(id_for_click.clone()))
                                >
                                    <div style="display: flex; align-items: center; gap: 12px;">
                                        <div style="width: 36px; height: 36px; border-radius: 8px; background: #F5EDE8; display: flex; align-items: center; justify-content: center; color: #C4A484; flex-shrink: 0;">
                                            {icons::icon_brain()}
                                        </div>
                                        <div style="min-width: 0;">
                                            <div class="text-bold" style="font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">{name}</div>
                                            <div class="text-xs text-muted">{format!("{arch} \u{2022} {size_str}")}</div>
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </Show>
            </Card>

            // Conversion targets
            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin-bottom: 32px;">
                // ONNX card
                <Card class="">
                    <div style="display: flex; align-items: center; gap: 14px; margin-bottom: 16px;">
                        <div style="width: 48px; height: 48px; border-radius: 10px; background: rgba(20,184,166,0.1); display: flex; align-items: center; justify-content: center; color: #14b8a6; font-weight: 800; font-size: 13px;">
                            "ONNX"
                        </div>
                        <div>
                            <div class="text-bold" style="font-size: 16px;">"ONNX Format"</div>
                            <div class="text-xs text-muted">"Open Neural Network Exchange"</div>
                        </div>
                    </div>
                    <p class="text-sm text-muted" style="margin-bottom: 16px; line-height: 1.5;">
                        "Universal model interchange format. Compatible with ONNX Runtime, TensorRT, OpenVINO, CoreML, Hailo DFC, and dozens more inference engines."
                    </p>
                    <div style="margin-bottom: 16px; padding: 12px; border-radius: 8px; background: #f9fafb;">
                        <div class="text-xs text-muted" style="margin-bottom: 6px;">"Supported targets:"</div>
                        <div style="display: flex; flex-wrap: wrap; gap: 6px;">
                            <span style="font-size: 11px; padding: 2px 8px; border-radius: 4px; background: #e5e7eb; color: #374151;">"ONNX Runtime"</span>
                            <span style="font-size: 11px; padding: 2px 8px; border-radius: 4px; background: #e5e7eb; color: #374151;">"TensorRT"</span>
                            <span style="font-size: 11px; padding: 2px 8px; border-radius: 4px; background: #e5e7eb; color: #374151;">"OpenVINO"</span>
                            <span style="font-size: 11px; padding: 2px 8px; border-radius: 4px; background: #e5e7eb; color: #374151;">"CoreML"</span>
                            <span style="font-size: 11px; padding: 2px 8px; border-radius: 4px; background: #e5e7eb; color: #374151;">"Hailo DFC"</span>
                        </div>
                    </div>
                    <button
                        class="btn btn-primary"
                        style=move || if converting_onnx.get() || selected_model.get().is_none() {
                            "width: 100%; cursor: not-allowed; opacity: 0.5;"
                        } else {
                            "width: 100%; cursor: pointer;"
                        }
                        on:click=on_convert_onnx
                        disabled=move || converting_onnx.get() || selected_model.get().is_none()
                    >
                        {move || if converting_onnx.get() {
                            "Converting...".to_string()
                        } else {
                            "Convert to ONNX".to_string()
                        }}
                    </button>
                </Card>

                // HEF card
                <Card class="">
                    <div style="display: flex; align-items: center; gap: 14px; margin-bottom: 16px;">
                        <div style="width: 48px; height: 48px; border-radius: 10px; background: rgba(212,165,116,0.12); display: flex; align-items: center; justify-content: center; color: #D4A574; font-weight: 800; font-size: 14px;">
                            "HEF"
                        </div>
                        <div>
                            <div class="text-bold" style="font-size: 16px;">"Hailo HEF"</div>
                            <div class="text-xs text-muted">"Hailo Execution Format"</div>
                        </div>
                    </div>
                    <p class="text-sm text-muted" style="margin-bottom: 16px; line-height: 1.5;">
                        "Optimized binary for Hailo-8/8L AI accelerators on edge devices. Compiles ONNX through Hailo Dataflow Compiler for maximum inference throughput."
                    </p>
                    <div style="margin-bottom: 16px; padding: 12px; border-radius: 8px; background: #f9fafb;">
                        <div class="text-xs text-muted" style="margin-bottom: 6px;">"Requirements:"</div>
                        <div style="display: flex; flex-wrap: wrap; gap: 6px;">
                            <span style="font-size: 11px; padding: 2px 8px; border-radius: 4px; background: rgba(212,165,116,0.15); color: #A0785A;">"Hailo DFC SDK"</span>
                            <span style="font-size: 11px; padding: 2px 8px; border-radius: 4px; background: #e5e7eb; color: #374151;">"Hailo-8"</span>
                            <span style="font-size: 11px; padding: 2px 8px; border-radius: 4px; background: #e5e7eb; color: #374151;">"Hailo-8L"</span>
                        </div>
                    </div>
                    <button
                        class="btn"
                        style=move || if converting_hef.get() || selected_model.get().is_none() {
                            "width: 100%; background: rgba(212,165,116,0.15); border: 1px solid rgba(212,165,116,0.3); color: #A0785A; cursor: not-allowed; opacity: 0.5;"
                        } else {
                            "width: 100%; background: rgba(212,165,116,0.15); border: 1px solid rgba(212,165,116,0.3); color: #A0785A; cursor: pointer;"
                        }
                        on:click=on_convert_hef
                        disabled=move || converting_hef.get() || selected_model.get().is_none()
                    >
                        {move || if converting_hef.get() {
                            "Converting...".to_string()
                        } else {
                            "Convert to HEF".to_string()
                        }}
                    </button>
                </Card>
            </div>

            // Conversion history (session-only)
            <Show
                when=move || !conversion_history.get().is_empty()
                fallback=|| ()
            >
                <Card title="Recent Conversions" class="">
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        {move || conversion_history.get().into_iter().map(|entry| {
                            let url = entry.download_url.clone();
                            let format_label = entry.format.clone();
                            let model_label = entry.model_id.clone();
                            let size_label = format!("{:.1} KB", entry.size_kb);
                            let badge_style = if entry.format == "ONNX" {
                                "font-size: 11px; font-weight: 700; padding: 3px 8px; border-radius: 4px; background: rgba(20,184,166,0.1); color: #14b8a6;"
                            } else {
                                "font-size: 11px; font-weight: 700; padding: 3px 8px; border-radius: 4px; background: rgba(212,165,116,0.12); color: #D4A574;"
                            };
                            view! {
                                <div style="display: flex; justify-content: space-between; align-items: center; padding: 12px; border-radius: 8px; background: #f9fafb;">
                                    <div style="display: flex; align-items: center; gap: 12px;">
                                        <span style=badge_style>
                                            {format_label}
                                        </span>
                                        <span class="text-sm">{model_label}</span>
                                        <span class="text-xs text-muted">{size_label}</span>
                                    </div>
                                    <a
                                        href=url
                                        class="btn btn-sm btn-secondary"
                                        style="font-size: 12px; padding: 4px 12px;"
                                    >
                                        {icons::icon_download()}
                                        " Download"
                                    </a>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </Card>
            </Show>
        </div>
    }
}

#[derive(Clone)]
struct ConversionEntry {
    model_id: String,
    format: String,
    size_kb: f64,
    download_url: String,
}
