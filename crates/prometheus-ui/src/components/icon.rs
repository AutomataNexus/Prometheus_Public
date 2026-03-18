// ============================================================================
// File: icon.rs
// Description: Dynamic icon component that renders SVG icons by name
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;

#[component]
pub fn Icon(
    #[prop(into)] name: String,
    #[prop(optional)] size: Option<u32>,
    #[prop(optional)] color: Option<&'static str>,
) -> impl IntoView {
    let sz = size.unwrap_or(20);
    let col = color.unwrap_or("currentColor");
    let style = format!("width: {sz}px; height: {sz}px; color: {col}; display: inline-flex;");

    view! {
        <span style=style>
            {match name.as_str() {
                "home" => crate::icons::icon_home().into_any(),
                "database" => crate::icons::icon_database().into_any(),
                "brain" => crate::icons::icon_brain().into_any(),
                "package" => crate::icons::icon_package().into_any(),
                "rocket" => crate::icons::icon_rocket().into_any(),
                "chart" => crate::icons::icon_chart().into_any(),
                "bot" => crate::icons::icon_bot().into_any(),
                "settings" => crate::icons::icon_settings().into_any(),
                "upload" => crate::icons::icon_upload().into_any(),
                "download" => crate::icons::icon_download().into_any(),
                "trash" => crate::icons::icon_trash().into_any(),
                "play" => crate::icons::icon_play().into_any(),
                "stop" => crate::icons::icon_stop().into_any(),
                "check" => crate::icons::icon_check().into_any(),
                "bell" => crate::icons::icon_bell().into_any(),
                "user" => crate::icons::icon_user().into_any(),
                "send" => crate::icons::icon_send().into_any(),
                "activity" => crate::icons::icon_activity().into_any(),
                "key" => crate::icons::icon_key().into_any(),
                "cpu" => crate::icons::icon_cpu().into_any(),
                "shield" => crate::icons::icon_shield().into_any(),
                "convert" => crate::icons::icon_convert().into_any(),
                "wallet" => crate::icons::icon_wallet().into_any(),
                _ => view! { <span></span> }.into_any(),
            }}
        </span>
    }
}
