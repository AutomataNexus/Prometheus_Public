// ============================================================================
// File: toast.rs
// Description: Toast notification system with auto-dismiss and severity levels
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::For;

#[derive(Clone, PartialEq)]
pub enum ToastLevel {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Clone)]
pub struct ToastMessage {
    pub id: u32,
    pub level: ToastLevel,
    pub message: String,
}

/// Provide toast functionality to the app. Call `use_toasts()` to get the signal,
/// and `show_toast()` to add a toast.
pub fn provide_toast_context() -> (ReadSignal<Vec<ToastMessage>>, WriteSignal<Vec<ToastMessage>>) {
    let (toasts, set_toasts) = signal(Vec::<ToastMessage>::new());
    (toasts, set_toasts)
}

static TOAST_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

/// Add a toast that auto-dismisses after ~4 seconds.
pub fn push_toast(set_toasts: WriteSignal<Vec<ToastMessage>>, level: ToastLevel, message: impl Into<String>) {
    let id = TOAST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let msg = ToastMessage { id, level, message: message.into() };
    set_toasts.update(|list| list.push(msg));

    // Auto-dismiss after 4.3s (4s progress bar + 0.3s slide-out)
    let set_toasts_clone = set_toasts;
    leptos::task::spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(4300).await;
        set_toasts_clone.update(|list| list.retain(|t| t.id != id));
    });
}

#[component]
pub fn ToastContainer(toasts: ReadSignal<Vec<ToastMessage>>, set_toasts: WriteSignal<Vec<ToastMessage>>) -> impl IntoView {
    view! {
        <div class="toast-container">
            <For
                each=move || toasts.get()
                key=|t| t.id
                children=move |toast| {
                    let class = match toast.level {
                        ToastLevel::Success => "toast",
                        ToastLevel::Error => "toast toast-error",
                        ToastLevel::Warning => "toast toast-warning",
                        ToastLevel::Info => "toast toast-info",
                    };
                    let icon = match toast.level {
                        ToastLevel::Success => "\u{2713}",
                        ToastLevel::Error => "\u{2717}",
                        ToastLevel::Warning => "\u{26A0}",
                        ToastLevel::Info => "\u{2139}",
                    };
                    let toast_id = toast.id;
                    let on_close = move |_: web_sys::MouseEvent| {
                        set_toasts.update(|list| list.retain(|t| t.id != toast_id));
                    };
                    view! {
                        <div class=class>
                            <span class="toast-icon">{icon}</span>
                            <span class="toast-body">{toast.message.clone()}</span>
                            <button class="toast-close" on:click=on_close>{"\u{00D7}"}</button>
                            <div class="toast-progress"></div>
                        </div>
                    }
                }
            />
        </div>
    }
}
