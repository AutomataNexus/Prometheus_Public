// ============================================================================
// File: quantize.rs
// Description: Model quantization page — Q8_0, Q4_0, Q4_1, F16 via AxonML quant
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 16, 2026
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
pub fn QuantizePage() -> impl IntoView {
    let models = RwSignal::new(Vec::<serde_json::Value>::new());
    let selected_model = RwSignal::new(None::<serde_json::Value>);
    let selected_quant = RwSignal::new("q8_0".to_string());
    let quantizing = RwSignal::new(false);
    let progress = RwSignal::new(0u32);
    let error_msg = RwSignal::new(None::<String>);

    // Fetch models
    leptos::task::spawn_local(async move {
        if let Ok(resp) = crate::api::auth_get("/api/v1/models").send().await {
            if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                models.set(data);
            }
        }
    });

    // Pixelated progress animation
    let pixel_style = move || {
        if !quantizing.get() { return String::new(); }
        let p = progress.get();
        format!(
            "width:{}%;height:20px;border-radius:4px;transition:width 0.3s;\
             background:repeating-linear-gradient(\
             90deg,#FFFDF7 0px,#FFFDF7 2px,#FAF8F5 2px,#FAF8F5 4px,#F3F0EC 4px,#F3F0EC 6px\
             );background-size:6px 100%;animation:pixel-shift 0.8s linear infinite;",
            p
        )
    };

    let on_quantize = move |_| {
        let model = match selected_model.get() {
            Some(m) => m,
            None => return,
        };
        let model_id = model.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let quant_type = selected_quant.get_untracked();

        quantizing.set(true);
        progress.set(10);
        error_msg.set(None);

        leptos::task::spawn_local(async move {
            progress.set(30);

            let body = serde_json::json!({
                "model_id": model_id,
                "quant_type": quant_type,
            });

            match crate::api::auth_post("/api/v1/models/quantize")
                .header("Content-Type", "application/json")
                .body(body.to_string())
                .unwrap()
                .send()
                .await
            {
                Ok(resp) if resp.ok() => {
                    progress.set(100);
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let new_id = data.get("quantized_model_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let ratio = data.get("compression_ratio").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let orig_kb = data.get("original_size_bytes").and_then(|v| v.as_u64()).unwrap_or(0) as f64 / 1024.0;
                        let new_kb = data.get("quantized_size_bytes").and_then(|v| v.as_u64()).unwrap_or(0) as f64 / 1024.0;
                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                            crate::components::toast::push_toast(
                                set_toasts,
                                crate::components::toast::ToastLevel::Success,
                                &format!("Quantized {new_id} \u{2014} {:.1}x compression ({orig_kb:.1}KB \u{2192} {new_kb:.1}KB)", ratio),
                            );
                        }
                    } else {
                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                            crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success, "Quantization complete");
                        }
                    }
                    // Refresh models list
                    if let Ok(resp) = crate::api::auth_get("/api/v1/models").send().await {
                        if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                            models.set(data);
                        }
                    }
                }
                Ok(resp) => {
                    let msg = resp.text().await.unwrap_or_else(|_| "Quantization failed".into());
                    error_msg.set(Some(msg));
                }
                Err(e) => {
                    error_msg.set(Some(format!("Request failed: {e}")));
                }
            }
            quantizing.set(false);
        });
    };

    view! {
        <div>
            <h1 class="page-title">"Quantization"</h1>
            <p class="page-subtitle">"Compress models for edge deployment with AxonML quantization"</p>

            // Error banner only (success uses toast now)
            {move || error_msg.get().map(|msg| view! {
                <div style="background:rgba(239,68,68,0.08);border:1px solid #ef4444;border-radius:10px;padding:14px 18px;margin-bottom:20px;color:#ef4444;font-size:14px;">
                    {msg}
                </div>
            })}

            // Quantization options info
            <div class="grid-2 mb-8" style="grid-template-columns: 1fr 1fr 1fr 1fr;">
                <div class="prometheus-card" style="padding:16px;text-align:center;">
                    <div style="font-weight:700;color:#14b8a6;font-size:1.2rem;">"Q8_0"</div>
                    <div class="text-xs text-muted" style="margin:4px 0;">"8-bit symmetric"</div>
                    <div class="text-sm text-bold">"~3.8x"</div>
                    <InfoTip text="8-bit block quantization. Best accuracy/compression trade-off. Max error ~1%. Recommended for most use cases." />
                </div>
                <div class="prometheus-card" style="padding:16px;text-align:center;">
                    <div style="font-weight:700;color:#f59e0b;font-size:1.2rem;">"Q4_0"</div>
                    <div class="text-xs text-muted" style="margin:4px 0;">"4-bit symmetric"</div>
                    <div class="text-sm text-bold">"~7.1x"</div>
                    <InfoTip text="4-bit block quantization. Aggressive compression for memory-constrained edge. Max error ~20%. Good for large models." />
                </div>
                <div class="prometheus-card" style="padding:16px;text-align:center;">
                    <div style="font-weight:700;color:#8b5cf6;font-size:1.2rem;">"Q4_1"</div>
                    <div class="text-xs text-muted" style="margin:4px 0;">"4-bit with min"</div>
                    <div class="text-sm text-bold">"~6.5x"</div>
                    <InfoTip text="4-bit with min offset. Better accuracy than Q4_0 at slightly less compression. Preserves asymmetric weight distributions." />
                </div>
                <div class="prometheus-card" style="padding:16px;text-align:center;">
                    <div style="font-weight:700;color:#06b6d4;font-size:1.2rem;">"F16"</div>
                    <div class="text-xs text-muted" style="margin:4px 0;">"Half precision"</div>
                    <div class="text-sm text-bold">"~2x"</div>
                    <InfoTip text="16-bit float. Near-perfect accuracy (max error 0.06%). Smallest compression but safest for quality-critical inference." />
                </div>
            </div>

            // Model selector
            <div class="prometheus-card" style="padding:20px;margin-bottom:24px;">
                <h3 style="font-size:1rem;font-weight:600;color:#374151;margin-bottom:16px;">"Select Model"</h3>
                <Show
                    when=move || !models.get().is_empty()
                    fallback=|| view! {
                        <p class="text-muted" style="padding:24px;text-align:center;">"No models available. Train a model first."</p>
                    }
                >
                    <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:12px;">
                        {move || models.get().into_iter().filter(|m| {
                            m.get("status").and_then(|v| v.as_str()).unwrap_or("") == "ready"
                        }).map(|model| {
                            let id = model.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = model.get("name").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
                            let arch = model.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let size = model.get("file_size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                            let size_str = if size > 1_048_576 { format!("{:.1} MB", size as f64 / 1_048_576.0) }
                                else { format!("{:.1} KB", size as f64 / 1024.0) };
                            let id_check = id.clone();
                            let model_clone = model.clone();

                            view! {
                                <div
                                    style=move || {
                                        let sel = selected_model.get().as_ref().and_then(|m| m.get("id").and_then(|v| v.as_str()).map(String::from)).unwrap_or_default();
                                        if sel == id_check {
                                            "border:2px solid #14b8a6;border-radius:12px;padding:14px;cursor:pointer;background:rgba(20,184,166,0.05);"
                                        } else {
                                            "border:1px solid #e5e7eb;border-radius:12px;padding:14px;cursor:pointer;"
                                        }
                                    }
                                    on:click={
                                        let mc = model_clone.clone();
                                        move |_| selected_model.set(Some(mc.clone()))
                                    }
                                >
                                    <div style="display:flex;align-items:center;gap:10px;">
                                        <div style="width:32px;height:32px;border-radius:6px;background:#F5EDE8;display:flex;align-items:center;justify-content:center;color:#C4A484;">
                                            {icons::icon_brain()}
                                        </div>
                                        <div>
                                            <div class="text-bold" style="font-size:13px;">{name}</div>
                                            <div class="text-xs text-muted">{format!("{arch} \u{2022} {size_str}")}</div>
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </Show>
            </div>

            // Quant type selector + action
            <Show when=move || selected_model.get().is_some()>
                <div class="prometheus-card" style="padding:20px;margin-bottom:24px;">
                    <h3 style="font-size:1rem;font-weight:600;color:#374151;margin-bottom:16px;">"Quantization Type"</h3>
                    <div style="display:flex;gap:12px;margin-bottom:20px;">
                        {["q8_0", "q4_0", "q4_1", "f16"].into_iter().map(|qt| {
                            let qt_str = qt.to_string();
                            let qt_click = qt.to_string();
                            let label = match qt {
                                "q8_0" => "Q8_0 (3.8x)",
                                "q4_0" => "Q4_0 (7.1x)",
                                "q4_1" => "Q4_1 (6.5x)",
                                "f16" => "F16 (2x)",
                                _ => qt,
                            };
                            view! {
                                <button
                                    class="btn"
                                    style=move || {
                                        if selected_quant.get() == qt_str {
                                            "background:rgba(194,113,79,0.08);color:#0d9488;border:2px solid rgba(194,113,79,0.25);font-weight:600;"
                                        } else {
                                            "background:#FAF8F5;color:#9ca3af;border:1px solid #e5e7eb;"
                                        }
                                    }
                                    on:click=move |_| selected_quant.set(qt_click.clone())
                                >
                                    {label}
                                </button>
                            }
                        }).collect_view()}
                    </div>

                    // Progress bar (pixelated)
                    <div style=move || if quantizing.get() { "margin-bottom:16px;" } else { "display:none;" }>
                        <div style="display:flex;justify-content:space-between;margin-bottom:4px;">
                            <span class="text-xs text-muted">"Quantizing..."</span>
                            <span class="text-xs text-bold">{move || format!("{}%", progress.get())}</span>
                        </div>
                        <div style="height:20px;background:#f3f0ec;border-radius:4px;overflow:hidden;">
                            <div style=pixel_style></div>
                        </div>
                    </div>

                    <button
                        class="btn"
                        style="width:100%;background:#FAF8F5;border:1px solid #E8D4C4;color:#374151;font-weight:600;"
                        disabled=move || quantizing.get()
                        on:click=on_quantize
                    >
                        {move || if quantizing.get() {
                            "Quantizing...".to_string()
                        } else {
                            format!("Quantize to {}", selected_quant.get().to_uppercase())
                        }}
                    </button>
                </div>
            </Show>

            // CSS animation for pixelated progress
            <style>
                "@keyframes pixel-shift { 0% { background-position: 0 0; } 100% { background-position: 6px 0; } }"
            </style>
        </div>
    }
}
