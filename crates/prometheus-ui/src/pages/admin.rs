// ============================================================================
// File: admin.rs
// Description: Admin panel — user management, email ops, system overview
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
pub fn AdminPage() -> impl IntoView {
    let users = RwSignal::new(Vec::<serde_json::Value>::new());
    let system_metrics = RwSignal::new(None::<serde_json::Value>);
    let show_create_user = RwSignal::new(false);
    let new_username = RwSignal::new(String::new());
    let new_email = RwSignal::new(String::new());
    let new_role = RwSignal::new("operator".to_string());
    let new_password = RwSignal::new(String::new());

    // Fetch users and system metrics
    leptos::task::spawn_local(async move {
        if let Ok(resp) = crate::api::auth_get("/api/v1/admin/users").send().await {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = data.get("users").and_then(|v| v.as_array()) {
                    users.set(arr.clone());
                }
            }
        }
        if let Ok(resp) = crate::api::auth_get("/api/v1/system/metrics").send().await {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                system_metrics.set(Some(data));
            }
        }
    });

    let total_users = Signal::derive(move || users.get().len());
    let active_users = Signal::derive(move || {
        users.get().iter().filter(|u| {
            u.get("status").and_then(|v| v.as_str()).unwrap_or("") == "active"
        }).count()
    });
    let pending_users = Signal::derive(move || {
        users.get().iter().filter(|u| {
            u.get("status").and_then(|v| v.as_str()).unwrap_or("") == "pending"
        }).count()
    });

    let on_create_user = move |_| {
        let username = new_username.get_untracked();
        let email = new_email.get_untracked();
        let role = new_role.get_untracked();
        let password = new_password.get_untracked();
        if username.is_empty() || email.is_empty() || password.is_empty() { return; }
        show_create_user.set(false);
        leptos::task::spawn_local(async move {
            let body = serde_json::json!({
                "username": username,
                "email": email,
                "role": role,
                "password": password,
            });
            let _ = crate::api::auth_post("/api/v1/admin/users")
                .header("Content-Type", "application/json")
                .body(body.to_string())
                .unwrap()
                .send()
                .await;
            // Refresh
            if let Ok(resp) = crate::api::auth_get("/api/v1/admin/users").send().await {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(arr) = data.get("users").and_then(|v| v.as_array()) {
                        users.set(arr.clone());
                    }
                }
            }
            new_username.set(String::new());
            new_email.set(String::new());
            new_password.set(String::new());
        });
    };

    view! {
        <div>
            <div class="flex-between mb-8">
                <div>
                    <h1 class="page-title">"Admin Panel"</h1>
                    <p class="page-subtitle">"User management, email operations, and system overview"</p>
                </div>
                <button class="btn btn-primary" on:click=move |_| show_create_user.set(true)>
                    {icons::icon_user_plus()}
                    " Create User"
                </button>
            </div>

            // Overview metrics
            <div class="metric-grid mb-8">
                <MetricCard label="Total Users" value=Signal::derive(move || total_users.get().to_string()) tooltip="Total registered users across all roles and statuses." />
                <MetricCard label="Active" value=Signal::derive(move || active_users.get().to_string()) tooltip="Users with active accounts who can log in." />
                <MetricCard label="Pending" value=Signal::derive(move || pending_users.get().to_string()) tooltip="Users awaiting admin approval before they can access the platform." />
                <MetricCard label="CPU Usage" value=Signal::derive(move || {
                    system_metrics.get()
                        .and_then(|m| m.get("cpu_usage_percent").and_then(|v| v.as_f64()))
                        .map(|v| format!("{v:.1}%"))
                        .unwrap_or_else(|| "--".into())
                }) tooltip="Server CPU utilization. High sustained usage may indicate too many concurrent training jobs." />
                <MetricCard label="Memory" value=Signal::derive(move || {
                    system_metrics.get()
                        .and_then(|m| m.get("memory_used_mb").and_then(|v| v.as_f64()))
                        .map(|v| format!("{:.0} MB", v))
                        .unwrap_or_else(|| "--".into())
                }) tooltip="Server memory usage. Training jobs are the primary memory consumer." />
                <MetricCard label="Disk" value=Signal::derive(move || {
                    system_metrics.get()
                        .and_then(|m| m.get("disk_used_percent").and_then(|v| v.as_f64()))
                        .map(|v| format!("{v:.0}%"))
                        .unwrap_or_else(|| "--".into())
                }) tooltip="Disk usage on the data directory. Datasets and models consume the most space." />
            </div>

            // User table
            <div class="prometheus-card" style="padding:20px;margin-bottom:24px;">
                <h3 style="font-size:1rem;font-weight:600;color:#374151;margin-bottom:16px;">"Users"</h3>
                <div style="overflow-x:auto;">
                    <table style="width:100%;border-collapse:collapse;font-size:0.85rem;">
                        <thead>
                            <tr style="border-bottom:2px solid #E8D4C4;text-align:left;">
                                <th style="padding:10px 6px;color:#6b7280;font-weight:500;font-size:0.75rem;">"User"</th>
                                <th style="padding:10px 6px;color:#6b7280;font-weight:500;font-size:0.75rem;">"Role"</th>
                                <th style="padding:10px 6px;color:#6b7280;font-weight:500;font-size:0.75rem;">"Tier"</th>
                                <th style="padding:10px 6px;color:#6b7280;font-weight:500;font-size:0.75rem;">"Tokens"</th>
                                <th style="padding:10px 6px;color:#6b7280;font-weight:500;font-size:0.75rem;">"Data/Models"</th>
                                <th style="padding:10px 6px;color:#6b7280;font-weight:500;font-size:0.75rem;">"Storage"</th>
                                <th style="padding:10px 6px;color:#6b7280;font-weight:500;font-size:0.75rem;">"Security"</th>
                                <th style="padding:10px 6px;color:#6b7280;font-weight:500;font-size:0.75rem;">"Last Login"</th>
                                <th style="padding:10px 6px;color:#6b7280;font-weight:500;font-size:0.75rem;">"Actions"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || users.get().into_iter().map(|user| {
                                let username = user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let email = user.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let role = user.get("role").and_then(|v| v.as_str()).unwrap_or("operator").to_string();
                                let status = user.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let tier = user.get("tier").and_then(|v| v.as_str()).unwrap_or("free").to_string();
                                let tokens_used = user.get("tokens_used").and_then(|v| v.as_u64()).unwrap_or(0);
                                let tokens_limit = user.get("tokens_limit").and_then(|v| v.as_u64()).unwrap_or(1000);
                                let mfa_on = user.get("mfa_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                                let email_verified = user.get("email_verified").and_then(|v| v.as_bool()).unwrap_or(false);
                                let dataset_count = user.get("dataset_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                let model_count = user.get("model_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                let training_count = user.get("training_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                let active_training = user.get("active_training").and_then(|v| v.as_u64()).unwrap_or(0);
                                let storage_bytes = user.get("storage_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                                let storage_str = if storage_bytes > 1_073_741_824 {
                                    format!("{:.1} GB", storage_bytes as f64 / 1_073_741_824.0)
                                } else if storage_bytes > 1_048_576 {
                                    format!("{:.1} MB", storage_bytes as f64 / 1_048_576.0)
                                } else {
                                    format!("{:.0} KB", storage_bytes as f64 / 1024.0)
                                };
                                let last_login = user.get("last_login").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let last_login_short = if last_login.len() > 16 { last_login[..16].replace('T', " ") } else if last_login.len() > 10 { last_login[..10].to_string() } else { last_login.clone() };
                                let tier_bg = match tier.as_str() {
                                    "enterprise" => "rgba(139,92,246,0.1)",
                                    "pro" => "rgba(59,130,246,0.1)",
                                    "basic" => "rgba(20,184,166,0.1)",
                                    _ => "rgba(107,114,128,0.1)",
                                };
                                let tier_color = match tier.as_str() {
                                    "enterprise" => "#7c3aed",
                                    "pro" => "#2563eb",
                                    "basic" => "#0d9488",
                                    _ => "#6b7280",
                                };
                                let token_pct = if tokens_limit > 0 { (tokens_used as f64 / tokens_limit as f64 * 100.0).min(100.0) } else { 0.0 };
                                let token_color = if token_pct > 90.0 { "#ef4444" } else if token_pct > 70.0 { "#f59e0b" } else { "#14b8a6" };
                                let status_color = match status.as_str() {
                                    "active" => "#22c55e",
                                    "pending" => "#f59e0b",
                                    "disabled" => "#ef4444",
                                    _ => "#6b7280",
                                };
                                let role_bg = match role.as_str() {
                                    "admin" => "rgba(239,68,68,0.1)",
                                    "operator" => "rgba(20,184,166,0.1)",
                                    _ => "rgba(107,114,128,0.1)",
                                };
                                let role_color = match role.as_str() {
                                    "admin" => "#dc2626",
                                    "operator" => "#0d9488",
                                    _ => "#6b7280",
                                };

                                let uname_approve = username.clone();
                                let uname_disable = username.clone();
                                let uname_delete = username.clone();
                                let uname_reset = username.clone();
                                let email_reset = email.clone();
                                let is_pending = status != "active" && status != "disabled";
                                let is_active = status == "active";

                                view! {
                                    <tr style="border-bottom:1px solid #f3f0ec;">
                                        <td style="padding:8px 6px;">
                                            <div style="font-weight:500;color:#111827;font-size:0.85rem;">{username}</div>
                                            <div style="font-size:0.68rem;color:#9ca3af;">{email}</div>
                                            <span style=format!("color:{status_color};font-size:0.68rem;font-weight:500;")>{status}</span>
                                        </td>
                                        <td style="padding:8px 6px;">
                                            <span style=format!("padding:2px 6px;border-radius:4px;font-size:0.68rem;font-weight:500;background:{role_bg};color:{role_color};")>{role}</span>
                                        </td>
                                        <td style="padding:8px 6px;">
                                            <span style=format!("padding:2px 6px;border-radius:4px;font-size:0.68rem;font-weight:500;background:{tier_bg};color:{tier_color};")>{tier}</span>
                                        </td>
                                        <td style="padding:8px 6px;">
                                            <div style="font-size:0.72rem;color:#374151;font-family:monospace;">{format!("{tokens_used}/{tokens_limit}")}</div>
                                            <div style="height:3px;width:50px;background:#f3f0ec;border-radius:2px;margin-top:2px;">
                                                <div style=format!("height:100%;width:{token_pct:.0}%;background:{token_color};border-radius:2px;")></div>
                                            </div>
                                        </td>
                                        <td style="padding:8px 6px;font-size:0.72rem;">
                                            <div style="color:#374151;">{format!("{dataset_count} datasets")}</div>
                                            <div style="color:#0d9488;">{format!("{model_count} models")}</div>
                                            {if active_training > 0 {
                                                format!("{active_training} training")
                                            } else {
                                                format!("{training_count} runs")
                                            }}
                                        </td>
                                        <td style="padding:8px 6px;font-size:0.75rem;color:#374151;font-family:monospace;">
                                            {storage_str}
                                        </td>
                                        <td style="padding:8px 6px;font-size:0.72rem;">
                                            <div>{if mfa_on { "MFA \u{2705}" } else { "MFA \u{274C}" }}</div>
                                            <div>{if email_verified { "Email \u{2705}" } else { "Email \u{274C}" }}</div>
                                        </td>
                                        <td style="padding:8px 6px;color:#9ca3af;font-size:0.7rem;">{last_login_short}</td>
                                        <td style="padding:8px 6px;">
                                            <div style="display:flex;gap:4px;">
                                                {if is_pending { Some(view! {
                                                    <button
                                                        class="btn btn-sm"
                                                        style="font-size:0.7rem;padding:2px 8px;background:rgba(34,197,94,0.1);color:#22c55e;border:1px solid rgba(34,197,94,0.3);"
                                                        on:click=move |_| {
                                                            let u = uname_approve.clone();
                                                            leptos::task::spawn_local(async move {
                                                                let _ = crate::api::auth_post(&format!("/api/v1/admin/users/{u}/approve"))
                                                                    .send().await;
                                                                if let Ok(resp) = crate::api::auth_get("/api/v1/admin/users").send().await {
                                                                    if let Ok(data) = resp.json::<serde_json::Value>().await { if let Some(arr) = data.get("users").and_then(|v| v.as_array()) { users.set(arr.clone()); } }
                                                                }
                                                            });
                                                        }
                                                    >"Approve"</button>
                                                })} else { None }}
                                                {if is_active { Some(view! {
                                                    <button
                                                        class="btn btn-sm"
                                                        style="font-size:0.7rem;padding:2px 8px;background:rgba(249,115,22,0.1);color:#f59e0b;border:1px solid rgba(249,115,22,0.3);"
                                                        on:click=move |_| {
                                                            let u = uname_disable.clone();
                                                            leptos::task::spawn_local(async move {
                                                                let _ = crate::api::auth_put(&format!("/api/v1/admin/users/{u}"))
                                                                    .header("Content-Type", "application/json")
                                                                    .body(serde_json::json!({"status": "disabled"}).to_string())
                                                                    .unwrap()
                                                                    .send().await;
                                                                if let Ok(resp) = crate::api::auth_get("/api/v1/admin/users").send().await {
                                                                    if let Ok(data) = resp.json::<serde_json::Value>().await { if let Some(arr) = data.get("users").and_then(|v| v.as_array()) { users.set(arr.clone()); } }
                                                                }
                                                            });
                                                        }
                                                    >"Disable"</button>
                                                })} else { None }}
                                                <button
                                                    class="btn btn-sm"
                                                    style="font-size:0.7rem;padding:2px 8px;background:rgba(59,130,246,0.1);color:#3b82f6;border:1px solid rgba(59,130,246,0.3);"
                                                    title="Send password reset email"
                                                    on:click=move |_| {
                                                        let e = email_reset.clone();
                                                        leptos::task::spawn_local(async move {
                                                            let _ = crate::api::auth_post("/api/v1/email/password-reset")
                                                                .header("Content-Type", "application/json")
                                                                .body(serde_json::json!({"email": e}).to_string())
                                                                .unwrap()
                                                                .send().await;
                                                            if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                                                                crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success, "Password reset email sent");
                                                            }
                                                        });
                                                    }
                                                >"Reset PW"</button>
                                                <button
                                                    class="btn btn-sm"
                                                    style="font-size:0.7rem;padding:2px 8px;background:rgba(239,68,68,0.1);color:#ef4444;border:1px solid rgba(239,68,68,0.3);"
                                                    on:click=move |_| {
                                                        let u = uname_delete.clone();
                                                        leptos::task::spawn_local(async move {
                                                            let _ = crate::api::auth_delete(&format!("/api/v1/admin/users/{u}"))
                                                                .send().await;
                                                            if let Ok(resp) = crate::api::auth_get("/api/v1/admin/users").send().await {
                                                                if let Ok(data) = resp.json::<serde_json::Value>().await { if let Some(arr) = data.get("users").and_then(|v| v.as_array()) { users.set(arr.clone()); } }
                                                            }
                                                        });
                                                    }
                                                >"Delete"</button>
                                            </div>
                                        </td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>

            // Email Operations
            <div class="prometheus-card" style="padding:20px;margin-bottom:24px;">
                <h3 style="font-size:1rem;font-weight:600;color:#374151;margin-bottom:16px;">"Email Operations"</h3>
                <div style="display:flex;gap:12px;flex-wrap:wrap;">
                    <button class="btn" style="background:#FAF8F5;border:1px solid #E8D4C4;color:#374151;"
                        on:click=move |_| {
                            leptos::task::spawn_local(async move {
                                let _ = crate::api::auth_post("/api/v1/email/daily-report")
                                    .send().await;
                                if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                                    crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success, "Daily report email sent");
                                }
                            });
                        }
                    >
                        {icons::icon_mail()}
                        " Send Daily Report"
                    </button>
                    <button class="btn" style="background:#FAF8F5;border:1px solid #E8D4C4;color:#374151;"
                        on:click=move |_| {
                            leptos::task::spawn_local(async move {
                                let _ = crate::api::auth_post("/api/v1/email/security-alert")
                                    .header("Content-Type", "application/json")
                                    .body(serde_json::json!({"alert_type": "system_check", "details": "Manual security check initiated by admin"}).to_string())
                                    .unwrap()
                                    .send().await;
                                if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                                    crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success, "Security alert email sent");
                                }
                            });
                        }
                    >
                        {icons::icon_shield()}
                        " Security Alert"
                    </button>
                </div>
            </div>

            // System Info
            <div class="prometheus-card" style="padding:20px;">
                <h3 style="font-size:1rem;font-weight:600;color:#374151;margin-bottom:16px;">"System"</h3>
                <div style="display:grid;grid-template-columns:1fr 1fr 1fr 1fr;gap:16px;">
                    <div>
                        <span class="text-xs text-muted">"Prometheus"</span>
                        <div class="text-sm text-bold">"v0.1.0"</div>
                    </div>
                    <div>
                        <span class="text-xs text-muted">"AxonML"</span>
                        <div class="text-sm text-bold">"v0.4.1"</div>
                    </div>
                    <div>
                        <span class="text-xs text-muted">"Aegis-DB"</span>
                        <div class="text-sm text-bold">"v0.2.2"</div>
                    </div>
                    <div>
                        <span class="text-xs text-muted">"Shield"</span>
                        <div class="text-sm text-bold">"AES-256-GCM"</div>
                    </div>
                </div>
                <div style="margin-top:12px;padding:8px 12px;border-radius:6px;background:rgba(20,184,166,0.05);border:1px solid rgba(20,184,166,0.15);">
                    <p style="font-size:0.8rem;color:#0d9488;margin:0;">
                        "All user passwords are hashed with Argon2id via Aegis-DB. Credentials and API keys are encrypted with AES-256-GCM. Admins cannot view raw passwords or secrets."
                    </p>
                </div>
            </div>

            // Create User Modal
            <Show when=move || show_create_user.get()>
                <div class="modal-backdrop" on:click=move |_| show_create_user.set(false)>
                    <div class="modal" on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation() style="max-width:450px;">
                        <h2 class="modal-title">"Create User"</h2>
                        <div style="display:grid;gap:12px;margin-bottom:16px;">
                            <div>
                                <label class="input-label">"Username"</label>
                                <input class="input-field" type="text" placeholder="john_doe"
                                    prop:value=move || new_username.get()
                                    on:input=move |ev| new_username.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="input-label">"Email"</label>
                                <input class="input-field" type="email" placeholder="john@example.com"
                                    prop:value=move || new_email.get()
                                    on:input=move |ev| new_email.set(event_target_value(&ev)) />
                            </div>
                            <div>
                                <label class="input-label">"Password"</label>
                                <input class="input-field" type="password" placeholder="Minimum 8 characters"
                                    prop:value=move || new_password.get()
                                    on:input=move |ev| new_password.set(event_target_value(&ev)) />
                                <p style="font-size:0.7rem;color:#9ca3af;margin-top:4px;">"Password will be hashed with Argon2id. You will not be able to see it after creation."</p>
                            </div>
                            <div>
                                <label class="input-label">"Role"</label>
                                <select class="input-field" style="width:100%;"
                                    prop:value=move || new_role.get()
                                    on:change=move |ev| new_role.set(event_target_value(&ev))>
                                    <option value="viewer">"Viewer (read-only)"</option>
                                    <option value="operator" selected>"Operator (train + deploy)"</option>
                                    <option value="admin">"Admin (full access)"</option>
                                </select>
                            </div>
                        </div>
                        <div class="modal-actions">
                            <button class="btn btn-ghost" on:click=move |_| show_create_user.set(false)>"Cancel"</button>
                            <button class="btn btn-primary" on:click=on_create_user>
                                {icons::icon_user_plus()}
                                " Create"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
