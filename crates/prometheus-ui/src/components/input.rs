// ============================================================================
// File: input.rs
// Description: Text input and select form components with labels and validation
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;

#[component]
pub fn TextInput(
    label: &'static str,
    #[prop(optional)] placeholder: Option<&'static str>,
    #[prop(optional)] input_type: Option<&'static str>,
    #[prop(optional)] error: Option<Signal<Option<String>>>,
    #[prop(optional)] help: Option<&'static str>,
    value: RwSignal<String>,
) -> impl IntoView {
    let has_error = move || error.map(|e| e.get().is_some()).unwrap_or(false);
    let error_msg = move || error.and_then(|e| e.get());

    view! {
        <div class="input-group">
            <label class="input-label">{label}</label>
            <input
                type=input_type.unwrap_or("text")
                class="input-field"
                class:input-error=has_error
                placeholder=placeholder.unwrap_or("")
                prop:value=move || value.get()
                on:input=move |ev| {
                    value.set(event_target_value(&ev));
                }
            />
            {help.map(|h| view! { <span class="input-help">{h}</span> })}
            {move || error_msg().map(|msg| view! { <span class="input-error-msg">{msg}</span> })}
        </div>
    }
}

#[component]
pub fn TextArea(
    label: &'static str,
    #[prop(optional)] placeholder: Option<&'static str>,
    #[prop(optional)] rows: Option<u32>,
    value: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="input-group">
            <label class="input-label">{label}</label>
            <textarea
                class="input-field"
                placeholder=placeholder.unwrap_or("")
                rows=rows.unwrap_or(4)
                prop:value=move || value.get()
                on:input=move |ev| {
                    value.set(event_target_value(&ev));
                }
            ></textarea>
        </div>
    }
}

#[component]
pub fn SelectInput(
    label: &'static str,
    options: Vec<(&'static str, &'static str)>,
    value: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="input-group">
            <label class="input-label">{label}</label>
            <select
                class="input-field"
                prop:value=move || value.get()
                on:change=move |ev| {
                    value.set(event_target_value(&ev));
                }
            >
                {options.into_iter().map(|(val, display)| {
                    view! { <option value=val>{display}</option> }
                }).collect_view()}
            </select>
        </div>
    }
}
