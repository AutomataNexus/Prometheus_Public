// ============================================================================
// File: modal.rs
// Description: Modal dialog overlay component with backdrop click-to-close
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

#[component]
pub fn Modal(
    title: &'static str,
    show: ReadSignal<bool>,
    on_close: Callback<()>,
    children: Children,
) -> impl IntoView {
    let rendered = children();
    view! {
        <div class="modal-backdrop" style=move || if show.get() { "" } else { "display: none;" }
            on:click=move |_: web_sys::MouseEvent| on_close.run(())
        >
            <div class="modal" on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()>
                <div class="flex-between mb-4">
                    <h2 class="modal-title">{title}</h2>
                    <button class="btn btn-ghost btn-sm" on:click=move |_: web_sys::MouseEvent| on_close.run(())>
                        {crate::icons::icon_x()}
                    </button>
                </div>
                {rendered}
            </div>
        </div>
    }
}

#[component]
pub fn ConfirmModal(
    title: &'static str,
    message: String,
    show: ReadSignal<bool>,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
    #[prop(optional)] confirm_text: Option<&'static str>,
    #[prop(optional)] danger: Option<bool>,
) -> impl IntoView {
    let btn_class = if danger.unwrap_or(false) {
        "btn btn-danger"
    } else {
        "btn btn-primary"
    };

    view! {
        <div class="modal-backdrop" style=move || if show.get() { "" } else { "display: none;" }
            on:click=move |_: web_sys::MouseEvent| on_cancel.run(())
        >
            <div class="modal" on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()>
                <h2 class="modal-title">{title}</h2>
                <p class="text-sm text-muted">{message.clone()}</p>
                <div class="modal-actions">
                    <button class="btn btn-ghost" on:click=move |_: web_sys::MouseEvent| on_cancel.run(())>
                        "Cancel"
                    </button>
                    <button class=btn_class on:click=move |_: web_sys::MouseEvent| on_confirm.run(())>
                        {confirm_text.unwrap_or("Confirm")}
                    </button>
                </div>
            </div>
        </div>
    }
}
