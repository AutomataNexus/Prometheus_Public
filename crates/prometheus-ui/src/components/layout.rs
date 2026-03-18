// ============================================================================
// File: layout.rs
// Description: AppShell layout component combining sidebar, header, and content area
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::children::Children;
use super::sidebar::Sidebar;
use super::header::Header;
use super::toast::{ToastContainer, provide_toast_context};

/// Shared user role signal — set by Header, read by Sidebar.
#[derive(Clone, Copy)]
pub struct UserRoleSignal(pub RwSignal<String>);

#[component]
pub fn AppShell(children: Children) -> impl IntoView {
    let collapsed = RwSignal::new(false);
    let (toasts, set_toasts) = provide_toast_context();

    // Store set_toasts in context so any page can push toasts
    provide_context(set_toasts);

    // Shared user role context
    let user_role = UserRoleSignal(RwSignal::new("operator".to_string()));
    provide_context(user_role);

    let main_style = move || {
        if collapsed.get() {
            "margin-left: 64px;"
        } else {
            "margin-left: 220px;"
        }
    };

    view! {
        <div class="app-shell">
            <Sidebar collapsed=collapsed />
            <div class="main-content" style=main_style>
                <Header collapsed=collapsed />
                <div class="page-content">
                    {children()}
                </div>
            </div>
            <ToastContainer toasts=toasts set_toasts=set_toasts />
        </div>
    }
}
