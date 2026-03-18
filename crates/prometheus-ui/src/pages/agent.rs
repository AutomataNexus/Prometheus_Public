// ============================================================================
// File: agent.rs
// Description: Athena AI agent chat interface page with markdown rendering
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::{Show, For};
use wasm_bindgen::JsCast;
use crate::icons;
use crate::components::toast::{ToastLevel, ToastMessage, push_toast};

#[derive(Clone)]
struct ChatMessage {
    id: u32,
    role: String,
    content: String,
}

/// Convert basic markdown to HTML for chat display.
fn markdown_to_html(text: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;

    for line in text.lines() {
        if line.trim().starts_with("```") {
            if in_code_block {
                html.push_str("</code></pre>");
                in_code_block = false;
            } else {
                html.push_str("<pre style=\"background:#1e293b;color:#e2e8f0;padding:12px;border-radius:6px;overflow-x:auto;font-size:0.8rem;\"><code>");
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            html.push_str(&line.replace('<', "&lt;").replace('>', "&gt;"));
            html.push('\n');
            continue;
        }

        let mut processed = line.to_string();

        // Bold: **text**
        while let Some(start) = processed.find("**") {
            if let Some(end) = processed[start + 2..].find("**") {
                let before = &processed[..start];
                let bold = &processed[start + 2..start + 2 + end];
                let after = &processed[start + 2 + end + 2..];
                processed = format!("{before}<strong>{bold}</strong>{after}");
            } else {
                break;
            }
        }

        // Inline code: `text`
        while let Some(start) = processed.find('`') {
            if let Some(end) = processed[start + 1..].find('`') {
                let before = &processed[..start];
                let code = &processed[start + 1..start + 1 + end];
                let after = &processed[start + 1 + end + 1..];
                processed = format!("{before}<code style=\"background:#f1f5f9;padding:1px 4px;border-radius:3px;font-size:0.85em;\">{code}</code>{after}");
            } else {
                break;
            }
        }

        let trimmed = processed.trim();
        if trimmed.starts_with("### ") {
            html.push_str(&format!("<h4 style=\"margin:8px 0 4px;font-size:0.9rem;\">{}</h4>", &trimmed[4..]));
        } else if trimmed.starts_with("## ") {
            html.push_str(&format!("<h3 style=\"margin:8px 0 4px;font-size:0.95rem;\">{}</h3>", &trimmed[3..]));
        } else if trimmed.starts_with("# ") {
            html.push_str(&format!("<h2 style=\"margin:8px 0 4px;\">{}</h2>", &trimmed[2..]));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            html.push_str(&format!("<div style=\"padding-left:16px;margin:2px 0;\">\u{2022} {}</div>", &trimmed[2..]));
        } else if trimmed.len() > 2 && trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && trimmed.chars().nth(1) == Some('.') {
            html.push_str(&format!("<div style=\"padding-left:16px;margin:2px 0;\">{}</div>", trimmed));
        } else if trimmed.is_empty() {
            html.push_str("<div style=\"height:8px;\"></div>");
        } else {
            html.push_str(&format!("<div>{}</div>", processed));
        }
    }

    if in_code_block {
        html.push_str("</code></pre>");
    }

    html
}

#[component]
pub fn AgentPage() -> impl IntoView {
    let messages = RwSignal::new(Vec::<ChatMessage>::new());
    let input = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let next_id = RwSignal::new(1u32);

    // Dataset-driven recommendation state
    let dataset_id = RwSignal::new(None::<String>);
    let recommendations = RwSignal::new(None::<serde_json::Value>);
    let loading_recs = RwSignal::new(false);
    let selected_arch = RwSignal::new(None::<serde_json::Value>);
    let model_name = RwSignal::new(String::new());
    let creating_model = RwSignal::new(false);
    let show_chat = RwSignal::new(false);

    // Extract dataset_id from localStorage (set by Analyze button)
    {
        let mut found_ds = None;
        if let Some(window) = web_sys::window() {
            if let Some(storage) = window.local_storage().ok().flatten() {
                if let Ok(Some(ds_id)) = storage.get_item("prometheus_analyze_dataset") {
                    if !ds_id.is_empty() {
                        found_ds = Some(ds_id);
                        let _ = storage.remove_item("prometheus_analyze_dataset");
                    }
                }
            }
            // Fallback: check URL query param (for direct links / bookmarks)
            if found_ds.is_none() {
                if let Ok(search) = window.location().search() {
                    if let Some(ds) = search.strip_prefix("?dataset=")
                        .or_else(|| search.split("dataset=").nth(1))
                    {
                        let ds_id = ds.split('&').next().unwrap_or(ds).to_string();
                        if !ds_id.is_empty() {
                            found_ds = Some(ds_id);
                        }
                    }
                }
            }
        }

        if let Some(ds_id) = found_ds {
            dataset_id.set(Some(ds_id.clone()));

            // Fetch recommendations for this dataset
            loading_recs.set(true);
            leptos::task::spawn_local(async move {
                if let Ok(resp) = crate::api::auth_get(&format!("/api/v1/datasets/{ds_id}/recommend"))
                    .send()
                    .await
                {
                    if resp.ok() {
                        if let Ok(data) = resp.json::<serde_json::Value>().await {
                            recommendations.set(Some(data));
                        }
                    } else {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        web_sys::console::error_1(&format!("Recommend failed ({status}): {text}").into());
                    }
                }
                loading_recs.set(false);
            });
        }
    }

    // If no dataset param, go straight to chat
    if dataset_id.get_untracked().is_none() {
        show_chat.set(true);
        messages.set(vec![ChatMessage {
            id: 0,
            role: "assistant".to_string(),
            content: "Hello! I'm **PrometheusForge**, your AI engineering assistant. I can analyze datasets, recommend model architectures, and help you train and deploy ML models.\n\nTo get started, go to **Datasets**, select a validated dataset, and click **Analyze** — I'll recommend the best model architectures for your data.\n\nOr just ask me anything about ML, training, or deployment!".to_string(),
        }]);
    }

    let on_send = move |_| {
        let msg = input.get().trim().to_string();
        if msg.is_empty() || loading.get() {
            return;
        }

        let id = next_id.get();
        next_id.update(|n| *n += 1);
        messages.update(|m| m.push(ChatMessage { id, role: "user".to_string(), content: msg.clone() }));
        input.set(String::new());
        loading.set(true);

        let ds_id = dataset_id.get();

        leptos::task::spawn_local(async move {
            let mut body = serde_json::json!({ "message": msg });
            if let Some(ref ds) = ds_id {
                body["dataset_id"] = serde_json::json!(ds);
            }

            let result = crate::api::auth_post("/api/v1/agent/chat")
                .json(&body)
                .unwrap()
                .send()
                .await;

            loading.set(false);

            let response_text = match result {
                Ok(resp) if resp.ok() => {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        body.get("response").and_then(|r| r.as_str()).unwrap_or("I couldn't process that request.").to_string()
                    } else {
                        "Error parsing response.".to_string()
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    format!("Request failed (HTTP {status}). Please try again.")
                }
                Err(e) => format!("Connection error: {e}"),
            };

            let resp_id = next_id.get();
            next_id.update(|n| *n += 1);
            messages.update(|m| m.push(ChatMessage { id: resp_id, role: "assistant".to_string(), content: response_text }));
        });
    };

    let on_key = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            on_send(());
        }
    };

    view! {
        <div class="chat-container" style="display: flex; flex-direction: column; height: calc(100vh - 80px);">
            <div style="padding: 16px 24px; border-bottom: 1px solid #E8D4C4;">
                <div style="display: flex; align-items: center; gap: 12px;">
                    <div style="width: 40px; height: 40px; border-radius: 50%; background: linear-gradient(135deg, #14b8a6, #0d9488); display: flex; align-items: center; justify-content: center; color: white;">
                        {icons::icon_bot()}
                    </div>
                    <div>
                        <h2 style="font-size: 1rem; font-weight: 600;">"PrometheusForge"</h2>
                        <span class="text-xs text-muted">"AI Engineering Agent"</span>
                    </div>
                    // Toggle to chat mode
                    <div style="margin-left: auto;">
                        <button
                            class="btn btn-ghost btn-sm"
                            style="border: 1px solid #e2e8f0;"
                            on:click=move |_: web_sys::MouseEvent| {
                                show_chat.set(!show_chat.get());
                                if messages.get().is_empty() {
                                    messages.set(vec![ChatMessage {
                                        id: 0,
                                        role: "assistant".to_string(),
                                        content: "I'm **PrometheusForge**. Ask me anything about your data, model architectures, training, or deployment!".to_string(),
                                    }]);
                                }
                            }
                        >
                            {move || if show_chat.get() && recommendations.get().is_some() { "Back to Recommendations" } else { "Open Chat" }}
                        </button>
                    </div>
                </div>
            </div>

            // Main content area
            <div style="flex: 1; overflow-y: auto; padding: 0;">
                // Recommendation cards (when dataset is provided and not in chat mode)
                <div style=move || if !show_chat.get() && recommendations.get().is_some() { "" } else { "display:none;" }>
                    {move || {
                        let recs = recommendations.get();
                        if recs.is_none() { return view! { <div></div> }.into_any(); }
                        let recs = recs.unwrap();
                        let ds_name = recs.get("dataset_name").and_then(|v| v.as_str()).unwrap_or("Dataset").to_string();
                        let row_count = recs.get("row_count").and_then(|v| v.as_u64()).unwrap_or(0);
                        let feature_count = recs.get("feature_count").and_then(|v| v.as_u64()).unwrap_or(0);
                        let rec_list = recs.get("recommendations").and_then(|v| v.as_array()).cloned().unwrap_or_default();

                        view! {
                            <div style="padding: 24px;">
                                <div style="margin-bottom: 24px;">
                                    <h2 style="font-size: 1.25rem; font-weight: 700; margin-bottom: 8px;">
                                        {format!("Model Recommendations for \u{201C}{ds_name}\u{201D}")}
                                    </h2>
                                    <p class="text-sm text-muted">
                                        {format!("{row_count} rows \u{2022} {feature_count} features \u{2022} PrometheusForge analyzed your data and found {} suitable architectures", rec_list.len())}
                                    </p>
                                </div>

                                {rec_list.iter().map(|rec| {
                                    let ds_name = ds_name.clone();
                                    let arch = rec.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let name = rec.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let score = rec.get("match_score").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let desc = rec.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let use_case = rec.get("use_case").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let inference_result = rec.get("inference_result").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let inputs = rec.get("inputs").and_then(|v| v.as_array())
                                        .map(|a| a.iter().filter_map(|i| i.as_str()).collect::<Vec<_>>().join(", "))
                                        .unwrap_or_default();
                                    let outputs = rec.get("outputs").and_then(|v| v.as_array())
                                        .map(|a| a.iter().filter_map(|i| i.as_str()).collect::<Vec<_>>().join(", "))
                                        .unwrap_or_default();

                                    let score_color = if score >= 90 { "#059669" } else if score >= 75 { "#d97706" } else { "#6b7280" };
                                    let score_bg = if score >= 90 { "#ecfdf5" } else if score >= 75 { "#fffbeb" } else { "#f9fafb" };

                                    let rec_clone = rec.clone();

                                    view! {
                                        <div
                                            class="card"
                                            style="padding: 20px; margin-bottom: 16px; cursor: pointer; transition: box-shadow 0.15s, border-color 0.15s; border: 2px solid transparent;"
                                            on:mouseenter=|ev: web_sys::MouseEvent| {
                                                if let Some(target) = ev.current_target() {
                                                    if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                                                        let _ = el.style().set_property("border-color", "#14b8a6");
                                                        let _ = el.style().set_property("box-shadow", "0 4px 12px rgba(20,184,166,0.15)");
                                                    }
                                                }
                                            }
                                            on:mouseleave=|ev: web_sys::MouseEvent| {
                                                if let Some(target) = ev.current_target() {
                                                    if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                                                        let _ = el.style().set_property("border-color", "transparent");
                                                        let _ = el.style().set_property("box-shadow", "none");
                                                    }
                                                }
                                            }
                                            on:click=move |_: web_sys::MouseEvent| {
                                                selected_arch.set(Some(rec_clone.clone()));
                                                let default_name = format!("{} - {}", ds_name, name);
                                                model_name.set(default_name);
                                            }
                                        >
                                            <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 12px;">
                                                <div>
                                                    <div style="display: flex; align-items: center; gap: 10px; margin-bottom: 4px;">
                                                        <h3 style="font-size: 1.05rem; font-weight: 700; margin: 0;">{name.clone()}</h3>
                                                        <span style=format!("padding: 2px 10px; border-radius: 12px; font-size: 0.7rem; font-weight: 600; background: {score_bg}; color: {score_color};")>
                                                            {format!("{score}% match")}
                                                        </span>
                                                    </div>
                                                    <span style="padding: 2px 8px; border-radius: 4px; font-size: 0.7rem; font-weight: 500; background: #ede9fe; color: #7c3aed;">
                                                        {use_case}
                                                    </span>
                                                </div>
                                                <code style="font-size: 0.75rem; padding: 2px 8px; background: #f1f5f9; border-radius: 4px; color: #475569;">{arch}</code>
                                            </div>

                                            <p style="font-size: 0.875rem; color: #374151; margin-bottom: 12px; line-height: 1.5;">{desc}</p>

                                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px; font-size: 0.8rem;">
                                                <div>
                                                    <span class="text-xs text-muted">"Inputs"</span>
                                                    <div style="color: #111827; margin-top: 2px; word-break: break-word;">{inputs}</div>
                                                </div>
                                                <div>
                                                    <span class="text-xs text-muted">"Outputs"</span>
                                                    <div style="color: #111827; margin-top: 2px;">{outputs}</div>
                                                </div>
                                            </div>

                                            <div style="margin-top: 12px; padding: 10px 14px; background: #f8fafc; border-radius: 8px; border-left: 3px solid #14b8a6;">
                                                <span class="text-xs text-muted">"Inference Result"</span>
                                                <div style="font-size: 0.8rem; color: #111827; margin-top: 2px;">{inference_result}</div>
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </div>

                // Loading spinner for recommendations
                <div style=move || if loading_recs.get() { "" } else { "display:none;" }>
                    <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 80px;">
                        <img src="/assets/nexus-spinner.png" alt="Loading" style="width: 64px; height: 64px; animation: spin 2s linear infinite;" />
                        <p class="text-sm text-muted" style="margin-top: 16px;">"PrometheusForge is analyzing your dataset..."</p>
                        <style>"@keyframes spin { to { transform: rotate(360deg); } }"</style>
                    </div>
                </div>

                // Show error/fallback when recommendations failed to load
                <div style=move || if !show_chat.get() && !loading_recs.get() && dataset_id.get().is_some() && recommendations.get().is_none() {
                    ""
                } else { "display:none;" }>
                    <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 80px;">
                        <p style="font-size: 1.1rem; font-weight: 600; color: #374151; margin-bottom: 8px;">"Could not load recommendations"</p>
                        <p class="text-sm text-muted" style="margin-bottom: 16px;">"The dataset may not be validated yet, or the server could not analyze it. Check the browser console for details."</p>
                        <button class="btn btn-primary" on:click=move |_: web_sys::MouseEvent| {
                            // Retry
                            if let Some(ds_id) = dataset_id.get() {
                                loading_recs.set(true);
                                leptos::task::spawn_local(async move {
                                    if let Ok(resp) = crate::api::auth_get(&format!("/api/v1/datasets/{ds_id}/recommend")).send().await {
                                        if resp.ok() {
                                            if let Ok(data) = resp.json::<serde_json::Value>().await {
                                                recommendations.set(Some(data));
                                            }
                                        }
                                    }
                                    loading_recs.set(false);
                                });
                            }
                        }>"Retry"</button>
                    </div>
                </div>

                // Chat messages area
                <div class="chat-messages" style=move || if show_chat.get() || (dataset_id.get().is_none() && !loading_recs.get()) { "" } else { "display:none;" }>
                    <For
                        each=move || messages.get()
                        key=|m| m.id
                        children=|msg| {
                            let is_assistant = msg.role == "assistant";
                            let class = format!("chat-message {}", msg.role);
                            if is_assistant {
                                let html = markdown_to_html(&msg.content);
                                view! { <div class=class inner_html=html></div> }.into_any()
                            } else {
                                let content = msg.content.clone();
                                view! { <div class=class>{content}</div> }.into_any()
                            }
                        }
                    />
                    <Show when=move || loading.get()>
                        <div class="chat-message assistant" style="display: flex; align-items: center; gap: 8px;">
                            <img src="/assets/nexus-spinner.png" alt="" style="width: 20px; height: 20px; animation: spin 2s linear infinite;" />
                            " Thinking..."
                        </div>
                    </Show>
                </div>
            </div>

            // Model creation modal (overlay)
            <div style=move || if selected_arch.get().is_some() {
                "position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 100; display: flex; align-items: center; justify-content: center;"
            } else {
                "display: none;"
            }>
                {move || {
                    let arch = selected_arch.get();
                    if arch.is_none() { return view! { <div></div> }.into_any(); }
                    let arch = arch.unwrap();
                    let arch_name = arch.get("name").and_then(|v| v.as_str()).unwrap_or("Model").to_string();
                    let arch_key = arch.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let use_case = arch.get("use_case").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let hp = arch.get("hyperparameters").cloned().unwrap_or_else(|| serde_json::json!({}));
                    let ds_id_for_train = dataset_id.get().unwrap_or_default();

                    let epochs = hp.get("epochs").and_then(|v| v.as_u64()).unwrap_or(100);
                    let batch = hp.get("batch_size").and_then(|v| v.as_u64()).unwrap_or(64);
                    let lr = hp.get("learning_rate").and_then(|v| v.as_f64()).unwrap_or(0.001);
                    let hidden = hp.get("hidden_dim").and_then(|v| v.as_u64()).unwrap_or(64);

                    view! {
                        <div
                            class="card"
                            style="width: 520px; max-height: 90vh; overflow-y: auto; padding: 32px;"
                            on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
                        >
                            <h2 style="font-size: 1.2rem; font-weight: 700; margin-bottom: 4px;">"Create Model"</h2>
                            <p class="text-sm text-muted" style="margin-bottom: 20px;">
                                {format!("{arch_name} \u{2022} {use_case}")}
                            </p>

                            // Name input
                            <div style="margin-bottom: 16px;">
                                <label style="display: block; font-size: 0.8rem; font-weight: 600; margin-bottom: 4px; color: #374151;">"Model Name"</label>
                                <input
                                    type="text"
                                    class="input"
                                    style="width: 100%;"
                                    placeholder="Name your model..."
                                    prop:value=move || model_name.get()
                                    on:input=move |ev| model_name.set(event_target_value(&ev))
                                />
                            </div>

                            // Architecture summary
                            <div style="background: #f8fafc; border-radius: 8px; padding: 16px; margin-bottom: 16px;">
                                <div style="font-size: 0.8rem; font-weight: 600; margin-bottom: 8px; color: #374151;">"Training Configuration"</div>
                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;">
                                    <div><span class="text-muted">"Architecture: "</span><strong>{arch_key.clone()}</strong></div>
                                    <div><span class="text-muted">"Epochs: "</span><strong>{epochs.to_string()}</strong></div>
                                    <div><span class="text-muted">"Batch Size: "</span><strong>{batch.to_string()}</strong></div>
                                    <div><span class="text-muted">"Learning Rate: "</span><strong>{format!("{lr}")}</strong></div>
                                    <div><span class="text-muted">"Hidden Dim: "</span><strong>{hidden.to_string()}</strong></div>
                                </div>
                            </div>

                            // Creating spinner
                            <div style=move || if creating_model.get() {
                                "display: flex; flex-direction: column; align-items: center; padding: 24px;"
                            } else { "display: none;" }>
                                <img src="/assets/nexus-spinner.png" alt="Training" style="width: 48px; height: 48px; animation: spin 2s linear infinite;" />
                                <p class="text-sm" style="margin-top: 12px; color: #374151;">"Launching training..."</p>
                            </div>

                            // Buttons
                            <div style=move || if creating_model.get() { "display: none;" } else {
                                "display: flex; gap: 12px; justify-content: flex-end; margin-top: 8px;"
                            }>
                                <button
                                    class="btn btn-ghost"
                                    style="border: 1px solid #e2e8f0;"
                                    on:click=move |_: web_sys::MouseEvent| {
                                        selected_arch.set(None);
                                    }
                                >
                                    "Cancel"
                                </button>
                                <button
                                    class="btn btn-primary"
                                    disabled=move || model_name.get().trim().is_empty()
                                    on:click=move |_: web_sys::MouseEvent| {
                                        let name = model_name.get().trim().to_string();
                                        if name.is_empty() { return; }
                                        let ds_id = ds_id_for_train.clone();
                                        let arch = arch_key.clone();
                                        let hp = hp.clone();
                                        creating_model.set(true);

                                        leptos::task::spawn_local(async move {
                                            let body = serde_json::json!({
                                                "dataset_id": ds_id,
                                                "architecture": arch,
                                                "hyperparameters": hp,
                                            });
                                            match crate::api::auth_post("/api/v1/training/start")
                                                .header("Content-Type", "application/json")
                                                .body(body.to_string())
                                                .unwrap()
                                                .send()
                                                .await
                                            {
                                                Ok(resp) if resp.ok() => {
                                                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                                        push_toast(set_toasts, ToastLevel::Success, "Training started! Check the Training page for progress.");
                                                    }
                                                    // Navigate to training page
                                                    if let Some(w) = web_sys::window() {
                                                        let _ = w.location().set_href("/training");
                                                    }
                                                }
                                                Ok(resp) => {
                                                    let status = resp.status();
                                                    let body_text = resp.text().await.unwrap_or_default();
                                                    // Try to extract error message
                                                    let err_msg = serde_json::from_str::<serde_json::Value>(&body_text)
                                                        .ok()
                                                        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()))
                                                        .unwrap_or_else(|| format!("Training failed (HTTP {status})"));
                                                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                                        push_toast(set_toasts, ToastLevel::Error, &err_msg);
                                                    }
                                                    creating_model.set(false);
                                                }
                                                Err(e) => {
                                                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                                        push_toast(set_toasts, ToastLevel::Error, &format!("Connection error: {e}"));
                                                    }
                                                    creating_model.set(false);
                                                }
                                            }
                                        });
                                    }
                                >
                                    "Start Training"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>

            // Chat input area (always visible)
            <div class="chat-input-area">
                <textarea
                    class="chat-input"
                    rows="1"
                    placeholder="Ask PrometheusForge about your data, architectures, or training..."
                    prop:value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                    on:keydown=on_key
                    on:focus=move |_| {
                        if !show_chat.get() && recommendations.get().is_some() {
                            show_chat.set(true);
                            if messages.get().is_empty() {
                                messages.set(vec![ChatMessage {
                                    id: 0,
                                    role: "assistant".to_string(),
                                    content: "I'm **PrometheusForge**. I've already analyzed your dataset and shown recommendations above. Click **Back to Recommendations** to see them, or ask me anything!".to_string(),
                                }]);
                            }
                        }
                    }
                ></textarea>
                <button
                    class="btn btn-primary"
                    disabled=move || input.get().trim().is_empty() || loading.get()
                    on:click=move |_| on_send(())
                >
                    {icons::icon_send()}
                </button>
            </div>
        </div>
    }
}
