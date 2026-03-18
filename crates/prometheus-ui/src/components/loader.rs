// ============================================================================
// File: loader.rs
// Description: Loading spinner and skeleton placeholder components
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;

#[component]
pub fn Spinner(#[prop(optional)] size: Option<u32>) -> impl IntoView {
    let sz = size.unwrap_or(24);
    let style = format!(
        "width: {sz}px; height: {sz}px; border: 3px solid #E8D4C4; border-top-color: #14b8a6; border-radius: 50%; animation: spin 0.8s linear infinite;"
    );
    view! {
        <div style=style></div>
        <style>"@keyframes spin { to { transform: rotate(360deg); } }"</style>
    }
}

#[component]
pub fn SkeletonLine(#[prop(optional)] width: Option<&'static str>) -> impl IntoView {
    let w = width.unwrap_or("100%");
    view! {
        <div class="skeleton" style=format!("height: 16px; width: {w}; margin-bottom: 8px;")></div>
    }
}

#[component]
pub fn SkeletonCard() -> impl IntoView {
    view! {
        <div class="card" style="min-height: 120px;">
            <SkeletonLine width="60%" />
            <SkeletonLine width="40%" />
            <SkeletonLine width="80%" />
        </div>
    }
}

#[component]
pub fn PageLoader() -> impl IntoView {
    view! {
        <div style="display: flex; align-items: center; justify-content: center; padding: 64px;">
            <div style="text-align: center;">
                <Spinner size=40 />
                <p class="text-sm text-muted" style="margin-top: 16px;">"Loading..."</p>
            </div>
        </div>
    }
}
