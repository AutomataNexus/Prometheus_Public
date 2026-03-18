// ============================================================================
// File: home.rs
// Description: Dashboard home page with system overview metrics and status cards
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use crate::components::*;
use crate::icons;

#[component]
pub fn HomePage() -> impl IntoView {
    let models_count = RwSignal::new(0u32);
    let datasets_count = RwSignal::new(0u32);
    let training_active = RwSignal::new(0u32);
    let training_total = RwSignal::new(0u32);
    let deployed_count = RwSignal::new(0u32);
    let eval_count = RwSignal::new(0u32);
    let agent_count = RwSignal::new("--".to_string());
    let aegis_status = RwSignal::new("checking".to_string());
    let usage = RwSignal::new(None::<serde_json::Value>);
    let loaded = RwSignal::new(false);

    {
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/datasets").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    datasets_count.set(data.len() as u32);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/models").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    models_count.set(data.len() as u32);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/training").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    let active = data.iter().filter(|r| {
                        r.get("status").and_then(|v| v.as_str()) == Some("running")
                    }).count() as u32;
                    training_active.set(active);
                    training_total.set(data.len() as u32);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/deployments").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    let deployed = data.iter().filter(|r| {
                        let s = r.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        s == "ready" || s == "deployed"
                    }).count() as u32;
                    deployed_count.set(deployed);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/evaluations").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    eval_count.set(data.len() as u32);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/agent/history").send().await {
                if let Ok(data) = resp.json::<Vec<serde_json::Value>>().await {
                    agent_count.set(data.len().to_string());
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/system/metrics").send().await {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    let status = data.get("aegis_db_status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("disconnected")
                        .to_string();
                    aegis_status.set(status);
                }
            }
            if let Ok(resp) = crate::api::auth_get("/api/v1/billing/usage").send().await {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    usage.set(Some(data));
                }
            }
            loaded.set(true);
        });
    }

    view! {
        <div>
            <h1 style="font-size: 1.5rem; font-weight: 700; color: #111827; margin-bottom: 4px;">"Dashboard"</h1>
            <p style="font-size: 0.875rem; color: #9ca3af; margin-bottom: 24px;">"Pipeline overview and system health"</p>

            // Metric Cards
            <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; margin-bottom: 24px;">
                <MetricCard
                    label="Models Trained"
                    value=Signal::derive(move || models_count.get().to_string())
                    icon_name="package"
                />
                <MetricCard
                    label="Training Active"
                    value=Signal::derive(move || training_active.get().to_string())
                    icon_name="brain"
                />
                <MetricCard
                    label="Deployed on Edge"
                    value=Signal::derive(move || deployed_count.get().to_string())
                    icon_name="rocket"
                />
                <MetricCard
                    label="Agent Queries"
                    value=Signal::derive(move || agent_count.get())
                    icon_name="bot"
                />
            </div>

            // Pipeline
            <Card title="Pipeline Status">
                <div style="display: flex; align-items: stretch; gap: 0; overflow-x: auto; padding: 8px 0;">
                    <PipelineStage
                        step=1 label="INGEST" detail="Upload Dataset" icon_name="upload"
                        state=Signal::derive(move || {
                            if !loaded.get() { return StageState::Idle; }
                            if datasets_count.get() > 0 { StageState::Complete } else { StageState::Idle }
                        })
                        href="/datasets"
                    />
                    <PipelineConnector active=Signal::derive(move || datasets_count.get() > 0) />
                    <PipelineStage
                        step=2 label="ANALYZE" detail="Gradient AI" icon_name="brain"
                        state=Signal::derive(move || {
                            if !loaded.get() { return StageState::Idle; }
                            if datasets_count.get() > 0 { StageState::Complete } else { StageState::Idle }
                        })
                        href="/datasets"
                    />
                    <PipelineConnector active=Signal::derive(move || training_total.get() > 0) />
                    <PipelineStage
                        step=3 label="TRAIN" detail="AxonML" icon_name="activity"
                        state=Signal::derive(move || {
                            if !loaded.get() { return StageState::Idle; }
                            if training_active.get() > 0 { StageState::Active }
                            else if training_total.get() > 0 { StageState::Complete }
                            else { StageState::Idle }
                        })
                        href="/training"
                    />
                    <PipelineConnector active=Signal::derive(move || eval_count.get() > 0) />
                    <PipelineStage
                        step=4 label="EVALUATE" detail="Metrics" icon_name="chart"
                        state=Signal::derive(move || {
                            if !loaded.get() { return StageState::Idle; }
                            if eval_count.get() > 0 { StageState::Complete } else { StageState::Idle }
                        })
                        href="/evaluation"
                    />
                    <PipelineConnector active=Signal::derive(move || models_count.get() > 0) />
                    <PipelineStage
                        step=5 label="CONVERT" detail="ONNX / HEF" icon_name="package"
                        state=Signal::derive(move || {
                            if !loaded.get() { return StageState::Idle; }
                            if models_count.get() > 0 { StageState::Complete } else { StageState::Idle }
                        })
                        href="/models"
                    />
                    <PipelineConnector active=Signal::derive(move || deployed_count.get() > 0) />
                    <PipelineStage
                        step=6 label="DEPLOY" detail="Edge Device" icon_name="rocket"
                        state=Signal::derive(move || {
                            if !loaded.get() { return StageState::Idle; }
                            if deployed_count.get() > 0 { StageState::Complete } else { StageState::Idle }
                        })
                        href="/deployment"
                    />
                </div>
            </Card>

            // Bottom row: Usage + Aegis-DB
            <div style="display: grid; grid-template-columns: 2fr 1fr; gap: 16px; margin-top: 24px;">
                // Usage & Limits
                <Card title="Your Resources">
                    {move || {
                        if let Some(ref u) = usage.get() {
                            let tier = u.get("tier").and_then(|v| v.as_str()).unwrap_or("free");
                            let tokens_used = u.get("tokens_used").and_then(|v| v.as_u64()).unwrap_or(0);
                            let tokens_limit = u.get("tokens_limit").and_then(|v| v.as_u64()).unwrap_or(1000);
                            let unlimited = u.get("unlimited").and_then(|v| v.as_bool()).unwrap_or(false);
                            let max_datasets = u.get("max_datasets").and_then(|v| v.as_u64()).unwrap_or(3);
                            let max_models = u.get("max_models").and_then(|v| v.as_u64()).unwrap_or(2);
                            let max_deployments = u.get("max_deployments").and_then(|v| v.as_u64()).unwrap_or(1);
                            let max_storage = u.get("max_dataset_size_bytes").and_then(|v| v.as_u64()).unwrap_or(50*1024*1024);
                            let max_trainings = u.get("max_concurrent_trainings").and_then(|v| v.as_u64()).unwrap_or(1);
                            // If unlimited (admin), treat all resource limits as unlimited
                            let max_datasets = if unlimited { u32::MAX as u64 } else { max_datasets };
                            let max_models = if unlimited { u32::MAX as u64 } else { max_models };
                            let max_deployments = if unlimited { u32::MAX as u64 } else { max_deployments };
                            let max_storage = if unlimited { u64::MAX } else { max_storage };
                            let max_trainings = if unlimited { u32::MAX as u64 } else { max_trainings };

                            let ds = datasets_count.get() as u64;
                            let md = models_count.get() as u64;
                            let dp = deployed_count.get() as u64;
                            let ta = training_active.get() as u64;

                            let tier_badge_bg = if unlimited {
                                "rgba(20,184,166,0.12)"
                            } else {
                                match tier {
                                    "enterprise" => "rgba(139,92,246,0.12)",
                                    "pro" => "rgba(59,130,246,0.12)",
                                    _ => "rgba(107,114,128,0.12)",
                                }
                            };
                            let tier_badge_color = if unlimited {
                                "#0d9488"
                            } else {
                                match tier {
                                    "enterprise" => "#7c3aed",
                                    "pro" => "#2563eb",
                                    _ => "#6b7280",
                                }
                            };
                            let tier_display = if unlimited {
                                "Admin"
                            } else {
                                match tier {
                                    "enterprise" => "Enterprise",
                                    "pro" => "Pro",
                                    _ => "Free",
                                }
                            };

                            view! {
                                <div>
                                    // Tier badge
                                    <div style="margin-bottom: 20px; display: flex; align-items: center; gap: 10px;">
                                        <span style=format!(
                                            "display: inline-block; padding: 4px 12px; border-radius: 6px; font-size: 0.75rem; font-weight: 600; background: {}; color: {};",
                                            tier_badge_bg, tier_badge_color
                                        )>
                                            {tier_display.to_uppercase()}
                                        </span>
                                        <span style="font-size: 0.8rem; color: #9ca3af;">"plan"</span>
                                    </div>

                                    // Resource bars grid
                                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px 24px;">
                                        <UsageBar
                                            label="AI Tokens"
                                            used=tokens_used
                                            limit=if unlimited { 0 } else { tokens_limit }
                                            unlimited=unlimited
                                            format_fn=UsageFormat::Number
                                        />
                                        <UsageBar
                                            label="Datasets"
                                            used=ds
                                            limit=if max_datasets == u32::MAX as u64 { 0 } else { max_datasets }
                                            unlimited=max_datasets == u32::MAX as u64
                                            format_fn=UsageFormat::Number
                                        />
                                        <UsageBar
                                            label="Models"
                                            used=md
                                            limit=if max_models == u32::MAX as u64 { 0 } else { max_models }
                                            unlimited=max_models == u32::MAX as u64
                                            format_fn=UsageFormat::Number
                                        />
                                        <UsageBar
                                            label="Deployments"
                                            used=dp
                                            limit=if max_deployments == u32::MAX as u64 { 0 } else { max_deployments }
                                            unlimited=max_deployments == u32::MAX as u64
                                            format_fn=UsageFormat::Number
                                        />
                                        <UsageBar
                                            label="Active Training"
                                            used=ta
                                            limit=if max_trainings == u32::MAX as u64 { 0 } else { max_trainings }
                                            unlimited=max_trainings == u32::MAX as u64
                                            format_fn=UsageFormat::Number
                                        />
                                        <UsageBar
                                            label="Storage"
                                            used=0
                                            limit=if max_storage == u64::MAX { 0 } else { max_storage / (1024*1024) }
                                            unlimited=max_storage == u64::MAX
                                            format_fn=UsageFormat::Megabytes
                                        />
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
                                    <div class="skeleton" style="height: 48px; border-radius: 8px;"></div>
                                    <div class="skeleton" style="height: 48px; border-radius: 8px;"></div>
                                    <div class="skeleton" style="height: 48px; border-radius: 8px;"></div>
                                    <div class="skeleton" style="height: 48px; border-radius: 8px;"></div>
                                </div>
                            }.into_any()
                        }
                    }}
                </Card>

                // Aegis-DB connection
                <Card title="Aegis-DB">
                    <div style="display: flex; flex-direction: column; gap: 12px;">
                        <div style="display: flex; align-items: center; gap: 8px;">
                            {move || {
                                let status = aegis_status.get();
                                if status == "connected" {
                                    view! { <Badge status=BadgeStatus::Online /> }.into_any()
                                } else {
                                    view! { <Badge status=BadgeStatus::Offline /> }.into_any()
                                }
                            }}
                            <span style="font-size: 0.85rem; color: #374151;">"localhost:9091"</span>
                        </div>
                        <div style="font-size: 0.75rem; color: #9ca3af; line-height: 1.5;">
                            "Local Rust database for user accounts, model metadata, training history, and deployment state."
                        </div>
                    </div>
                </Card>
            </div>
        </div>
    }
}

// ── Pipeline sub-components ──────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum StageState {
    Idle,
    Active,
    Complete,
}

#[component]
fn PipelineStage(
    step: u32,
    label: &'static str,
    detail: &'static str,
    icon_name: &'static str,
    state: Signal<StageState>,
    href: &'static str,
) -> impl IntoView {
    let stage_style = move || {
        let base = "display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 16px 20px; border-radius: 12px; min-width: 110px; text-decoration: none; cursor: pointer; transition: all 0.2s ease; border: 1.5px solid ";
        match state.get() {
            StageState::Complete => format!("{}#14b8a6; background: rgba(20,184,166,0.08); color: #0d9488;", base),
            StageState::Active => format!("{}#f59e0b; background: rgba(245,158,11,0.08); color: #d97706;", base),
            StageState::Idle => format!("{}#e8d4c4; background: #faf8f5; color: #9ca3af;", base),
        }
    };

    let icon_bg = move || {
        match state.get() {
            StageState::Complete => "width: 36px; height: 36px; border-radius: 50%; background: rgba(20,184,166,0.15); display: flex; align-items: center; justify-content: center; color: #0d9488;",
            StageState::Active => "width: 36px; height: 36px; border-radius: 50%; background: rgba(245,158,11,0.15); display: flex; align-items: center; justify-content: center; color: #d97706;",
            StageState::Idle => "width: 36px; height: 36px; border-radius: 50%; background: #f0ebe6; display: flex; align-items: center; justify-content: center; color: #b8a898;",
        }
    };

    let step_indicator = move || {
        match state.get() {
            StageState::Complete => view! {
                <div style="color: #14b8a6;">{icons::icon_check()}</div>
            }.into_any(),
            _ => view! {
                <div style="font-size: 0.65rem; font-weight: 600; opacity: 0.5;">{format!("{}", step)}</div>
            }.into_any(),
        }
    };

    view! {
        <a href=href style=stage_style>
            <div style=icon_bg>
                <Icon name=icon_name.to_string() size=18 />
            </div>
            <span style="font-size: 0.7rem; font-weight: 700; letter-spacing: 0.08em;">{label}</span>
            <span style="font-size: 0.7rem; opacity: 0.7;">{detail}</span>
            {step_indicator}
        </a>
    }
}

#[component]
fn PipelineConnector(active: Signal<bool>) -> impl IntoView {
    let style = move || {
        let color = if active.get() { "#14b8a6" } else { "#d4c4b4" };
        format!(
            "display: flex; align-items: center; padding: 0 4px; color: {}; align-self: center;",
            color
        )
    };
    view! {
        <div style=style>
            <svg width="24" height="12" viewBox="0 0 24 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M0 6H20M20 6L14 1M20 6L14 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
        </div>
    }
}

// ── Usage bar component ──────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum UsageFormat {
    Number,
    Megabytes,
}

#[component]
fn UsageBar(
    label: &'static str,
    used: u64,
    limit: u64,
    unlimited: bool,
    format_fn: UsageFormat,
) -> impl IntoView {
    let pct = if unlimited || limit == 0 { 0.0 } else { (used as f64 / limit as f64 * 100.0).min(100.0) };
    let bar_color = if pct > 90.0 { "#dc2626" } else if pct > 70.0 { "#f59e0b" } else { "#14b8a6" };

    let used_str = match format_fn {
        UsageFormat::Number => format!("{}", used),
        UsageFormat::Megabytes => format!("{} MB", used),
    };
    let limit_str = if unlimited {
        "\u{221E}".to_string()
    } else {
        match format_fn {
            UsageFormat::Number => format!("{}", limit),
            UsageFormat::Megabytes => format!("{} MB", limit),
        }
    };

    view! {
        <div>
            <div style="display: flex; justify-content: space-between; margin-bottom: 4px;">
                <span style="font-size: 0.75rem; font-weight: 500; color: #374151;">{label}</span>
                <span style="font-size: 0.75rem; color: #9ca3af;">
                    {used_str}" / "{limit_str}
                </span>
            </div>
            <div style="height: 6px; background: #f0ebe6; border-radius: 3px; overflow: hidden;">
                <div style=format!(
                    "height: 100%; width: {}%; background: {}; border-radius: 3px; transition: width 0.4s ease;",
                    if unlimited { 0.0 } else { pct }, bar_color
                )></div>
            </div>
        </div>
    }
}
