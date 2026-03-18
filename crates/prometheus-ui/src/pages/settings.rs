// ============================================================================
// File: settings.rs
// Description: User profile settings — password, preferences, personal API keys
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 16, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use crate::components::*;
use crate::icons;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let username = RwSignal::new("User".to_string());
    let user_email = RwSignal::new(String::new());
    let user_role = RwSignal::new("operator".to_string());
    let current_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let mfa_enabled = RwSignal::new(false);

    // Fetch user profile
    leptos::task::spawn_local(async move {
        if let Ok(resp) = crate::api::auth_get("/api/v1/auth/me").send().await {
            if resp.ok() {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    if let Some(name) = body.get("username").and_then(|v| v.as_str()) {
                        username.set(name.to_string());
                    }
                    if let Some(email) = body.get("email").and_then(|v| v.as_str()) {
                        user_email.set(email.to_string());
                    }
                    if let Some(role) = body.get("role").and_then(|v| v.as_str()) {
                        user_role.set(role.to_string());
                    }
                }
            }
        }
        // Check MFA status
        if let Ok(resp) = crate::api::auth_get("/api/v1/profile/preferences").send().await {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(mfa) = body.get("mfa_enabled").and_then(|v| v.as_bool()) {
                    mfa_enabled.set(mfa);
                }
            }
        }
    });

    let on_change_password = move |_| {
        let current = current_password.get_untracked();
        let new_pass = new_password.get_untracked();
        if current.is_empty() || new_pass.len() < 8 { return; }
        leptos::task::spawn_local(async move {
            match crate::api::auth_put("/api/v1/auth/change-password")
                .header("Content-Type", "application/json")
                .body(serde_json::json!({
                    "current_password": current,
                    "new_password": new_pass,
                }).to_string())
                .unwrap()
                .send()
                .await
            {
                Ok(resp) if resp.ok() => {
                    current_password.set(String::new());
                    new_password.set(String::new());
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                        crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success, "Password changed successfully");
                    }
                }
                _ => {
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                        crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Error, "Password change failed. Check your current password.");
                    }
                }
            }
        });
    };

    view! {
        <div>
            <h1 class="page-title">"Profile & Settings"</h1>
            <p class="page-subtitle">"Manage your account, security, and preferences"</p>

            // User Profile Card
            <div class="prometheus-card" style="padding:20px;margin-bottom:24px;">
                <div style="display:flex;align-items:center;gap:16px;">
                    <div style="width:64px;height:64px;border-radius:50%;background:#F5EDE8;display:flex;align-items:center;justify-content:center;color:#C4A484;font-size:24px;">
                        {icons::icon_user()}
                    </div>
                    <div>
                        <div class="text-bold" style="font-size:1.1rem;">{move || username.get()}</div>
                        <div class="text-sm text-muted">{move || user_email.get()}</div>
                        <div style="margin-top:4px;">
                            <span style="display:inline-block;padding:2px 8px;border-radius:4px;font-size:0.75rem;font-weight:500;background:rgba(20,184,166,0.1);color:#0d9488;">
                                {move || user_role.get()}
                            </span>
                        </div>
                    </div>
                </div>
            </div>

            // Change Password
            <div class="prometheus-card" style="padding:20px;margin-bottom:24px;">
                <h3 style="font-size:1rem;font-weight:600;color:#374151;margin-bottom:16px;">"Change Password"</h3>
                <div style="display:grid;gap:12px;max-width:400px;">
                    <div>
                        <label class="input-label">"Current Password"</label>
                        <input class="input-field" type="password"
                            prop:value=move || current_password.get()
                            on:input=move |ev| current_password.set(event_target_value(&ev)) />
                    </div>
                    <div>
                        <label class="input-label">"New Password"</label>
                        <input class="input-field" type="password" placeholder="Minimum 8 characters"
                            prop:value=move || new_password.get()
                            on:input=move |ev| new_password.set(event_target_value(&ev)) />
                    </div>
                    <button class="btn" style="background:#FAF8F5;border:1px solid #E8D4C4;color:#374151;width:fit-content;"
                        on:click=on_change_password>
                        "Change Password"
                    </button>
                </div>
                <p style="font-size:0.75rem;color:#9ca3af;margin-top:8px;">"Passwords are hashed with Argon2id. We cannot see your password."</p>
            </div>

            // MFA
            <div class="prometheus-card" style="padding:20px;margin-bottom:24px;">
                <h3 style="font-size:1rem;font-weight:600;color:#374151;margin-bottom:16px;">"Two-Factor Authentication"</h3>
                <div style="display:flex;align-items:center;gap:12px;">
                    <span style=move || if mfa_enabled.get() { "color:#22c55e;font-weight:600;" } else { "color:#9ca3af;" }>
                        {move || if mfa_enabled.get() { "Enabled" } else { "Not configured" }}
                    </span>
                    <button class="btn btn-sm" style="background:#FAF8F5;border:1px solid #E8D4C4;color:#374151;"
                        on:click=move |_| {
                            leptos::task::spawn_local(async move {
                                let _ = crate::api::auth_post("/api/v1/mfa/setup").send().await;
                                if let Some(set_toasts) = use_context::<WriteSignal<Vec<crate::components::toast::ToastMessage>>>() {
                                    crate::components::toast::push_toast(set_toasts, crate::components::toast::ToastLevel::Success, "MFA setup initiated. Check your authenticator app.");
                                }
                            });
                        }
                    >
                        {move || if mfa_enabled.get() { "Reconfigure" } else { "Setup MFA" }}
                    </button>
                </div>
            </div>

            // Ingestion Keys
            <div class="mb-8">
                <IngestionKeysPanel />
            </div>
        </div>
    }
}
