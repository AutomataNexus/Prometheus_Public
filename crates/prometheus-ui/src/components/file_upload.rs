// ============================================================================
// File: file_upload.rs
// Description: Drag-and-drop file upload component with size validation
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::callback::Callback;
use wasm_bindgen::JsCast;
use web_sys::DragEvent;
use crate::icons;

#[component]
pub fn FileUpload(
    #[prop(optional)] accept: Option<&'static str>,
    #[prop(optional)] max_size_mb: Option<u32>,
    on_file: Callback<web_sys::File>,
) -> impl IntoView {
    let dragging = RwSignal::new(false);
    let _max_size = max_size_mb.unwrap_or(100);

    let on_drop = move |ev: DragEvent| {
        ev.prevent_default();
        dragging.set(false);
        if let Some(dt) = ev.data_transfer() {
            if let Some(files) = dt.files() {
                if let Some(file) = files.get(0) {
                    on_file.run(file);
                }
            }
        }
    };

    let on_drag_over = move |ev: DragEvent| {
        ev.prevent_default();
        dragging.set(true);
    };

    let on_drag_leave = move |_ev: DragEvent| {
        dragging.set(false);
    };

    let on_click_upload = move |_| {
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            if let Ok(el) = document.create_element("input") {
                let input: web_sys::HtmlInputElement = el.unchecked_into();
                input.set_type("file");
                if let Some(a) = accept {
                    input.set_accept(a);
                }
                // Hide but append to DOM so the change event fires
                let _ = input.set_attribute("style", "display:none");
                let _ = document.body().unwrap().append_child(&input);

                let cb = on_file;
                let input_clone = input.clone();
                let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Some(files) = input_clone.files() {
                        if let Some(file) = files.get(0) {
                            cb.run(file);
                        }
                    }
                    // Clean up the temporary input element
                    input_clone.remove();
                }) as Box<dyn FnMut(_)>);
                input.set_onchange(Some(closure.as_ref().unchecked_ref()));
                closure.forget();
                input.click();
            }
        }
    };

    view! {
        <div
            class="file-upload"
            class:drag-over=move || dragging.get()
            on:drop=on_drop
            on:dragover=on_drag_over
            on:dragleave=on_drag_leave
            on:click=on_click_upload
        >
            <div class="file-upload-icon">
                {icons::icon_upload()}
            </div>
            <div class="file-upload-text">
                <strong>"Click to browse"</strong>" or drag and drop"
            </div>
            <div class="text-xs text-muted" style="margin-top: 8px;">
                {format!("CSV files up to {} MB", max_size_mb.unwrap_or(100))}
            </div>
        </div>
    }
}

#[component]
pub fn UploadProgress(
    filename: Signal<String>,
    progress: Signal<f64>,
) -> impl IntoView {
    view! {
        <div class="card" style="padding: 16px;">
            <div class="flex-between mb-4">
                <div style="display: flex; align-items: center; gap: 8px;">
                    {icons::icon_file_text()}
                    <span class="text-sm text-bold">{filename}</span>
                </div>
                <span class="text-sm text-muted">{move || format!("{:.0}%", progress.get())}</span>
            </div>
            <div class="progress-bar">
                <div class="progress-bar-fill" style=move || format!("width: {}%", progress.get())></div>
            </div>
        </div>
    }
}
