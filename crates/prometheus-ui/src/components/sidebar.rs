// ============================================================================
// File: sidebar.rs
// Description: Collapsible navigation sidebar with role-based menu items
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
use crate::components::icon::Icon;
use crate::components::layout::UserRoleSignal;

#[derive(Clone)]
struct NavItem {
    label: &'static str,
    path: &'static str,
    icon_name: &'static str,
    admin_only: bool,
}

#[component]
pub fn Sidebar(collapsed: RwSignal<bool>) -> impl IntoView {
    let location = use_location();

    // Reordered nav items per user request:
    // Dashboard, Datasets, Agent, Models, Training, Evaluation, Convert, Deployment
    // Settings is admin-only
    let nav_items: Vec<NavItem> = vec![
        NavItem { label: "Dashboard", path: "/dashboard", icon_name: "home", admin_only: false },
        NavItem { label: "Datasets", path: "/datasets", icon_name: "database", admin_only: false },
        NavItem { label: "Agent", path: "/agent", icon_name: "bot", admin_only: false },
        NavItem { label: "Models", path: "/models", icon_name: "package", admin_only: false },
        NavItem { label: "Training", path: "/training", icon_name: "brain", admin_only: false },
        NavItem { label: "Monitor", path: "/monitor", icon_name: "activity", admin_only: false },
        NavItem { label: "Evaluation", path: "/evaluation", icon_name: "chart", admin_only: false },
        NavItem { label: "Convert", path: "/convert", icon_name: "convert", admin_only: false },
        NavItem { label: "Quantize", path: "/quantize", icon_name: "cpu", admin_only: false },
        NavItem { label: "Deployment", path: "/deployment", icon_name: "rocket", admin_only: false },
        NavItem { label: "Billing", path: "/billing", icon_name: "wallet", admin_only: false },
        NavItem { label: "Settings", path: "/settings", icon_name: "settings", admin_only: true },
    ];

    let sidebar_style = move || {
        if collapsed.get() {
            "width: 64px;"
        } else {
            "width: 220px;"
        }
    };

    let is_admin = move || {
        use_context::<UserRoleSignal>()
            .map(|r| r.0.get() == "admin")
            .unwrap_or(false)
    };

    view! {
        <nav class="sidebar" style=sidebar_style>
            <div class="sidebar-logo" style="display: flex; align-items: center; gap: 12px; padding: 16px; overflow: hidden;">
                <img
                    src="/assets/logo.png?v=3"
                    alt="Prometheus"
                    style=move || if collapsed.get() {
                        "width: 32px; height: 32px; min-width: 32px; border-radius: 8px; transition: all 0.2s ease;"
                    } else {
                        "width: 48px; height: 48px; min-width: 48px; border-radius: 10px; transition: all 0.2s ease;"
                    }
                />
                <div style=move || if collapsed.get() { "display:none;" } else { "" }>
                    <h1 class="sidebar-logo-title">"Prometheus"</h1>
                    <span class="sidebar-slogan">"AI-Forged Edge Intelligence"</span>
                </div>
            </div>
            <div class="sidebar-nav">
                {nav_items.into_iter().map(|item| {
                    let path = item.path;
                    let pathname = location.pathname.clone();
                    let is_active = move || {
                        let current = pathname.get();
                        if path == "/dashboard" {
                            current == "/dashboard" || current == "/"
                        } else {
                            current.starts_with(path)
                        }
                    };
                    let admin_only = item.admin_only;
                    let should_show = move || !admin_only || is_admin();
                    view! {
                        <Show when=should_show>
                            <a
                                href=path
                                class="nav-item"
                                class:active=is_active
                                title=item.label
                            >
                                <Icon name=item.icon_name.to_string() size=20 />
                                <span style=move || if collapsed.get() { "display:none;" } else { "" }>
                                    {item.label}
                                </span>
                            </a>
                        </Show>
                    }
                }).collect_view()}
            </div>
            <div
                class="sidebar-secured"
                style=move || if collapsed.get() { "display:none;" } else { "" }
            >
                {crate::icons::icon_shield()}
                <span>"Secured by Nexus"</span>
            </div>
            <button
                class="sidebar-toggle"
                on:click=move |_: web_sys::MouseEvent| collapsed.update(|c| *c = !*c)
                title=move || if collapsed.get() { "Expand sidebar" } else { "Collapse sidebar" }
            >
                <span style=move || if collapsed.get() { "transform: rotate(180deg); display:inline-block;" } else { "display:inline-block;" }>
                    {"\u{2039}"}
                </span>
            </button>
            <div
                class="sidebar-version"
                style=move || if collapsed.get() { "display:none;" } else { "" }
            >
                "Prometheus v0.1.0"
            </div>
        </nav>
    }
}
