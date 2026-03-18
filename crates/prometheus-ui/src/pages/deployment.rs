// ============================================================================
// File: deployment.rs
// Description: Edge deployment management page for deploying models to targets
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
pub fn DeploymentPage() -> impl IntoView {
    let deployments = RwSignal::new(Vec::<serde_json::Value>::new());
    let targets = RwSignal::new(Vec::<serde_json::Value>::new());
    let models = RwSignal::new(Vec::<serde_json::Value>::new());
    let show_deploy = RwSignal::new(false);
    let show_add_target = RwSignal::new(false);
    let selected_model = RwSignal::new(String::new());
    let selected_target = RwSignal::new(String::new());

    // Custom controller fields
    let target_name = RwSignal::new(String::new());
    let target_ip = RwSignal::new(String::new());
    let target_username = RwSignal::new("devops".to_string());
    let target_port = RwSignal::new("22".to_string());
    let target_auth_method = RwSignal::new("password".to_string());
    let target_password = RwSignal::new(String::new());
    let target_save = RwSignal::new(true);

    {
        let deployments = deployments;
        let targets = targets;
        let models = models;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/deployments").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    deployments.set(data);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/deployments/targets").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    targets.set(data);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/models").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    models.set(data);
                }
            }
        });
    }

    let on_deploy = move |_| {
        let model_id = selected_model.get();
        let target_ip = selected_target.get();
        if model_id.is_empty() || target_ip.is_empty() {
            return;
        }
        show_deploy.set(false);
        leptos::task::spawn_local(async move {
            let _ = crate::api::auth_post("/api/v1/deployments")
                .json(&serde_json::json!({
                    "model_id": model_id,
                    "target_ip": target_ip,
                }))
                .unwrap()
                .send()
                .await;
            // Refresh
            if let Ok(resp) = crate::api::auth_get("/api/v1/deployments").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    deployments.set(data);
                }
            }
        });
    };

    let dep_table_columns = vec![
        Column { key: "id".into(), label: "ID".into(), sortable: true },
        Column { key: "model".into(), label: "Model".into(), sortable: false },
        Column { key: "target".into(), label: "Target".into(), sortable: false },
        Column { key: "status".into(), label: "Status".into(), sortable: true },
        Column { key: "deployed_at".into(), label: "Deployed".into(), sortable: true },
    ];

    let dep_table_rows: Signal<Vec<Vec<String>>> = Signal::derive(move || {
        deployments.get().iter().map(|d| {
            vec![
                d.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                d.get("model_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                d.get("target_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                d.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string(),
                d.get("deployed_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            ]
        }).collect()
    });

    view! {
        <div>
            <div class="flex-between mb-8">
                <div>
                    <h1 class="page-title">"Deployment"</h1>
                    <p class="page-subtitle">"Deploy models to Raspberry Pi edge controllers"</p>
                </div>
                <button class="btn btn-primary" on:click=move |_| show_deploy.set(true)>
                    {icons::icon_rocket()}
                    " Deploy Model"
                </button>
            </div>

            // Edge Targets
            <div class="flex-between mb-4">
                <h2 class="text-bold">"Edge Controllers"</h2>
                <button class="btn btn-sm" style="background:#F5EDE8;border:1px solid #E8D4C4;color:#374151;" on:click=move |_| show_add_target.set(true)>
                    {icons::icon_cpu()}
                    " Add Controller"
                </button>
            </div>
            <div class="grid-3 mb-8">
                {move || targets.get().into_iter().map(|target| {
                    let name = target.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let ip = target.get("ip").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let status = target.get("status").and_then(|v| v.as_str()).unwrap_or("offline").to_string();
                    let current_model = target.get("current_model").and_then(|v| v.as_str()).unwrap_or("None").to_string();
                    let badge_status = badge::status_to_badge(&status);

                    view! {
                        <Card>
                            <div class="flex-between mb-4">
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    {icons::icon_cpu()}
                                    <span class="text-bold">{name}</span>
                                </div>
                                <Badge status=badge_status />
                            </div>
                            <div class="text-sm text-muted mb-4">{ip}</div>
                            <div>
                                <span class="text-xs text-muted">"Current Model: "</span>
                                <span class="text-sm">{current_model}</span>
                            </div>
                        </Card>
                    }
                }).collect_view()}
            </div>

            // Deployment History
            <div class="prometheus-card" style="padding: 20px;">
                <h3 style="font-size: 1rem; font-weight: 600; color: #374151; margin-bottom: 16px;">"Deployment History"</h3>
                <DataTable
                    columns=dep_table_columns
                    rows=dep_table_rows
                    empty_message="No deployments yet."
                />
            </div>

            // Deploy Modal
            <Show when=move || show_deploy.get()>
                <div class="modal-backdrop" on:click=move |_| show_deploy.set(false)>
                    <div class="modal" on:click=move |ev| ev.stop_propagation()>
                        <h2 class="modal-title">"Deploy Model to Edge"</h2>
                        <div style="display: flex; flex-direction: column; gap: 16px; margin: 16px 0;">
                            <div class="input-group">
                                <label class="input-label">"Model"</label>
                                <select class="input-field" style="width: 100%;"
                                    on:change=move |ev| selected_model.set(event_target_value(&ev))>
                                    <option value="">"Select model..."</option>
                                    {move || models.get().into_iter().map(|m| {
                                        let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        view! { <option value=id.clone()>{name}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                            <div class="input-group">
                                <label class="input-label">"Target Controller"</label>
                                <select class="input-field" style="width: 100%;"
                                    on:change=move |ev| selected_target.set(event_target_value(&ev))>
                                    <option value="">"Select target..."</option>
                                    {move || targets.get().into_iter().map(|t| {
                                        let ip = t.get("ip").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        view! { <option value=ip.clone()>{format!("{name} ({ip})")}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                        </div>
                        <div class="modal-actions">
                            <button class="btn btn-ghost" on:click=move |_| show_deploy.set(false)>"Cancel"</button>
                            <button class="btn btn-primary" on:click=on_deploy>
                                {icons::icon_rocket()}
                                " Deploy"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>

            // Add Controller Modal
            <Show when=move || show_add_target.get()>
                <div class="modal-backdrop" on:click=move |_| show_add_target.set(false)>
                    <div class="modal" on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation() style="max-width: 520px;">
                        <h2 class="modal-title">"Add Edge Controller"</h2>
                        <div style="padding:8px 12px;border-radius:6px;background:rgba(20,184,166,0.08);border:1px solid rgba(20,184,166,0.2);margin-bottom:12px;">
                            <p style="font-size:0.8rem;color:#0d9488;margin:0;">
                                "Credentials are encrypted with AES-256-GCM via Shield credential vault. Prometheus cannot read your raw credentials."
                            </p>
                        </div>
                        <div style="display:grid;gap:12px;margin-bottom:16px;">
                            <div>
                                <label class="input-label">"Controller Name"</label>
                                <input class="input-field" type="text" placeholder="e.g. Warehouse Pi-4"
                                    prop:value=move || target_name.get()
                                    on:input=move |ev| target_name.set(event_target_value(&ev)) />
                            </div>
                            <div style="display:grid;grid-template-columns:2fr 1fr;gap:8px;">
                                <div>
                                    <label class="input-label">"IP Address"</label>
                                    <input class="input-field" type="text" placeholder="192.168.1.100 or Tailscale IP"
                                        prop:value=move || target_ip.get()
                                        on:input=move |ev| target_ip.set(event_target_value(&ev)) />
                                </div>
                                <div>
                                    <label class="input-label">"SSH Port"</label>
                                    <input class="input-field" type="number"
                                        prop:value=move || target_port.get()
                                        on:input=move |ev| target_port.set(event_target_value(&ev)) />
                                </div>
                            </div>
                            <div>
                                <label class="input-label">"Username"</label>
                                <input class="input-field" type="text" placeholder="devops"
                                    prop:value=move || target_username.get()
                                    on:input=move |ev| target_username.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="input-label">"Authentication"</label>
                                <select class="input-field" style="width:100%;"
                                    prop:value=move || target_auth_method.get()
                                    on:change=move |ev| target_auth_method.set(event_target_value(&ev))>
                                    <option value="password">"Password"</option>
                                    <option value="key">"SSH Key (paste)"</option>
                                </select>
                            </div>
                            <div>
                                <label class="input-label">{move || if target_auth_method.get() == "password" { "Password" } else { "SSH Private Key" }}</label>
                                {move || if target_auth_method.get() == "password" {
                                    view! {
                                        <input class="input-field" type="password" placeholder="Enter password"
                                            prop:value=move || target_password.get()
                                            on:input=move |ev| target_password.set(event_target_value(&ev)) />
                                    }.into_any()
                                } else {
                                    view! {
                                        <textarea class="input-field" style="min-height:80px;font-family:monospace;font-size:12px;"
                                            placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                                            prop:value=move || target_password.get()
                                            on:input=move |ev| target_password.set(event_target_value(&ev))>
                                        </textarea>
                                    }.into_any()
                                }}
                            </div>
                            <label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:0.85rem;color:#6b7280;">
                                <input type="checkbox" checked
                                    on:change=move |ev| {
                                        use wasm_bindgen::JsCast;
                                        let t = web_sys::EventTarget::from(ev.target().unwrap()).unchecked_into::<web_sys::HtmlInputElement>();
                                        target_save.set(t.checked());
                                    } />
                                "Save controller for future deployments"
                            </label>
                        </div>
                        <div class="modal-actions">
                            <button class="btn btn-ghost" on:click=move |_| show_add_target.set(false)>"Cancel"</button>
                            <button class="btn btn-primary" on:click=move |_| {
                                let name = target_name.get_untracked();
                                let ip = target_ip.get_untracked();
                                if name.is_empty() || ip.is_empty() { return; }
                                show_add_target.set(false);
                                let user = target_username.get_untracked();
                                let port: u64 = target_port.get_untracked().parse().unwrap_or(22);
                                let auth = target_auth_method.get_untracked();
                                let secret = target_password.get_untracked();
                                leptos::task::spawn_local(async move {
                                    let body = serde_json::json!({
                                        "name": name,
                                        "ip": ip,
                                        "port": port,
                                        "username": user,
                                        "auth_method": auth,
                                        "password": if auth == "password" { secret.clone() } else { String::new() },
                                        "ssh_key": if auth == "key" { secret.clone() } else { String::new() },
                                    });
                                    let _ = crate::api::auth_post("/api/v1/deployments/targets")
                                        .header("Content-Type", "application/json")
                                        .body(body.to_string())
                                        .unwrap()
                                        .send()
                                        .await;
                                    // Refresh targets
                                    if let Ok(resp) = crate::api::auth_get("/api/v1/deployments/targets").send().await {
                                        if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                                            targets.set(data);
                                        }
                                    }
                                });
                            }>
                                {icons::icon_check()}
                                " Add Controller"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
