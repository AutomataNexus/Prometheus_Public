// ============================================================================
// File: button.rs
// Description: Reusable button component with primary, ghost, and danger variants
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::callback::Callback;
use leptos::children::Children;

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Ghost,
    Danger,
}

#[component]
pub fn Button(
    #[prop(optional)] variant: Option<ButtonVariant>,
    #[prop(optional)] small: Option<bool>,
    #[prop(optional)] disabled: Option<bool>,
    #[prop(optional)] on_click: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    let variant_class = match variant.unwrap_or(ButtonVariant::Primary) {
        ButtonVariant::Primary => "btn btn-primary",
        ButtonVariant::Ghost => "btn btn-ghost",
        ButtonVariant::Danger => "btn btn-danger",
    };
    let class = if small.unwrap_or(false) {
        format!("{variant_class} btn-sm")
    } else {
        variant_class.to_string()
    };
    let is_disabled = disabled.unwrap_or(false);

    view! {
        <button
            class=class
            disabled=is_disabled
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            {children()}
        </button>
    }
}
