// ============================================================================
// File: table.rs
// Description: Sortable data table component with column definitions and pagination
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use leptos::callback::Callback;

#[derive(Clone)]
pub struct Column {
    pub key: String,
    pub label: String,
    pub sortable: bool,
}

#[component]
pub fn DataTable(
    columns: Vec<Column>,
    rows: Signal<Vec<Vec<String>>>,
    #[prop(optional)] on_row_click: Option<Callback<usize>>,
    #[prop(optional)] empty_message: Option<&'static str>,
) -> impl IntoView {
    let current_page = RwSignal::new(0usize);
    let page_size = 10;

    let total_pages = move || {
        let total = rows.get().len();
        (total + page_size - 1) / page_size
    };

    let paged_rows = move || {
        let all = rows.get();
        let start = current_page.get() * page_size;
        let end = (start + page_size).min(all.len());
        if start < all.len() {
            all[start..end].to_vec()
        } else {
            vec![]
        }
    };

    let page_offset = move || current_page.get() * page_size;

    view! {
        <div>
            <Show
                when=move || !rows.get().is_empty()
                fallback=move || view! {
                    <div style="padding: 48px; text-align: center; color: #6b7280;">
                        {empty_message.unwrap_or("No data available")}
                    </div>
                }
            >
                <table class="data-table">
                    <thead>
                        <tr>
                            {columns.iter().map(|col| {
                                let label = col.label.clone();
                                view! { <th>{label}</th> }
                            }).collect_view()}
                        </tr>
                    </thead>
                    <tbody>
                        {move || paged_rows().into_iter().enumerate().map(|(i, row)| {
                            let idx = page_offset() + i;
                            let click_handler = on_row_click;
                            view! {
                                <tr
                                    style=move || if click_handler.is_some() { "cursor: pointer;" } else { "" }
                                    on:click=move |_| {
                                        if let Some(cb) = click_handler {
                                            cb.run(idx);
                                        }
                                    }
                                >
                                    {row.into_iter().map(|cell| {
                                        view! { <td>{cell}</td> }
                                    }).collect_view()}
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
                <Show when=move || { total_pages() > 1 }>
                    <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0;">
                        <span class="text-sm text-muted">
                            {move || format!("Page {} of {}", current_page.get() + 1, total_pages())}
                        </span>
                        <div style="display: flex; gap: 8px;">
                            <button
                                class="btn btn-ghost btn-sm"
                                disabled=move || current_page.get() == 0
                                on:click=move |_| current_page.update(|p| *p = p.saturating_sub(1))
                            >
                                "Previous"
                            </button>
                            <button
                                class="btn btn-ghost btn-sm"
                                disabled=move || current_page.get() + 1 >= total_pages()
                                on:click=move |_| current_page.update(|p| *p += 1)
                            >
                                "Next"
                            </button>
                        </div>
                    </div>
                </Show>
            </Show>
        </div>
    }
}
