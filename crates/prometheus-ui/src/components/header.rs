// ============================================================================
// File: header.rs
// Description: Top navigation header with breadcrumbs, notifications, and user menu
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use leptos_router::hooks::use_location;
use crate::icons;
use crate::api;
use crate::components::layout::UserRoleSignal;

#[component]
pub fn Header(collapsed: RwSignal<bool>) -> impl IntoView {
    let location = use_location();
    let show_notifications = RwSignal::new(false);
    let show_user_menu = RwSignal::new(false);
    let username = RwSignal::new("User".to_string());
    let user_role = RwSignal::new("operator".to_string());

    // Fetch current user info on mount
    leptos::task::spawn_local(async move {
        if let Ok(resp) = api::auth_get("/api/v1/auth/me").send().await {
            if resp.ok() {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    if let Some(name) = body.get("username").and_then(|v| v.as_str()) {
                        username.set(name.to_string());
                    }
                    if let Some(role) = body.get("role").and_then(|v| v.as_str()) {
                        user_role.set(role.to_string());
                        // Update shared context so Sidebar can read it
                        if let Some(shared) = use_context::<UserRoleSignal>() {
                            shared.0.set(role.to_string());
                        }
                    }
                }
            }
        }
    });

    let page_name = move || {
        let path = location.pathname.get();
        match path.as_str() {
            "/" => "Dashboard",
            "/datasets" => "Datasets",
            "/training" => "Training",
            "/models" => "Models",
            "/deployment" => "Deployment",
            "/evaluation" => "Evaluation",
            "/monitor" => "Training Monitor",
            "/agent" => "PrometheusForge",
            "/quantize" => "Quantization",
            "/admin" => "Admin Panel",
            "/settings" => "Profile & Settings",
            "/login" => "Login",
            p if p.starts_with("/datasets/") => "Dataset Detail",
            p if p.starts_with("/training/") => "Training Detail",
            p if p.starts_with("/models/") => "Model Detail",
            _ => "Prometheus",
        }
    };

    let is_admin = move || {
        let r = user_role.get();
        r == "admin"
    };

    // Close dropdowns when clicking outside
    let on_backdrop = move |_: web_sys::MouseEvent| {
        show_notifications.set(false);
        show_user_menu.set(false);
    };

    view! {
        <header class="header">
            <div style="display: flex; align-items: center; gap: 16px;">
                <button
                    class="btn btn-ghost btn-sm"
                    on:click=move |_: web_sys::MouseEvent| collapsed.update(|c| *c = !*c)
                    title="Toggle sidebar"
                    style="padding: 4px 8px; border: none;"
                >
                    {"\u{2630}"}
                </button>
                <div class="header-breadcrumbs">
                    "Prometheus / " <span>{page_name}</span>
                </div>
            </div>
            <div style="display: flex; align-items: center; gap: 8px;">
                // Notifications bell
                <div style="position: relative;">
                    <button
                        class="btn btn-ghost btn-sm"
                        style="position: relative; border: none; cursor: pointer;"
                        on:click=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            show_user_menu.set(false);
                            show_notifications.update(|v| *v = !*v);
                        }
                        title="Notifications"
                    >
                        {icons::icon_bell()}
                    </button>

                    // Notifications dropdown
                    <Show when=move || show_notifications.get() fallback=|| ()>
                        <div style="position: absolute; right: 0; top: 100%; margin-top: 8px; width: 320px; background: #FFFDF7; border: 1px solid #E8D4C4; border-radius: 10px; box-shadow: 0 10px 25px rgba(0,0,0,0.1); z-index: 50; overflow: hidden;">
                            <div style="padding: 14px 16px; border-bottom: 1px solid #E8D4C4; display: flex; justify-content: space-between; align-items: center;">
                                <span style="font-weight: 600; font-size: 0.9rem; color: #111827;">"Notifications"</span>
                            </div>
                            <div style="padding: 24px 16px; text-align: center; color: #9ca3af; font-size: 0.85rem;">
                                {icons::icon_bell()}
                                <p style="margin-top: 8px;">"No new notifications"</p>
                            </div>
                        </div>
                    </Show>
                </div>

                // User menu
                <div style="position: relative;">
                    <button
                        style="display: flex; align-items: center; gap: 8px; padding: 6px 12px; border-radius: 8px; border: 1px solid #E8D4C4; background: transparent; cursor: pointer; transition: background 0.15s;"
                        on:click=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            show_notifications.set(false);
                            show_user_menu.update(|v| *v = !*v);
                        }
                    >
                        <div style="width: 28px; height: 28px; border-radius: 50%; background: #F5EDE8; display: flex; align-items: center; justify-content: center; color: #C4A484;">
                            {icons::icon_user()}
                        </div>
                        <span style="font-size: 0.875rem; font-weight: 500; color: #111827;">{move || username.get()}</span>
                        <span style="font-size: 0.65rem; color: #9ca3af; transform: rotate(90deg);">{"\u{203A}"}</span>
                    </button>

                    // User dropdown
                    <Show when=move || show_user_menu.get() fallback=|| ()>
                        <div style="position: absolute; right: 0; top: 100%; margin-top: 8px; width: 220px; background: #FFFDF7; border: 1px solid #E8D4C4; border-radius: 10px; box-shadow: 0 10px 25px rgba(0,0,0,0.1); z-index: 50; overflow: hidden;">
                            // User info
                            <div style="padding: 14px 16px; border-bottom: 1px solid #E8D4C4;">
                                <div style="font-weight: 600; font-size: 0.9rem; color: #111827;">{move || username.get()}</div>
                                <div style="display: inline-block; margin-top: 4px; padding: 2px 8px; border-radius: 4px; font-size: 0.7rem; font-weight: 500; background: rgba(20,184,166,0.15); color: #0d9488;">
                                    {move || user_role.get()}
                                </div>
                            </div>
                            // Menu items
                            <div style="padding: 4px 0;">
                                // Profile link
                                <a
                                    href="/settings"
                                    style="display: flex; align-items: center; gap: 10px; padding: 10px 16px; font-size: 0.85rem; color: #374151; text-decoration: none; transition: background 0.1s;"
                                    on:click=move |_| show_user_menu.set(false)
                                >
                                    {icons::icon_user()}
                                    "Profile & Settings"
                                </a>
                                // Billing
                                <a
                                    href="/billing"
                                    style="display: flex; align-items: center; gap: 10px; padding: 10px 16px; font-size: 0.85rem; color: #374151; text-decoration: none; transition: background 0.1s;"
                                    on:click=move |_| show_user_menu.set(false)
                                >
                                    {icons::icon_wallet()}
                                    "Billing & Usage"
                                </a>
                                // Admin panel (admin only)
                                <Show when=is_admin>
                                    <a
                                        href="/admin"
                                        style="display: flex; align-items: center; gap: 10px; padding: 10px 16px; font-size: 0.85rem; color: #374151; text-decoration: none; transition: background 0.1s;"
                                        on:click=move |_| show_user_menu.set(false)
                                    >
                                        {icons::icon_shield()}
                                        "Admin Panel"
                                    </a>
                                </Show>
                                <div style="border-top: 1px solid #E8D4C4; margin: 4px 0;"></div>
                                <button
                                    style="display: flex; align-items: center; gap: 10px; width: 100%; padding: 10px 16px; font-size: 0.85rem; color: #dc2626; background: none; border: none; cursor: pointer; text-align: left; transition: background 0.1s;"
                                    on:click=move |_: web_sys::MouseEvent| {
                                        api::redirect_to_login();
                                    }
                                >
                                    {icons::icon_log_out()}
                                    "Logout"
                                </button>
                            </div>
                        </div>
                    </Show>
                </div>
            </div>
        </header>

        // Invisible backdrop to close dropdowns
        <Show when=move || show_notifications.get() || show_user_menu.get() fallback=|| ()>
            <div
                style="position: fixed; inset: 0; z-index: 30;"
                on:click=on_backdrop
            ></div>
        </Show>
    }
}
