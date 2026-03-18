// ============================================================================
// File: models.rs
// Description: Models listing page displaying all trained ML models
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use crate::components::*;
use crate::icons;

#[component]
pub fn ModelsPage() -> impl IntoView {
    let models = RwSignal::new(Vec::<serde_json::Value>::new());

    {
        let models = models;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/models").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    models.set(data);
                }
            }
        });
    }

    view! {
        <div>
            <h1 class="page-title">"Models"</h1>
            <p class="page-subtitle">"Trained model registry"</p>

            <Show
                when=move || !models.get().is_empty()
                fallback=|| view! {
                    <div style="padding: 64px; text-align: center;">
                        <div style="color: #C4A484; margin-bottom: 16px;">{icons::icon_package()}</div>
                        <p class="text-muted">"No models trained yet. Start a training job to create your first model."</p>
                    </div>
                }
            >
                <div class="grid-3">
                    {move || models.get().into_iter().map(|model| {
                        let id = model.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let name = model.get("name").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
                        let arch = model.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let domain = model.get("domain").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let params = model.get("parameters").and_then(|v| v.as_u64()).unwrap_or(0);
                        let f1 = model.get("metrics").and_then(|m| m.get("f1")).and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let status = model.get("status").and_then(|v| v.as_str()).unwrap_or("ready").to_string();
                        let badge_status = badge::status_to_badge(&status);
                        let model_id = id.clone();

                        view! {
                            <a href=format!("/models/{model_id}") style="text-decoration: none;">
                                <Card>
                                    <div class="flex-between mb-4">
                                        <div style="width: 40px; height: 40px; border-radius: 8px; background: #F5EDE8; display: flex; align-items: center; justify-content: center; color: #C4A484;">
                                            {icons::icon_brain()}
                                        </div>
                                        <Badge status=badge_status />
                                    </div>
                                    <h3 class="text-bold" style="margin-bottom: 4px; color: #111827;">{name}</h3>
                                    <p class="text-xs text-muted mb-4">{format!("{arch} \u{2022} {domain}")}</p>
                                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px;">
                                        <div>
                                            <span class="text-xs text-muted">"Parameters"</span>
                                            <div class="text-sm text-bold">{format_params(params)}</div>
                                        </div>
                                        <div>
                                            <span class="text-xs text-muted">"F1 Score"</span>
                                            <div class="text-sm text-bold" style="color: #14b8a6;">{format!("{f1:.3}")}</div>
                                        </div>
                                    </div>
                                </Card>
                            </a>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

fn format_params(params: u64) -> String {
    if params < 1000 {
        format!("{params}")
    } else if params < 1_000_000 {
        format!("{:.1}K", params as f64 / 1000.0)
    } else {
        format!("{:.1}M", params as f64 / 1_000_000.0)
    }
}
