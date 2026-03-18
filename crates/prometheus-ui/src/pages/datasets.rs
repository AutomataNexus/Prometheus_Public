// ============================================================================
// File: datasets.rs
// Description: Dataset listing page with file upload, catalog browser, and connect source
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 9, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use leptos::callback::Callback;
use crate::components::*;
use crate::components::toast::{ToastLevel, ToastMessage, push_toast};
use crate::icons;

#[component]
pub fn DatasetsPage() -> impl IntoView {
    let datasets = RwSignal::new(Vec::<serde_json::Value>::new());
    let show_upload = RwSignal::new(false);
    let show_catalog = RwSignal::new(false);
    let show_connect = RwSignal::new(false);
    let uploading = RwSignal::new(false);
    let upload_progress = RwSignal::new(0.0f64);
    let upload_filename = RwSignal::new(String::new());

    // Catalog state
    let catalog = RwSignal::new(Vec::<serde_json::Value>::new());
    let catalog_loading = RwSignal::new(false);
    let importing = RwSignal::new(false);

    // Connect source state
    let connect_source_type = RwSignal::new("aegis_bridge".to_string());
    let connect_host = RwSignal::new(String::new());
    let connect_port = RwSignal::new("9090".to_string());
    let connect_collection = RwSignal::new(String::new());
    let connect_database = RwSignal::new(String::new());
    let connect_query = RwSignal::new(String::new());
    let connect_token = RwSignal::new(String::new());
    let connecting = RwSignal::new(false);
    let save_connection = RwSignal::new(false);
    let connection_name = RwSignal::new(String::new());
    let saved_connections = RwSignal::new(Vec::<serde_json::Value>::new());

    // Fetch datasets on mount
    {
        let datasets = datasets;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/datasets")
                .send()
                .await
            {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    datasets.set(data);
                }
            }
        });
    }

    let refresh_datasets = move || {
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/datasets")
                .send()
                .await
            {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    datasets.set(data);
                }
            }
        });
    };

    let on_file_selected = Callback::new(move |file: web_sys::File| {
        upload_filename.set(file.name());
        uploading.set(true);
        upload_progress.set(0.0);

        leptos::task::spawn_local(async move {
            upload_progress.set(50.0);

            let form_data = web_sys::FormData::new().unwrap();
            let _ = form_data.append_with_blob("file", &file);

            match crate::api::auth_post("/api/v1/datasets")
                .body(form_data)
                .unwrap()
                .send()
                .await
            {
                Ok(resp) if resp.ok() => {
                    upload_progress.set(100.0);
                    uploading.set(false);
                    show_upload.set(false);
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                        push_toast(set_toasts, ToastLevel::Success, "Dataset uploaded successfully");
                    }
                    if let Ok(resp) = crate::api::auth_get("/api/v1/datasets")
                        .send()
                        .await
                    {
                        if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                            datasets.set(data);
                        }
                    }
                }
                _ => {
                    uploading.set(false);
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                        push_toast(set_toasts, ToastLevel::Error, "Dataset upload failed");
                    }
                }
            }
        });
    });

    // Load catalog
    let load_catalog = move |_| {
        show_catalog.set(true);
        catalog_loading.set(true);
        leptos::task::spawn_local(async move {
            match crate::api::auth_get("/api/v1/datasets/catalog")
                .send()
                .await
            {
                Ok(resp) if resp.ok() => {
                    if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                        catalog.set(data);
                    }
                }
                _ => {
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                        push_toast(set_toasts, ToastLevel::Error, "Failed to load dataset catalog");
                    }
                }
            }
            catalog_loading.set(false);
        });
    };

    // Connect source handler
    let on_connect = move |_| {
        connecting.set(true);
        let src_type = connect_source_type.get();
        let host = connect_host.get();
        let port = connect_port.get();
        let collection = connect_collection.get();
        let database = connect_database.get();
        let query = connect_query.get();
        let token = connect_token.get();

        leptos::task::spawn_local(async move {
            let mut body = serde_json::Map::new();
            body.insert("source_type".into(), serde_json::Value::String(src_type.clone()));

            match src_type.as_str() {
                "aegis_bridge" => {
                    body.insert("controller_ip".into(), serde_json::Value::String(host));
                    body.insert("aegis_port".into(), serde_json::Value::String(port));
                    body.insert("collection".into(), serde_json::Value::String(collection));
                }
                "influxdb" => {
                    body.insert("host".into(), serde_json::Value::String(host));
                    body.insert("database".into(), serde_json::Value::String(database));
                    body.insert("token".into(), serde_json::Value::String(token));
                    body.insert("query".into(), serde_json::Value::String(query));
                }
                "postgresql" | "tidb" => {
                    body.insert("host".into(), serde_json::Value::String(host));
                    body.insert("port".into(), serde_json::Value::String(port));
                    body.insert("database".into(), serde_json::Value::String(database));
                    body.insert("query".into(), serde_json::Value::String(query));
                }
                "mongodb" => {
                    body.insert("endpoint".into(), serde_json::Value::String(host));
                    body.insert("api_key".into(), serde_json::Value::String(token));
                    body.insert("database".into(), serde_json::Value::String(database));
                    body.insert("collection".into(), serde_json::Value::String(collection));
                }
                "sqlite" => {
                    body.insert("file_path".into(), serde_json::Value::String(host));
                    body.insert("query".into(), serde_json::Value::String(query));
                }
                "spacetimedb" => {
                    body.insert("host".into(), serde_json::Value::String(host));
                    body.insert("database".into(), serde_json::Value::String(database));
                    body.insert("query".into(), serde_json::Value::String(query));
                }
                _ => {}
            }

            // Optionally save connection for future reuse (encrypted server-side)
            if save_connection.get_untracked() {
                body.insert("save_connection".into(), serde_json::Value::Bool(true));
                let cname = connection_name.get_untracked();
                if !cname.is_empty() {
                    body.insert("connection_name".into(), serde_json::Value::String(cname));
                }
            }

            let json_body = serde_json::Value::Object(body);
            match crate::api::auth_post("/api/v1/datasets/connect")
                .header("Content-Type", "application/json")
                .body(json_body.to_string())
                .unwrap()
                .send()
                .await
            {
                Ok(resp) if resp.ok() => {
                    show_connect.set(false);
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                        push_toast(set_toasts, ToastLevel::Success, "Data source connected successfully");
                    }
                    // Refresh
                    if let Ok(resp) = crate::api::auth_get("/api/v1/datasets")
                        .send()
                        .await
                    {
                        if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                            datasets.set(data);
                        }
                    }
                }
                Ok(resp) => {
                    let msg = resp.text().await.unwrap_or_else(|_| "Connection failed".into());
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                        push_toast(set_toasts, ToastLevel::Error, &msg);
                    }
                }
                Err(e) => {
                    if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                        push_toast(set_toasts, ToastLevel::Error, &format!("Connection failed: {e}"));
                    }
                }
            }
            connecting.set(false);
        });
    };

    let table_columns = vec![
        Column { key: "name".into(), label: "Name".into(), sortable: true },
        Column { key: "domain".into(), label: "Domain".into(), sortable: true },
        Column { key: "rows".into(), label: "Rows".into(), sortable: true },
        Column { key: "columns".into(), label: "Columns".into(), sortable: true },
        Column { key: "source".into(), label: "Source".into(), sortable: true },
        Column { key: "tags".into(), label: "Tags".into(), sortable: false },
        Column { key: "size".into(), label: "Size".into(), sortable: true },
        Column { key: "status".into(), label: "Status".into(), sortable: true },
    ];

    let table_rows: Signal<Vec<Vec<String>>> = Signal::derive(move || {
        datasets.get().iter().map(|ds| {
            let str_field = |key: &str| -> String {
                ds.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let tags = ds.get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", "))
                .unwrap_or_default();
            let col_count = ds.get("columns")
                .and_then(|v| v.as_array())
                .map(|a| a.len().to_string())
                .unwrap_or_default();
            let source = ds.get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("upload")
                .to_string();
            let compressed = ds.get("compressed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            vec![
                str_field("name"),
                str_field("domain"),
                ds.get("row_count").and_then(|v| v.as_u64()).map(|v| v.to_string()).unwrap_or_default(),
                col_count,
                source,
                tags,
                format_bytes(ds.get("file_size_bytes").and_then(|v| v.as_u64()).unwrap_or(0)),
                {
                    let s = ds.get("status").and_then(|v| v.as_str()).unwrap_or("active");
                    if s == "paused" {
                        "Paused".to_string()
                    } else if compressed {
                        "Compressed".to_string()
                    } else {
                        "Active".to_string()
                    }
                },
            ]
        }).collect()
    });

    let on_row_click = Callback::new(move |idx: usize| {
        let ds_list = datasets.get();
        if let Some(ds) = ds_list.get(idx) {
            if let Some(id) = ds.get("id").and_then(|v| v.as_str()) {
                if let Some(window) = web_sys::window() {
                    let _ = window.location().set_href(&format!("/datasets/{id}"));
                }
            }
        }
    });

    view! {
        <div>
            <div class="flex-between mb-8">
                <div>
                    <h1 class="page-title">"Datasets"</h1>
                    <p class="page-subtitle">"Upload data, browse the catalog, or connect external sources"</p>
                </div>
                <div style="display:flex;gap:8px;">
                    <button class="btn btn-secondary" on:click=load_catalog>
                        {icons::icon_database()}
                        " Browse Catalog"
                    </button>
                    <button class="btn btn-secondary" on:click=move |_| {
                        show_connect.set(true);
                        leptos::task::spawn_local(async move {
                            if let Ok(resp) = crate::api::auth_get("/api/v1/connections").send().await {
                                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                                    saved_connections.set(data);
                                }
                            }
                        });
                    }>
                        {icons::icon_activity()}
                        " Connect Source"
                    </button>
                    <button class="btn btn-primary" on:click=move |_| show_upload.set(true)>
                        {icons::icon_upload()}
                        " Upload Dataset"
                    </button>
                </div>
            </div>

            // ── Upload Panel ─────────────────────────────────────────
            <Show when=move || show_upload.get()>
                <Card title="Upload Dataset" class="mb-8">
                    <Show
                        when=move || !uploading.get()
                        fallback=move || view! {
                            <UploadProgress
                                filename=Signal::derive(move || upload_filename.get())
                                progress=Signal::derive(move || upload_progress.get())
                            />
                        }
                    >
                        <FileUpload
                            accept=".csv"
                            max_size_mb=100
                            on_file=on_file_selected
                        />
                    </Show>
                    <div style="margin-top:12px;text-align:right;">
                        <button class="btn btn-ghost" on:click=move |_| show_upload.set(false)>"Cancel"</button>
                    </div>
                </Card>
            </Show>

            // ── Dataset Catalog ──────────────────────────────────────
            <Show when=move || show_catalog.get()>
                <Card title="Dataset Catalog" class="mb-8">
                    <p style="color:var(--text-secondary);margin-bottom:16px;">
                        "Pre-loaded datasets across 30 domains. Click "
                        <strong>"Import"</strong>
                        " to add to your workspace. Data stays compressed until training."
                    </p>
                    <Show
                        when=move || !catalog_loading.get()
                        fallback=move || view! {
                            <div style="text-align:center;padding:32px;color:var(--text-secondary);">
                                {icons::icon_loader()}
                                " Loading catalog..."
                            </div>
                        }
                    >
                        {move || {
                            let cat = catalog.get();
                            if cat.is_empty() {
                                view! {
                                    <p style="color:var(--text-secondary);text-align:center;padding:24px;">
                                        "No catalog datasets found on this server."
                                    </p>
                                }.into_any()
                            } else {
                                view! {
                                    <div style="max-height:500px;overflow-y:auto;">
                                        {cat.into_iter().map(|domain_entry| {
                                            let domain = domain_entry.get("domain")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("unknown")
                                                .to_string();
                                            let count = domain_entry.get("dataset_count")
                                                .and_then(|v| v.as_u64())
                                                .unwrap_or(0);
                                            let total_size = domain_entry.get("total_size")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("0 B")
                                                .to_string();
                                            let items = domain_entry.get("datasets")
                                                .and_then(|v| v.as_array())
                                                .cloned()
                                                .unwrap_or_default();

                                            let domain_display = domain.clone();
                                            let expanded = RwSignal::new(false);

                                            view! {
                                                <div style="border:1px solid var(--border);border-radius:8px;margin-bottom:8px;overflow:hidden;">
                                                    <button
                                                        style="width:100%;padding:12px 16px;display:flex;justify-content:space-between;align-items:center;background:var(--surface);border:none;cursor:pointer;color:var(--text-primary);font-size:14px;"
                                                        on:click=move |_| expanded.set(!expanded.get())
                                                    >
                                                        <span style="font-weight:600;text-transform:capitalize;">
                                                            {domain_display.clone()}
                                                        </span>
                                                        <span style="color:var(--text-secondary);font-size:13px;">
                                                            {format!("{count} datasets · {total_size}")}
                                                        </span>
                                                    </button>
                                                    <Show when=move || expanded.get()>
                                                        <div style="border-top:1px solid var(--border);padding:8px;">
                                                            {items.clone().into_iter().map(|item| {
                                                                let name = item.get("name")
                                                                    .and_then(|v| v.as_str())
                                                                    .unwrap_or("unknown")
                                                                    .to_string();
                                                                let file_type = item.get("file_type")
                                                                    .and_then(|v| v.as_str())
                                                                    .unwrap_or("csv")
                                                                    .to_string();
                                                                let size = item.get("file_size")
                                                                    .and_then(|v| v.as_str())
                                                                    .unwrap_or("0 B")
                                                                    .to_string();
                                                                let rows = item.get("row_count")
                                                                    .and_then(|v| v.as_u64());
                                                                let path = item.get("path")
                                                                    .and_then(|v| v.as_str())
                                                                    .unwrap_or("")
                                                                    .to_string();
                                                                let description = item.get("description")
                                                                    .and_then(|v| v.as_str())
                                                                    .unwrap_or("")
                                                                    .to_string();
                                                                let columns_preview = item.get("columns_preview")
                                                                    .and_then(|v| v.as_str())
                                                                    .unwrap_or("")
                                                                    .to_string();
                                                                let item_domain = domain.clone();
                                                                let import_name = name.clone();
                                                                let import_path = path.clone();
                                                                let import_type = file_type.clone();
                                                                let display_name = name.clone();
                                                                let display_type = file_type.clone();

                                                                let row_info = rows.map(|r| format!(" · {r} rows")).unwrap_or_default();

                                                                view! {
                                                                    <div style="padding:8px 12px;border-radius:6px;margin:2px 0;border-bottom:1px solid rgba(232,212,196,0.3);">
                                                                        <div style="display:flex;justify-content:space-between;align-items:flex-start;">
                                                                        <div style="flex:1;min-width:0;">
                                                                            <span style="font-weight:500;color:var(--text-primary);">
                                                                                {display_name}
                                                                            </span>
                                                                            <span style="color:var(--text-secondary);font-size:12px;margin-left:8px;">
                                                                                {format!("{display_type} · {size}{row_info}")}
                                                                            </span>
                                                                            {if !description.is_empty() {
                                                                                Some(view! { <div style="font-size:0.75rem;color:#6b7280;margin-top:2px;">{description}</div> })
                                                                            } else { None }}
                                                                            {if !columns_preview.is_empty() {
                                                                                Some(view! { <div style="font-size:0.68rem;color:#9ca3af;margin-top:1px;font-family:monospace;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:500px;">{columns_preview}</div> })
                                                                            } else { None }}
                                                                        </div>
                                                                        <button
                                                                            class="btn btn-primary"
                                                                            style="padding:4px 12px;font-size:12px;"
                                                                            disabled=move || importing.get()
                                                                            on:click=move |_| {
                                                                                let p = import_path.clone();
                                                                                let n = import_name.clone();
                                                                                let d = item_domain.clone();
                                                                                let ft = import_type.clone();
                                                                                importing.set(true);
                                                                                leptos::task::spawn_local(async move {
                                                                                    let body = serde_json::json!({
                                                                                        "path": p,
                                                                                        "name": n,
                                                                                        "domain": d,
                                                                                        "file_type": ft,
                                                                                    });
                                                                                    match crate::api::auth_post("/api/v1/datasets/catalog/import")
                                                                                        .header("Content-Type", "application/json")
                                                                                        .body(body.to_string())
                                                                                        .unwrap()
                                                                                        .send()
                                                                                        .await
                                                                                    {
                                                                                        Ok(resp) if resp.ok() => {
                                                                                            if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                                                                                push_toast(set_toasts, ToastLevel::Success, &format!("Imported: {}", n.clone()));
                                                                                            }
                                                                                            // Refresh dataset list
                                                                                            if let Ok(resp) = crate::api::auth_get("/api/v1/datasets").send().await {
                                                                                                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                                                                                                    datasets.set(data);
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        _ => {
                                                                                            if let Some(set_toasts) = use_context::<WriteSignal<Vec<ToastMessage>>>() {
                                                                                                push_toast(set_toasts, ToastLevel::Error, "Import failed");
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    importing.set(false);
                                                                                });
                                                                            }
                                                                        >
                                                                            {move || if importing.get() { "Importing..." } else { "Import" }}
                                                                        </button>
                                                                    </div>
                                                                    </div>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                    </Show>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            }
                        }}
                    </Show>
                    <div style="margin-top:12px;text-align:right;">
                        <button class="btn btn-ghost" on:click=move |_| show_catalog.set(false)>"Close"</button>
                    </div>
                </Card>
            </Show>

            // ── Connect Source Panel ─────────────────────────────────
            <Show when=move || show_connect.get()>
                <Card title="Connect External Data Source" class="mb-8">
                    <div style="display:grid;gap:16px;">
                        <div>
                            <label style="display:block;font-weight:500;margin-bottom:4px;color:var(--text-primary);">"Source Type"</label>
                            <select
                                style="width:100%;padding:8px 12px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-primary);"
                                on:change=move |e| {
                                    use wasm_bindgen::JsCast;
                                    let target = web_sys::EventTarget::from(e.target().unwrap()).unchecked_into::<web_sys::HtmlSelectElement>();
                                    connect_source_type.set(target.value());
                                }
                            >
                                <option value="aegis_bridge">"AegisBridge (Edge Controller)"</option>
                                <option value="influxdb">"InfluxDB"</option>
                                <option value="postgresql">"PostgreSQL"</option>
                                <option value="tidb">"TiDB"</option>
                                <option value="mongodb">"MongoDB (Data API)"</option>
                                <option value="sqlite">"SQLite3"</option>
                                <option value="spacetimedb">"SpaceTimeDB"</option>
                            </select>
                        </div>
                        <div>
                            <label style="display:block;font-weight:500;margin-bottom:4px;color:var(--text-primary);">
                                {move || match connect_source_type.get().as_str() {
                                    "aegis_bridge" => "Controller IP",
                                    "sqlite" => "File Path",
                                    "mongodb" => "Data API Endpoint",
                                    _ => "Host",
                                }}
                            </label>
                            <input
                                type="text"
                                style="width:100%;padding:8px 12px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-primary);"
                                placeholder=move || match connect_source_type.get().as_str() {
                                    "aegis_bridge" => "192.168.1.100",
                                    "sqlite" => "/data/mydb.sqlite3",
                                    "mongodb" => "https://data.mongodb-api.com/app/...",
                                    _ => "db.example.com",
                                }
                                on:input=move |e| {
                                    use wasm_bindgen::JsCast;
                                    let target = web_sys::EventTarget::from(e.target().unwrap()).unchecked_into::<web_sys::HtmlInputElement>();
                                    connect_host.set(target.value());
                                }
                            />
                        </div>
                        <div style={move || if matches!(connect_source_type.get().as_str(), "aegis_bridge" | "postgresql" | "tidb") { "display:block" } else { "display:none" }}>
                            <label style="display:block;font-weight:500;margin-bottom:4px;color:var(--text-primary);">"Port"</label>
                            <input
                                type="text"
                                style="width:100%;padding:8px 12px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-primary);"
                                prop:value=move || connect_port.get()
                                on:input=move |e| {
                                    use wasm_bindgen::JsCast;
                                    let target = web_sys::EventTarget::from(e.target().unwrap()).unchecked_into::<web_sys::HtmlInputElement>();
                                    connect_port.set(target.value());
                                }
                            />
                        </div>
                        <div style={move || if matches!(connect_source_type.get().as_str(), "sqlite") { "display:none" } else { "display:block" }}>
                            <label style="display:block;font-weight:500;margin-bottom:4px;color:var(--text-primary);">
                                {move || if connect_source_type.get() == "aegis_bridge" { "Collection" } else { "Database" }}
                            </label>
                            <input
                                type="text"
                                style="width:100%;padding:8px 12px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-primary);"
                                placeholder=move || if connect_source_type.get() == "aegis_bridge" { "sensor_readings" } else { "mydb" }
                                on:input=move |e| {
                                    use wasm_bindgen::JsCast;
                                    let target = web_sys::EventTarget::from(e.target().unwrap()).unchecked_into::<web_sys::HtmlInputElement>();
                                    let val = target.value();
                                    if connect_source_type.get() == "aegis_bridge" || connect_source_type.get() == "mongodb" {
                                        connect_collection.set(val);
                                    } else {
                                        connect_database.set(val);
                                    }
                                }
                            />
                        </div>
                        <div style={move || if matches!(connect_source_type.get().as_str(), "influxdb" | "mongodb") { "display:block" } else { "display:none" }}>
                            <label style="display:block;font-weight:500;margin-bottom:4px;color:var(--text-primary);">"Token / API Key"</label>
                            <input
                                type="password"
                                style="width:100%;padding:8px 12px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-primary);"
                                on:input=move |e| {
                                    use wasm_bindgen::JsCast;
                                    let target = web_sys::EventTarget::from(e.target().unwrap()).unchecked_into::<web_sys::HtmlInputElement>();
                                    connect_token.set(target.value());
                                }
                            />
                        </div>
                        <div style={move || if matches!(connect_source_type.get().as_str(), "aegis_bridge") { "display:none" } else { "display:block" }}>
                            <label style="display:block;font-weight:500;margin-bottom:4px;color:var(--text-primary);">"Query"</label>
                            <textarea
                                style="width:100%;padding:8px 12px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-primary);min-height:80px;font-family:monospace;font-size:13px;"
                                placeholder="SELECT * FROM data WHERE ..."
                                on:input=move |e| {
                                    use wasm_bindgen::JsCast;
                                    let target = web_sys::EventTarget::from(e.target().unwrap()).unchecked_into::<web_sys::HtmlTextAreaElement>();
                                    connect_query.set(target.value());
                                }
                            ></textarea>
                        </div>
                        // Save connection checkbox + name
                        <div style="padding:12px;border:1px solid var(--border);border-radius:6px;background:var(--surface-alt,var(--surface));">
                            <label style="display:flex;align-items:center;gap:8px;cursor:pointer;color:var(--text-primary);">
                                <input
                                    type="checkbox"
                                    on:change=move |e| {
                                        use wasm_bindgen::JsCast;
                                        let target = web_sys::EventTarget::from(e.target().unwrap()).unchecked_into::<web_sys::HtmlInputElement>();
                                        save_connection.set(target.checked());
                                    }
                                />
                                <span style="font-weight:500;">"Save this connection for future use"</span>
                            </label>
                            <p style="margin:4px 0 0 28px;font-size:12px;color:var(--text-muted);">"Credentials are encrypted with AES-256-GCM. Prometheus cannot read your raw credentials."</p>
                            <div style={move || if save_connection.get() { "display:block;margin-top:8px;margin-left:28px;" } else { "display:none" }}>
                                <input
                                    type="text"
                                    style="width:100%;padding:6px 10px;border:1px solid var(--border);border-radius:4px;background:var(--surface);color:var(--text-primary);font-size:13px;"
                                    placeholder="Connection name (e.g. Production MongoDB)"
                                    on:input=move |e| {
                                        use wasm_bindgen::JsCast;
                                        let target = web_sys::EventTarget::from(e.target().unwrap()).unchecked_into::<web_sys::HtmlInputElement>();
                                        connection_name.set(target.value());
                                    }
                                />
                            </div>
                        </div>

                        // Saved connections list
                        <div style={move || if saved_connections.get().is_empty() { "display:none" } else { "display:block" }}>
                            <label style="display:block;font-weight:500;margin-bottom:4px;color:var(--text-primary);">"Saved Connections"</label>
                            <div style="display:flex;flex-direction:column;gap:6px;">
                                {move || saved_connections.get().iter().map(|conn| {
                                    let name = conn.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed").to_string();
                                    let src = conn.get("source_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let id = conn.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let id_use = id.clone();
                                    let id_del = id.clone();
                                    view! {
                                        <div style="display:flex;align-items:center;justify-content:space-between;padding:8px 12px;border:1px solid var(--border);border-radius:6px;background:var(--surface);">
                                            <div>
                                                <span style="font-weight:500;color:var(--text-primary);">{name}</span>
                                                <span style="margin-left:8px;font-size:12px;color:var(--text-muted);">{src}</span>
                                            </div>
                                            <div style="display:flex;gap:6px;">
                                                <button
                                                    class="btn btn-sm btn-primary"
                                                    on:click=move |_| {
                                                        let cid = id_use.clone();
                                                        leptos::task::spawn_local(async move {
                                                            let _ = crate::api::auth_post(&format!("/api/v1/connections/{}/use", cid))
                                                                .header("Content-Type", "application/json")
                                                                .body("{}")
                                                                .unwrap()
                                                                .send()
                                                                .await;
                                                        });
                                                    }
                                                >"Use"</button>
                                                <button
                                                    class="btn btn-sm btn-ghost"
                                                    style="color:var(--error);"
                                                    on:click=move |_| {
                                                        let cid = id_del.clone();
                                                        leptos::task::spawn_local(async move {
                                                            let _ = crate::api::auth_delete(&format!("/api/v1/connections/{}", cid))
                                                                .send()
                                                                .await;
                                                        });
                                                    }
                                                >"Delete"</button>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>

                        <div style="display:flex;gap:8px;justify-content:flex-end;">
                            <button class="btn btn-ghost" on:click=move |_| show_connect.set(false)>"Cancel"</button>
                            <button
                                class="btn btn-primary"
                                disabled=move || connecting.get()
                                on:click=on_connect
                            >
                                {move || if connecting.get() { "Connecting..." } else { "Connect" }}
                            </button>
                        </div>
                    </div>
                </Card>
            </Show>

            <div class="mb-8">
                <IngestionKeysPanel />
            </div>

            <Card title="All Datasets">
                <DataTable
                    columns=table_columns
                    rows=table_rows
                    on_row_click=on_row_click
                    empty_message="No datasets yet. Upload a file, browse the catalog, or connect an external source."
                />
            </Card>
        </div>
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
