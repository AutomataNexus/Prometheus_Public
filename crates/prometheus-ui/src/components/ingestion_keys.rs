// ============================================================================
// File: ingestion_keys.rs
// Description: Ingestion API key management panel for creating and revoking keys
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use crate::components::toast::{ToastLevel, ToastMessage, push_toast};

#[derive(Clone, Debug)]
struct IngestionKey {
    id: String,
    name: String,
    prefix: String,
    created_at: String,
}

#[component]
pub fn IngestionKeysPanel() -> impl IntoView {
    let keys = RwSignal::new(Vec::<IngestionKey>::new());
    let loading = RwSignal::new(true);
    let creating = RwSignal::new(false);
    let new_key_name = RwSignal::new(String::new());
    let revealed_key = RwSignal::new(None::<String>);
    let show_create = RwSignal::new(false);

    // Fetch keys on mount
    {
        let keys = keys;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/ingestion-keys").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    keys.set(data.iter().filter_map(|d| {
                        Some(IngestionKey {
                            id: d.get("id")?.as_str()?.to_string(),
                            name: d.get("name")?.as_str()?.to_string(),
                            prefix: d.get("key_prefix").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            created_at: d.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        })
                    }).collect());
                }
            }
            loading.set(false);
        });
    }

    let on_create = move |_| {
        let name = new_key_name.get();
        if name.is_empty() {
            return;
        }
        creating.set(true);
        leptos::task::spawn_local(async move {
            let resp = crate::api::auth_post("/api/v1/ingestion-keys")
                .json(&serde_json::json!({ "name": name }))
                .unwrap()
                .send()
                .await;

            creating.set(false);

            match resp {
                Ok(r) if r.ok() => {
                    if let Ok(data) = r.json::<serde_json::Value>().await {
                        let raw = data.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let prefix = data.get("prefix").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let created = data.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string();

                        revealed_key.set(Some(raw));
                        keys.update(|list| list.push(IngestionKey {
                            id,
                            name: name.clone(),
                            prefix,
                            created_at: created,
                        }));
                        new_key_name.set(String::new());
                        show_create.set(false);

                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                            push_toast(set_toasts, ToastLevel::Success, "Ingestion key created");
                        }
                    }
                }
                Ok(r) => {
                    let text = r.text().await.unwrap_or_default();
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                        push_toast(set_toasts, ToastLevel::Error, format!("Failed to create key: {text}"));
                    }
                }
                Err(e) => {
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                        push_toast(set_toasts, ToastLevel::Error, format!("Network error: {e}"));
                    }
                }
            }
        });
    };

    let on_copy_key = move |_| {
        if let Some(key) = revealed_key.get() {
            let escaped = key.replace('\\', "\\\\").replace('\'', "\\'");
            let _ = js_sys::eval(&format!("navigator.clipboard.writeText('{escaped}')"));
            if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                push_toast(set_toasts, ToastLevel::Success, "Key copied to clipboard");
            }
        }
    };

    let on_dismiss_key = move |_| {
        revealed_key.set(None);
    };

    view! {
        <div style="border: 1px solid #E8D4C4; border-radius: 12px; background: #FFFDF7; padding: 20px;">
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px;">
                <div>
                    <div style="font-size: 1rem; font-weight: 700; color: #111827;">"Ingestion Keys"</div>
                    <div style="font-size: 0.8125rem; color: #6b7280; margin-top: 2px;">
                        "API keys for edge controllers and external sources to push data to Prometheus"
                    </div>
                </div>
                <button
                    class="btn btn-primary"
                    style="font-size: 0.8125rem; padding: 6px 14px;"
                    on:click=move |_| show_create.set(true)
                >
                    "+ New Key"
                </button>
            </div>

            // Revealed key banner (shown once after creation)
            <Show when=move || revealed_key.get().is_some() fallback=|| ()>
                <div style="background: rgba(20,184,166,0.08); border: 1px solid rgba(20,184,166,0.3); border-radius: 8px; padding: 14px; margin-bottom: 16px;">
                    <div style="font-size: 0.8125rem; font-weight: 600; color: #0f766e; margin-bottom: 6px;">
                        "Save this key now \u{2014} it won\u{2019}t be shown again"
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <code style="flex: 1; font-size: 0.8125rem; padding: 8px 10px; background: #fff; border: 1px solid #e5e7eb; border-radius: 6px; font-family: 'JetBrains Mono', monospace; word-break: break-all; color: #111827;">
                            {move || revealed_key.get().unwrap_or_default()}
                        </code>
                        <button
                            class="btn btn-ghost"
                            style="font-size: 0.75rem; padding: 6px 10px; flex-shrink: 0;"
                            on:click=on_copy_key
                        >
                            "Copy"
                        </button>
                        <button
                            class="btn btn-ghost"
                            style="font-size: 0.75rem; padding: 6px 10px; flex-shrink: 0;"
                            on:click=on_dismiss_key
                        >
                            "Dismiss"
                        </button>
                    </div>
                </div>
            </Show>

            // Create form
            <Show when=move || show_create.get() fallback=|| ()>
                <div style="display: flex; gap: 8px; margin-bottom: 16px;">
                    <input
                        type="text"
                        placeholder="Key name (e.g. Warren AHU-6)"
                        class="input"
                        style="flex: 1;"
                        prop:value=move || new_key_name.get()
                        on:input=move |ev| new_key_name.set(event_target_value(&ev))
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                let name = new_key_name.get();
                                if name.is_empty() || creating.get() {
                                    return;
                                }
                                creating.set(true);
                                leptos::task::spawn_local(async move {
                                    let resp = crate::api::auth_post("/api/v1/ingestion-keys")
                                        .json(&serde_json::json!({ "name": name }))
                                        .unwrap()
                                        .send()
                                        .await;
                                    creating.set(false);
                                    if let Ok(r) = resp {
                                        if r.ok() {
                                            if let Ok(data) = r.json::<serde_json::Value>().await {
                                                let raw = data.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let prefix = data.get("prefix").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let created = data.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                revealed_key.set(Some(raw));
                                                keys.update(|list| list.push(IngestionKey { id, name: name.clone(), prefix, created_at: created }));
                                                new_key_name.set(String::new());
                                                show_create.set(false);
                                                if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                                    push_toast(set_toasts, ToastLevel::Success, "Ingestion key created");
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    />
                    <button
                        class="btn btn-primary"
                        style="font-size: 0.8125rem; padding: 6px 14px; flex-shrink: 0;"
                        disabled=move || creating.get() || new_key_name.get().is_empty()
                        on:click=on_create
                    >
                        {move || if creating.get() { "Creating..." } else { "Create" }}
                    </button>
                    <button
                        class="btn btn-ghost"
                        style="font-size: 0.8125rem; padding: 6px 10px; flex-shrink: 0;"
                        on:click=move |_| { show_create.set(false); new_key_name.set(String::new()); }
                    >
                        "Cancel"
                    </button>
                </div>
            </Show>

            // Keys table
            <Show when=move || loading.get() fallback=|| ()>
                <div style="text-align: center; padding: 20px; color: #6b7280; font-size: 0.875rem;">
                    "Loading keys..."
                </div>
            </Show>

            <Show when=move || !loading.get() && keys.get().is_empty() fallback=|| ()>
                <div style="text-align: center; padding: 24px; color: #9ca3af; font-size: 0.875rem;">
                    "No ingestion keys yet. Create one to start streaming data from your controllers."
                </div>
            </Show>

            <Show when=move || !loading.get() && !keys.get().is_empty() fallback=|| ()>
                <div style="display: flex; flex-direction: column; gap: 1px;">
                    {move || keys.get().iter().map(|key| {
                        let key_id = key.id.clone();
                        let key_name = key.name.clone();
                        let key_prefix = key.prefix.clone();
                        let created = key.created_at.split('T').next().unwrap_or(&key.created_at).to_string();

                        let on_revoke = move |_| {
                            let kid = key_id.clone();
                            leptos::task::spawn_local(async move {
                                let resp = crate::api::auth_delete(&format!("/api/v1/ingestion-keys/{kid}"))
                                    .send()
                                    .await;
                                if let Ok(r) = resp {
                                    if r.ok() {
                                        keys.update(|list| list.retain(|k| k.id != kid));
                                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                            push_toast(set_toasts, ToastLevel::Success, "Ingestion key revoked");
                                        }
                                    }
                                }
                            });
                        };

                        view! {
                            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 0; border-bottom: 1px solid #f3eeea;">
                                <div style="display: flex; align-items: center; gap: 12px;">
                                    <div style="width: 32px; height: 32px; border-radius: 6px; background: rgba(20,184,166,0.1); display: flex; align-items: center; justify-content: center; flex-shrink: 0;">
                                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#14b8a6" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                            <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/>
                                        </svg>
                                    </div>
                                    <div>
                                        <div style="font-size: 0.875rem; font-weight: 600; color: #111827;">{key_name}</div>
                                        <div style="font-size: 0.75rem; color: #9ca3af; font-family: 'JetBrains Mono', monospace;">
                                            {format!("{}...", key_prefix)}
                                            " \u{00B7} "
                                            {created}
                                        </div>
                                    </div>
                                </div>
                                <button
                                    class="btn btn-ghost"
                                    style="font-size: 0.75rem; padding: 4px 10px; color: #ef4444;"
                                    on:click=on_revoke
                                >
                                    "Revoke"
                                </button>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </Show>

            // Usage hint
            <div style="margin-top: 16px; padding: 12px; background: #FAF8F5; border-radius: 8px; border: 1px solid #f0ebe6;">
                <div style="font-size: 0.75rem; font-weight: 600; color: #374151; margin-bottom: 4px;">"Usage"</div>
                <code style="font-size: 0.6875rem; color: #6b7280; font-family: 'JetBrains Mono', monospace; line-height: 1.6;">
                    "curl -X POST https://prometheus.automatanexus.com/api/v1/datasets \\"<br/>
                    "  -H \"Authorization: Bearer prom_your_key_here\" \\"<br/>
                    "  -F \"file=@data.csv\" -F \"name=My Dataset\""
                </code>
            </div>
        </div>
    }
}
