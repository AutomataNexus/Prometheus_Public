// ============================================================================
// File: dataset_detail.rs
// Description: Dataset detail page with preview, validation, and version management
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use leptos_router::hooks::use_params_map;
use crate::components::*;
use crate::components::toast::{ToastLevel, ToastMessage, push_toast};

#[component]
pub fn DatasetDetailPage() -> impl IntoView {
    let params = use_params_map();
    let dataset = RwSignal::new(None::<serde_json::Value>);
    let preview_data = RwSignal::new(None::<serde_json::Value>);
    let ds_status = RwSignal::new("active".to_string());
    let is_validated = RwSignal::new(false);
    let is_locked = RwSignal::new(false);
    let validating = RwSignal::new(false);
    let validation_result = RwSignal::new(None::<serde_json::Value>);
    let show_stats = RwSignal::new(false);

    // Preview pagination/sort state
    let current_page = RwSignal::new(0usize);
    let sort_col = RwSignal::new(None::<usize>);
    let sort_dir = RwSignal::new("asc".to_string());
    // Trigger signal — bump this to re-fetch preview
    let fetch_trigger = RwSignal::new(0u32);

    let id = move || params.get().get("id").unwrap_or_default();

    // Fetch dataset detail on mount
    {
        let dataset = dataset;
        let ds_status = ds_status;
        let is_validated = is_validated;
        let is_locked = is_locked;
        leptos::task::spawn_local(async move {
            let ds_id = params.get_untracked().get("id").unwrap_or_default();
            if let Ok(resp) = crate::api::auth_get(&format!("/api/v1/datasets/{ds_id}"))
                .send()
                .await
            {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("active").to_string();
                    ds_status.set(status);
                    is_validated.set(data.get("is_validated").and_then(|v| v.as_bool()).unwrap_or(false));
                    is_locked.set(data.get("locked").and_then(|v| v.as_bool()).unwrap_or(false));
                    dataset.set(Some(data));
                }
            }
            // Trigger initial preview fetch
            fetch_trigger.set(1);
        });
    }

    // Reactive preview fetch — runs whenever fetch_trigger changes
    Effect::new(move || {
        let _trigger = fetch_trigger.get();
        if _trigger == 0 { return; } // skip initial
        let ds_id = params.get_untracked().get("id").unwrap_or_default();
        let page = current_page.get_untracked();
        let sc = sort_col.get_untracked();
        let sd = sort_dir.get_untracked();
        leptos::task::spawn_local(async move {
            let mut url = format!("/api/v1/datasets/{ds_id}/preview?page={page}&page_size=100");
            if let Some(col) = sc {
                url.push_str(&format!("&sort_col={col}&sort_dir={sd}"));
            }
            if let Ok(resp) = crate::api::auth_get(&url).send().await {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    preview_data.set(Some(data));
                }
            }
        });
    });

    view! {
        <div>
            <Show
                when=move || dataset.get().is_some()
                fallback=|| view! { <PageLoader /> }
            >
                {move || {
                    let ds = dataset.get().unwrap();
                    let name = ds.get("name").and_then(|v| v.as_str()).unwrap_or("Dataset").to_string();
                    let domain = ds.get("domain").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let rows = ds.get("row_count").and_then(|v| v.as_u64()).unwrap_or(0);
                    let cols = ds.get("columns").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
                    let ds_id = id();

                    // Metadata fields
                    let location = ds.get("location").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let source_field = ds.get("source").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let device_id = ds.get("device_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let equipment_id = ds.get("equipment_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let controller_ip = ds.get("controller_ip").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let collection_interval = ds.get("collection_interval").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let column_count = ds.get("columns").and_then(|v| v.as_array()).map(|a| a.len().to_string()).unwrap_or_default();
                    let tags = ds.get("tags")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_default();
                    let source = ds.get("source").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let created_at = ds.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let status_str = ds.get("status").and_then(|v| v.as_str()).unwrap_or("active").to_string();

                    let meta_entries: Vec<(&str, String)> = vec![
                        ("Equipment ID", equipment_id),
                        ("Location", location),
                        ("Source", source_field),
                        ("Device ID", device_id),
                        ("Controller IP", controller_ip),
                        ("Collection Interval", collection_interval),
                        ("Columns", column_count),
                        ("Tags", tags),
                        ("Source", source),
                        ("Ingestion Status", if status_str == "paused" { "Paused".into() } else { "Active".into() }),
                        ("Created", created_at),
                    ].into_iter().filter(|(_, v)| !v.is_empty()).collect();
                    let has_meta = !meta_entries.is_empty();

                    let ds_id_toggle = ds_id.clone();
                    let ds_id_delete = ds_id.clone();
                    let ds_id_validate = ds_id.clone();
                    let ds_id_unlock = ds_id.clone();
                    let ds_id_analyze = ds_id.clone();

                    // Column stats as compact table data
                    let col_stats: Vec<(String, f64, f64, f64, f64)> = ds.get("column_stats")
                        .and_then(|cs| cs.as_object())
                        .map(|stats| {
                            stats.iter().map(|(col, stat)| {
                                let min = stat.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let max = stat.get("max").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let mean = stat.get("mean").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let std = stat.get("std").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                (col.clone(), min, max, mean, std)
                            }).collect()
                        })
                        .unwrap_or_default();
                    let stats_count = col_stats.len();

                    view! {
                        <div>
                            <div class="flex-between mb-8">
                                <div>
                                    <h1 class="page-title">{name}</h1>
                                    <p class="page-subtitle">{format!("{domain} \u{2022} {rows} rows \u{2022} {cols} columns")}</p>
                                </div>
                                <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
                                    // Validation badge
                                    <span style=move || {
                                        if is_locked.get() && is_validated.get() {
                                            "padding: 4px 12px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; background: #dbeafe; color: #1e40af;"
                                        } else if is_validated.get() {
                                            "padding: 4px 12px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; background: #d1fae5; color: #065f46;"
                                        } else {
                                            "padding: 4px 12px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; background: #fef3c7; color: #92400e;"
                                        }
                                    }>
                                        {move || {
                                            if is_locked.get() && is_validated.get() {
                                                "Locked (Training Ready)"
                                            } else if is_validated.get() {
                                                "Validated"
                                            } else {
                                                "Not Validated"
                                            }
                                        }}
                                    </span>
                                    // Ingestion status badge
                                    <span style=move || {
                                        let s = ds_status.get();
                                        if s == "paused" {
                                            "padding: 4px 12px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; background: #fef3c7; color: #92400e;"
                                        } else {
                                            "padding: 4px 12px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; background: #d1fae5; color: #065f46;"
                                        }
                                    }>
                                        {move || if ds_status.get() == "paused" { "Paused" } else { "Active" }}
                                    </span>
                                    // Pause/Resume toggle
                                    <button
                                        class="btn btn-ghost btn-sm"
                                        style="border: 1px solid #e2e8f0;"
                                        on:click=move |_: web_sys::MouseEvent| {
                                            let did = ds_id_toggle.clone();
                                            let current = ds_status.get();
                                            let new_status = if current == "paused" { "active" } else { "paused" };
                                            let new_status_owned = new_status.to_string();
                                            leptos::task::spawn_local(async move {
                                                let body = serde_json::json!({ "status": new_status_owned });
                                                if let Ok(resp) = crate::api::auth_post(&format!("/api/v1/datasets/{did}/status"))
                                                    .header("Content-Type", "application/json")
                                                    .body(body.to_string())
                                                    .unwrap()
                                                    .send()
                                                    .await
                                                {
                                                    if resp.ok() {
                                                        ds_status.set(new_status_owned);
                                                    }
                                                }
                                            });
                                        }
                                    >
                                        {move || if ds_status.get() == "paused" { "Resume Ingestion" } else { "Pause Ingestion" }}
                                    </button>
                                    // Validate button (hidden when locked)
                                    <button
                                        class="btn btn-primary btn-sm"
                                        style=move || if is_locked.get() { "display:none;" } else { "" }
                                        disabled=move || validating.get()
                                        on:click=move |_: web_sys::MouseEvent| {
                                            let did = ds_id_validate.clone();
                                            validating.set(true);
                                            validation_result.set(None);
                                            leptos::task::spawn_local(async move {
                                                match crate::api::auth_post(&format!("/api/v1/datasets/{did}/validate"))
                                                    .header("Content-Type", "application/json")
                                                    .body("{}")
                                                    .unwrap()
                                                    .send()
                                                    .await
                                                {
                                                    Ok(resp) if resp.ok() => {
                                                        if let Ok(result) = resp.json::<serde_json::Value>().await {
                                                            let valid = result.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
                                                            is_validated.set(valid);
                                                            is_locked.set(valid);
                                                            validation_result.set(Some(result));
                                                            if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                                                if valid {
                                                                    push_toast(set_toasts, ToastLevel::Success, "Dataset validated and locked for training");
                                                                } else {
                                                                    push_toast(set_toasts, ToastLevel::Error, "Validation failed - see details below");
                                                                }
                                                            }
                                                        }
                                                    }
                                                    _ => {
                                                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                                            push_toast(set_toasts, ToastLevel::Error, "Validation request failed");
                                                        }
                                                    }
                                                }
                                                validating.set(false);
                                            });
                                        }
                                    >
                                        {move || if validating.get() { "Validating..." } else { "Validate" }}
                                    </button>
                                    // Unlock button (hidden when not locked)
                                    <button
                                        class="btn btn-ghost btn-sm"
                                        style=move || if is_locked.get() { "border: 1px solid #e2e8f0; color: #dc2626;" } else { "display:none;" }
                                        on:click=move |_: web_sys::MouseEvent| {
                                            let did = ds_id_unlock.clone();
                                            leptos::task::spawn_local(async move {
                                                if let Ok(resp) = crate::api::auth_post(&format!("/api/v1/datasets/{did}/unlock"))
                                                    .header("Content-Type", "application/json")
                                                    .body("{}")
                                                    .unwrap()
                                                    .send()
                                                    .await
                                                {
                                                    if resp.ok() {
                                                        is_validated.set(false);
                                                        is_locked.set(false);
                                                        if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                                            push_toast(set_toasts, ToastLevel::Info, "Dataset unlocked - will need re-validation before training");
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    >
                                        "Unlock"
                                    </button>
                                    <button
                                        class="btn btn-primary btn-sm"
                                        on:click=move |_: web_sys::MouseEvent| {
                                            if let Some(window) = web_sys::window() {
                                                if let Some(storage) = window.local_storage().ok().flatten() {
                                                    let _ = storage.set_item("prometheus_analyze_dataset", &ds_id_analyze);
                                                }
                                                // Force full page navigation with query param as backup
                                                let _ = window.location().assign(&format!("/agent?dataset={}", ds_id_analyze));
                                            }
                                        }
                                    >
                                        {crate::icons::icon_bot()}
                                        " Analyze"
                                    </button>
                                    <button
                                        class="btn btn-danger btn-sm"
                                        on:click=move |_: web_sys::MouseEvent| {
                                            let did = ds_id_delete.clone();
                                            if let Some(window) = web_sys::window() {
                                                let confirmed = window.confirm_with_message(
                                                    "Delete this dataset? This cannot be undone."
                                                ).unwrap_or(false);
                                                if confirmed {
                                                    leptos::task::spawn_local(async move {
                                                        if let Ok(resp) = crate::api::auth_delete(&format!("/api/v1/datasets/{did}"))
                                                            .send()
                                                            .await
                                                        {
                                                            if resp.ok() {
                                                                if let Some(w) = web_sys::window() {
                                                                    let _ = w.location().set_href("/datasets");
                                                                }
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                    >
                                        {crate::icons::icon_trash()}
                                        " Delete"
                                    </button>
                                </div>
                            </div>

                            // Validation results panel
                            <Show when=move || validation_result.get().is_some()>
                                {move || {
                                    let result = validation_result.get().unwrap();
                                    let valid = result.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
                                    let rows_scanned = result.get("rows_scanned").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let errors = result.get("errors").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                    let warnings = result.get("warnings").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                    let columns = result.get("columns").and_then(|v| v.as_array()).cloned().unwrap_or_default();

                                    let card_style = if valid {
                                        "border-left: 4px solid #10b981; background: #ecfdf5; padding: 20px;"
                                    } else {
                                        "border-left: 4px solid #ef4444; background: #fef2f2; padding: 20px;"
                                    };
                                    let heading_style = if valid { "color: #065f46; margin: 0;" } else { "color: #991b1b; margin: 0;" };

                                    view! {
                                        <div class="card mb-8" style=card_style>
                                            <div class="flex-between mb-4">
                                                <h3 class="text-bold" style=heading_style>
                                                    {if valid { "Validation Passed" } else { "Validation Failed" }}
                                                </h3>
                                                <span class="text-sm text-muted">{format!("{rows_scanned} rows scanned")}</span>
                                            </div>

                                            {if !errors.is_empty() {
                                                let items = errors.iter().filter_map(|e| e.as_str().map(|s| s.to_string())).map(|e| {
                                                    view! { <li style="color: #dc2626; margin-bottom: 4px;">{e}</li> }
                                                }).collect_view();
                                                view! {
                                                    <div class="mb-4">
                                                        <span class="text-sm text-bold" style="color: #dc2626;">"Errors:"</span>
                                                        <ul style="margin: 4px 0 0 16px; padding: 0;">{items}</ul>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <div></div> }.into_any()
                                            }}

                                            {if !warnings.is_empty() {
                                                let items = warnings.iter().filter_map(|w| w.as_str().map(|s| s.to_string())).map(|w| {
                                                    view! { <li style="color: #d97706; margin-bottom: 4px;">{w}</li> }
                                                }).collect_view();
                                                view! {
                                                    <div class="mb-4">
                                                        <span class="text-sm text-bold" style="color: #d97706;">"Warnings:"</span>
                                                        <ul style="margin: 4px 0 0 16px; padding: 0;">{items}</ul>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <div></div> }.into_any()
                                            }}

                                            <div style="overflow-x: auto; margin-top: 12px;">
                                                <table class="data-table" style="font-size: 0.8rem;">
                                                    <thead><tr><th>"Column"</th><th>"Type"</th><th>"Numeric"</th><th>"String"</th><th>"Empty"</th><th>"Issues"</th></tr></thead>
                                                    <tbody>
                                                        {columns.iter().map(|col| {
                                                            let cname = col.get("column").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                            let typ = col.get("inferred_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                            let nc = col.get("numeric_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                                            let sc = col.get("string_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                                            let ec = col.get("empty_pct").and_then(|v| v.as_str()).unwrap_or("0.0%").to_string();
                                                            let issues = col.get("issues").and_then(|v| v.as_array())
                                                                .map(|a| a.iter().filter_map(|i| i.as_str()).collect::<Vec<_>>().join("; "))
                                                                .unwrap_or_default();
                                                            let type_style = match typ.as_str() {
                                                                "numeric" => "color: #065f46; font-weight: 600;",
                                                                "string" => "color: #1e40af; font-weight: 600;",
                                                                "mixed" => "color: #dc2626; font-weight: 600;",
                                                                _ => "color: #6b7280; font-weight: 600;",
                                                            };
                                                            view! {
                                                                <tr>
                                                                    <td class="text-bold">{cname}</td>
                                                                    <td><span style=type_style>{typ}</span></td>
                                                                    <td>{nc.to_string()}</td>
                                                                    <td>{sc.to_string()}</td>
                                                                    <td>{ec}</td>
                                                                    <td style="color: #dc2626;">{issues}</td>
                                                                </tr>
                                                            }
                                                        }).collect_view()}
                                                    </tbody>
                                                </table>
                                            </div>
                                            <button class="btn btn-ghost btn-sm mt-4" style="border: 1px solid #d1d5db;"
                                                on:click=move |_: web_sys::MouseEvent| validation_result.set(None)
                                            >"Dismiss"</button>
                                        </div>
                                    }
                                }}
                            </Show>

                            {if has_meta {
                                let items = meta_entries.iter().map(|(label, value)| {
                                    let label = *label;
                                    let value = value.clone();
                                    view! {
                                        <div>
                                            <span class="text-xs text-muted">{label}</span>
                                            <div class="text-sm text-bold">{value}</div>
                                        </div>
                                    }
                                }).collect_view();
                                view! {
                                    <Card title="Dataset Info" class="mb-8">
                                        <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 16px;">
                                            {items}
                                        </div>
                                    </Card>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }}

                            // Column Statistics — compact collapsible table
                            {if stats_count > 0 {
                                view! {
                                    <div class="card mb-8" style="padding: 0; overflow: hidden;">
                                        <button
                                            style="width: 100%; display: flex; justify-content: space-between; align-items: center; padding: 14px 20px; background: none; border: none; cursor: pointer; font-size: 0.9rem; font-weight: 600; color: #111827;"
                                            on:click=move |_: web_sys::MouseEvent| show_stats.update(|v| *v = !*v)
                                        >
                                            <span>{format!("Column Statistics ({stats_count} columns)")}</span>
                                            <span style="font-size: 0.75rem; color: #9ca3af;">
                                                {move || if show_stats.get() { "\u{25B2} Hide" } else { "\u{25BC} Show" }}
                                            </span>
                                        </button>
                                        <div style=move || if show_stats.get() {
                                            "overflow-x: auto; border-top: 1px solid #e5e7eb;"
                                        } else { "display: none;" }>
                                            <table class="data-table" style="font-size: 0.8rem; margin: 0;">
                                                <thead>
                                                    <tr>
                                                        <th>"Column"</th>
                                                        <th style="text-align: right;">"Min"</th>
                                                        <th style="text-align: right;">"Max"</th>
                                                        <th style="text-align: right;">"Mean"</th>
                                                        <th style="text-align: right;">"Std Dev"</th>
                                                        <th style="text-align: right;">"Range"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {col_stats.iter().map(|(col, min, max, mean, std)| {
                                                        let col = col.clone();
                                                        let range = max - min;
                                                        let min_s = format!("{min:.2}");
                                                        let max_s = format!("{max:.2}");
                                                        let mean_s = format!("{mean:.2}");
                                                        let std_s = format!("{std:.3}");
                                                        let range_s = format!("{range:.2}");
                                                        view! {
                                                            <tr>
                                                                <td class="text-bold">{col}</td>
                                                                <td style="text-align: right; font-variant-numeric: tabular-nums;">{min_s}</td>
                                                                <td style="text-align: right; font-variant-numeric: tabular-nums;">{max_s}</td>
                                                                <td style="text-align: right; font-variant-numeric: tabular-nums;">{mean_s}</td>
                                                                <td style="text-align: right; font-variant-numeric: tabular-nums;">{std_s}</td>
                                                                <td style="text-align: right; font-variant-numeric: tabular-nums;">{range_s}</td>
                                                            </tr>
                                                        }
                                                    }).collect_view()}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }}

                            // Data Preview with pagination + sorting
                            <Card title="Data Preview">
                                <div style="overflow-x: auto;">
                                    {move || {
                                        let pd = preview_data.get();
                                        if pd.is_none() {
                                            return view! {
                                                <div style="padding: 48px; text-align: center; color: #6b7280;">
                                                    "Loading preview..."
                                                </div>
                                            }.into_any();
                                        }
                                        let pd = pd.unwrap();
                                        let headers = pd.get("headers").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                        let data_rows = pd.get("rows").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                        let total_rows = pd.get("total_rows").and_then(|v| v.as_u64()).unwrap_or(0);
                                        let page = pd.get("page").and_then(|v| v.as_u64()).unwrap_or(0);
                                        let total_pages = pd.get("total_pages").and_then(|v| v.as_u64()).unwrap_or(1);

                                        let row_start = page * 100 + 1;
                                        let row_end = ((page + 1) * 100).min(total_rows);

                                        view! {
                                            <div>
                                                <div style="display: flex; justify-content: space-between; align-items: center; padding: 8px 0; margin-bottom: 8px;">
                                                    <span class="text-sm text-muted">
                                                        {format!("Showing rows {row_start}-{row_end} of {total_rows}")}
                                                    </span>
                                                    <span class="text-sm text-muted">
                                                        {move || {
                                                            match sort_col.get() {
                                                                Some(_) => format!("Sorted {}", sort_dir.get()),
                                                                None => "Click column header to sort".to_string(),
                                                            }
                                                        }}
                                                    </span>
                                                </div>

                                                <table class="data-table">
                                                    <thead>
                                                        <tr>
                                                            {headers.iter().enumerate().map(|(i, h)| {
                                                                let col_name = h.as_str().unwrap_or("").to_string();
                                                                view! {
                                                                    <th
                                                                        style="cursor: pointer; user-select: none; white-space: nowrap;"
                                                                        on:click=move |_: web_sys::MouseEvent| {
                                                                            let current_col = sort_col.get();
                                                                            if current_col == Some(i) {
                                                                                let cur = sort_dir.get();
                                                                                sort_dir.set(if cur == "asc" { "desc".into() } else { "asc".into() });
                                                                            } else {
                                                                                sort_col.set(Some(i));
                                                                                sort_dir.set("asc".into());
                                                                            }
                                                                            current_page.set(0);
                                                                            fetch_trigger.update(|v| *v += 1);
                                                                        }
                                                                    >
                                                                        {col_name}
                                                                        {move || {
                                                                            if sort_col.get() == Some(i) {
                                                                                if sort_dir.get() == "asc" { " \u{25B2}" } else { " \u{25BC}" }
                                                                            } else {
                                                                                " \u{25BD}"
                                                                            }
                                                                        }}
                                                                    </th>
                                                                }
                                                            }).collect_view()}
                                                        </tr>
                                                    </thead>
                                                    <tbody>
                                                        {data_rows.iter().map(|row| {
                                                            let cells = row.as_array().cloned().unwrap_or_default();
                                                            view! {
                                                                <tr>
                                                                    {cells.iter().map(|cell| {
                                                                        let val = cell.as_str().unwrap_or("").to_string();
                                                                        view! { <td>{val}</td> }
                                                                    }).collect_view()}
                                                                </tr>
                                                            }
                                                        }).collect_view()}
                                                    </tbody>
                                                </table>

                                                {if total_pages > 1 {
                                                    view! {
                                                        <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0;">
                                                            <span class="text-sm text-muted">
                                                                {format!("Page {} of {}", page + 1, total_pages)}
                                                            </span>
                                                            <div style="display: flex; gap: 8px;">
                                                                <button
                                                                    class="btn btn-ghost btn-sm"
                                                                    disabled=move || current_page.get() == 0
                                                                    on:click=move |_: web_sys::MouseEvent| {
                                                                        current_page.update(|p| *p = p.saturating_sub(1));
                                                                        fetch_trigger.update(|v| *v += 1);
                                                                    }
                                                                >"Previous"</button>
                                                                <button
                                                                    class="btn btn-ghost btn-sm"
                                                                    disabled=move || { current_page.get() as u64 + 1 >= total_pages }
                                                                    on:click=move |_: web_sys::MouseEvent| {
                                                                        current_page.update(|p| *p += 1);
                                                                        fetch_trigger.update(|v| *v += 1);
                                                                    }
                                                                >"Next"</button>
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <div></div> }.into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    }}
                                </div>
                            </Card>
                        </div>
                    }
                }}
            </Show>
        </div>
    }
}
