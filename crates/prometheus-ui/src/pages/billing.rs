// ============================================================================
// File: billing.rs
// Description: Billing and subscription management page with Stripe integration
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use crate::components::card::Card;

#[component]
pub fn BillingPage() -> impl IntoView {
    let usage = RwSignal::new(None::<serde_json::Value>);
    let loading = RwSignal::new(true);
    let checkout_loading = RwSignal::new(None::<String>);
    let portal_loading = RwSignal::new(false);
    let toast_msg = RwSignal::new(None::<(String, bool)>);

    // Check URL params for success/cancel
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    if search.contains("success=true") {
        toast_msg.set(Some(("Subscription activated! Your plan has been upgraded.".into(), true)));
    } else if search.contains("canceled=true") {
        toast_msg.set(Some(("Checkout canceled. No changes were made.".into(), false)));
    }

    // Fetch usage on mount
    {
        let usage = usage;
        let loading = loading;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::auth_get("/api/v1/billing/usage").send().await {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    usage.set(Some(data));
                }
            }
            loading.set(false);
        });
    }

    let current_tier = move || {
        usage.get()
            .and_then(|u| u.get("tier").and_then(|t| t.as_str().map(String::from)))
            .unwrap_or_else(|| "free".to_string())
    };

    let tokens_used = move || {
        usage.get().and_then(|u| u.get("tokens_used").and_then(|v| v.as_u64())).unwrap_or(0)
    };

    let tokens_limit = move || {
        usage.get().and_then(|u| u.get("tokens_limit").and_then(|v| v.as_u64())).unwrap_or(1000)
    };

    let pct_used = move || {
        usage.get().and_then(|u| u.get("percentage_used").and_then(|v| v.as_f64())).unwrap_or(0.0)
    };

    let is_unlimited = move || {
        usage.get().and_then(|u| u.get("unlimited").and_then(|v| v.as_bool())).unwrap_or(false)
    };

    let max_datasets = move || {
        usage.get().and_then(|u| u.get("max_datasets").and_then(|v| v.as_u64())).unwrap_or(3)
    };
    let max_models = move || {
        usage.get().and_then(|u| u.get("max_models").and_then(|v| v.as_u64())).unwrap_or(2)
    };
    let max_trainings = move || {
        usage.get().and_then(|u| u.get("max_concurrent_trainings").and_then(|v| v.as_u64())).unwrap_or(1)
    };
    let max_deployments = move || {
        usage.get().and_then(|u| u.get("max_deployments").and_then(|v| v.as_u64())).unwrap_or(1)
    };

    let start_checkout = move |tier: &'static str| {
        let checkout_loading = checkout_loading;
        let toast_msg = toast_msg;
        checkout_loading.set(Some(tier.to_string()));
        leptos::task::spawn_local(async move {
            let body = serde_json::json!({ "tier": tier });
            match crate::api::auth_post("/api/v1/billing/checkout")
                .json(&body)
                .unwrap()
                .send()
                .await
            {
                Ok(resp) if resp.ok() => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(url) = data.get("checkout_url").and_then(|v| v.as_str()) {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href(url);
                            }
                            return;
                        }
                    }
                    toast_msg.set(Some(("Failed to create checkout session".into(), false)));
                }
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    toast_msg.set(Some((format!("Checkout error: {text}"), false)));
                }
                Err(e) => {
                    toast_msg.set(Some((format!("Network error: {e}"), false)));
                }
            }
            checkout_loading.set(None);
        });
    };

    let open_portal = move |_| {
        portal_loading.set(true);
        leptos::task::spawn_local(async move {
            match crate::api::auth_post("/api/v1/billing/portal").send().await {
                Ok(resp) if resp.ok() => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(url) = data.get("portal_url").and_then(|v| v.as_str()) {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href(url);
                            }
                            return;
                        }
                    }
                    toast_msg.set(Some(("Failed to open billing portal".into(), false)));
                }
                _ => {
                    toast_msg.set(Some(("Could not open billing portal. You may need an active subscription first.".into(), false)));
                }
            }
            portal_loading.set(false);
        });
    };

    view! {
        <div>
            <h1 class="page-title">"Billing & Subscriptions"</h1>
            <p class="page-subtitle">"Manage your plan, usage, and sponsorships"</p>

            // Toast
            {move || toast_msg.get().map(|(msg, success)| {
                let bg = if success { "#22c55e" } else { "#dc2626" };
                view! {
                    <div style=format!(
                        "padding: 12px 20px; border-radius: 8px; color: white; background: {bg}; margin-bottom: 20px; display: flex; justify-content: space-between; align-items: center;"
                    )>
                        <span>{msg}</span>
                        <button
                            style="background: none; border: none; color: white; cursor: pointer; font-size: 18px;"
                            on:click=move |_| toast_msg.set(None)
                        >"×"</button>
                    </div>
                }
            })}

            // Loading
            {move || loading.get().then(|| view! {
                <div style="text-align: center; padding: 60px;">
                    <div class="loader"></div>
                    <p class="text-muted" style="margin-top: 12px;">"Loading billing info..."</p>
                </div>
            })}

            // Content (when loaded)
            {move || (!loading.get()).then(|| {
                let tier = current_tier();
                let tier_display = match tier.as_str() {
                    "basic" => "Basic",
                    "pro" => "Pro",
                    "enterprise" => "Enterprise",
                    _ => "Free",
                };
                let tier_color = match tier.as_str() {
                    "basic" => "#3b82f6",
                    "pro" => "#8b5cf6",
                    "enterprise" => "#C2714F",
                    _ => "#6b7280",
                };

                let used = tokens_used();
                let limit = tokens_limit();
                let pct = pct_used();
                let unlimited = is_unlimited();
                let bar_color = if pct > 90.0 { "#dc2626" } else if pct > 70.0 { "#f97316" } else { "#14b8a6" };
                let bar_width = if pct > 100.0 { 100.0 } else { pct };

                let is_free = tier == "free";
                let is_basic = tier == "basic";
                let is_pro = tier == "pro";
                let is_enterprise = tier == "enterprise";

                view! {
                    <div>
                        // Current Plan + Usage row
                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin-bottom: 24px;">
                            <Card title="Current Plan">
                                <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 16px;">
                                    <span style=format!(
                                        "display: inline-block; padding: 4px 16px; border-radius: 20px; font-weight: 700; font-size: 18px; color: white; background: {tier_color};"
                                    )>
                                        {tier_display}
                                    </span>
                                    {if unlimited {
                                        view! { <span class="text-muted">"(Unlimited — Admin)"</span> }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }}
                                </div>
                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                                    <div class="metric-card">
                                        <span class="metric-label">"Max Datasets"</span>
                                        <span class="metric-value">{format_limit(max_datasets())}</span>
                                    </div>
                                    <div class="metric-card">
                                        <span class="metric-label">"Max Models"</span>
                                        <span class="metric-value">{format_limit(max_models())}</span>
                                    </div>
                                    <div class="metric-card">
                                        <span class="metric-label">"Concurrent Training"</span>
                                        <span class="metric-value">{format_limit(max_trainings())}</span>
                                    </div>
                                    <div class="metric-card">
                                        <span class="metric-label">"Max Deployments"</span>
                                        <span class="metric-value">{format_limit(max_deployments())}</span>
                                    </div>
                                </div>
                                <div style="margin-top: 16px;">
                                    <button
                                        class="btn btn-ghost"
                                        on:click=open_portal
                                        disabled=move || portal_loading.get()
                                    >
                                        {move || if portal_loading.get() { "Opening..." } else { "Manage Subscription" }}
                                    </button>
                                </div>
                            </Card>

                            <Card title="Token Usage">
                                {if unlimited {
                                    view! {
                                        <div style="text-align: center; padding: 20px;">
                                            <span style="font-size: 32px; font-weight: 700; color: #14b8a6;">"∞"</span>
                                            <p class="text-muted">"Unlimited tokens (Admin)"</p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                                                <span style="font-size: 14px; font-weight: 600;">{format!("{} / {} tokens", format_number(used), format_number(limit))}</span>
                                                <span style=format!("font-size: 14px; color: {bar_color}; font-weight: 600;")>
                                                    {format!("{:.1}%", pct)}
                                                </span>
                                            </div>
                                            <div style="height: 12px; background: #f3f4f6; border-radius: 6px; overflow: hidden;">
                                                <div style=format!(
                                                    "height: 100%; width: {bar_width}%; background: {bar_color}; border-radius: 6px; transition: width 0.5s ease;"
                                                )></div>
                                            </div>
                                            {if { pct > 100.0 } {
                                                view! {
                                                    <p style="margin-top: 8px; font-size: 12px; color: #dc2626;">
                                                        "You have exceeded your token limit. Overage billed at $0.50 per 1,000 tokens."
                                                    </p>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                            <p class="text-muted" style="margin-top: 12px; font-size: 13px;">
                                                "Tokens reset each billing period. Paid plans allow overage billing."
                                            </p>
                                        </div>
                                    }.into_any()
                                }}
                            </Card>
                        </div>

                        // Plans heading
                        <h2 style="font-size: 1.25rem; font-weight: 700; color: #111827; margin-bottom: 16px;">"Plans"</h2>
                        <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; margin-bottom: 32px;">
                            // Free
                            <div style=move || if is_free {
                                "background: white; border-radius: 12px; padding: 24px; border: 2px solid #3b82f6; display: flex; flex-direction: column;"
                            } else {
                                "background: white; border-radius: 12px; padding: 24px; border: 1px solid #E8D4C4; display: flex; flex-direction: column;"
                            }>
                                <h3 style="font-size: 18px; font-weight: 700; color: #111827; margin-bottom: 4px;">"Free"</h3>
                                <div style="margin-bottom: 16px;">
                                    <span style="font-size: 32px; font-weight: 800; color: #111827;">"$0"</span>
                                    <span class="text-muted" style="font-size: 14px;">"/mo"</span>
                                </div>
                                <ul style="list-style: none; padding: 0; margin: 0 0 20px 0; flex: 1;">
                                    {feature_item("5,000 tokens/mo")}
                                    {feature_item("3 datasets (25 MB)")}
                                    {feature_item("2 models")}
                                    {feature_item("1 concurrent training")}
                                    {feature_item("1 deployment")}
                                </ul>
                                <button class="btn" disabled=true style="width: 100%; background: #f3f4f6; color: #6b7280; cursor: default;">
                                    {if is_free { "Current Plan" } else { "Downgrade via Portal" }}
                                </button>
                            </div>

                            // Basic
                            <div style=move || if is_basic {
                                "background: white; border-radius: 12px; padding: 24px; border: 2px solid #3b82f6; display: flex; flex-direction: column;"
                            } else {
                                "background: white; border-radius: 12px; padding: 24px; border: 1px solid #E8D4C4; display: flex; flex-direction: column;"
                            }>
                                <h3 style="font-size: 18px; font-weight: 700; color: #111827; margin-bottom: 4px;">"Basic"</h3>
                                <div style="margin-bottom: 16px;">
                                    <span style="font-size: 32px; font-weight: 800; color: #111827;">"$12"</span>
                                    <span class="text-muted" style="font-size: 14px;">"/mo"</span>
                                </div>
                                <ul style="list-style: none; padding: 0; margin: 0 0 20px 0; flex: 1;">
                                    {feature_item("10,000 tokens/mo")}
                                    {feature_item("10 datasets (50 MB)")}
                                    {feature_item("5 models")}
                                    {feature_item("2 concurrent trainings")}
                                    {feature_item("3 deployments")}
                                    {feature_item("Overage billing")}
                                </ul>
                                {if is_basic {
                                    view! { <button class="btn" disabled=true style="width: 100%; background: #f3f4f6; color: #6b7280; cursor: default;">"Current Plan"</button> }.into_any()
                                } else {
                                    let sc = start_checkout;
                                    view! { <button class="btn btn-primary" style="width: 100%;" on:click=move |_| sc("basic")>"Upgrade"</button> }.into_any()
                                }}
                            </div>

                            // Pro (popular)
                            <div style=move || if is_pro {
                                "background: white; border-radius: 12px; padding: 24px; border: 2px solid #14b8a6; display: flex; flex-direction: column; position: relative;"
                            } else {
                                "background: white; border-radius: 12px; padding: 24px; border: 2px solid #14b8a6; display: flex; flex-direction: column; position: relative;"
                            }>
                                <div style="position: absolute; top: -12px; left: 50%; transform: translateX(-50%); background: #14b8a6; color: white; padding: 2px 16px; border-radius: 12px; font-size: 12px; font-weight: 600;">
                                    "Most Popular"
                                </div>
                                <h3 style="font-size: 18px; font-weight: 700; color: #111827; margin-bottom: 4px;">"Pro"</h3>
                                <div style="margin-bottom: 16px;">
                                    <span style="font-size: 32px; font-weight: 800; color: #111827;">"$49"</span>
                                    <span class="text-muted" style="font-size: 14px;">"/mo"</span>
                                </div>
                                <ul style="list-style: none; padding: 0; margin: 0 0 20px 0; flex: 1;">
                                    {feature_item("50,000 tokens/mo")}
                                    {feature_item("50 datasets (250 MB)")}
                                    {feature_item("25 models")}
                                    {feature_item("5 concurrent trainings")}
                                    {feature_item("10 deployments")}
                                    {feature_item("Overage billing")}
                                    {feature_item("Priority support")}
                                </ul>
                                {if is_pro {
                                    view! { <button class="btn" disabled=true style="width: 100%; background: #f3f4f6; color: #6b7280; cursor: default;">"Current Plan"</button> }.into_any()
                                } else {
                                    let sc = start_checkout;
                                    view! { <button class="btn btn-primary" style="width: 100%;" on:click=move |_| sc("pro")>"Upgrade"</button> }.into_any()
                                }}
                            </div>

                            // Enterprise
                            <div style=move || if is_enterprise {
                                "background: white; border-radius: 12px; padding: 24px; border: 2px solid #C2714F; display: flex; flex-direction: column;"
                            } else {
                                "background: white; border-radius: 12px; padding: 24px; border: 1px solid #E8D4C4; display: flex; flex-direction: column;"
                            }>
                                <h3 style="font-size: 18px; font-weight: 700; color: #111827; margin-bottom: 4px;">"Enterprise"</h3>
                                <div style="margin-bottom: 16px;">
                                    <span style="font-size: 32px; font-weight: 800; color: #111827;">"$249"</span>
                                    <span class="text-muted" style="font-size: 14px;">"/mo"</span>
                                </div>
                                <ul style="list-style: none; padding: 0; margin: 0 0 20px 0; flex: 1;">
                                    {feature_item("1,000,000 tokens/mo")}
                                    {feature_item("500 datasets (50 GB)")}
                                    {feature_item("200 models")}
                                    {feature_item("20 concurrent trainings")}
                                    {feature_item("100 deployments")}
                                    {feature_item("Overage billing")}
                                    {feature_item("Dedicated support")}
                                </ul>
                                {if is_enterprise {
                                    view! { <button class="btn" disabled=true style="width: 100%; background: #f3f4f6; color: #6b7280; cursor: default;">"Current Plan"</button> }.into_any()
                                } else {
                                    let sc = start_checkout;
                                    view! { <button class="btn btn-primary" style="width: 100%;" on:click=move |_| sc("enterprise")>"Upgrade"</button> }.into_any()
                                }}
                            </div>
                        </div>

                        // Sponsorships
                        <h2 style="font-size: 1.25rem; font-weight: 700; color: #111827; margin-bottom: 16px;">"Support Open Source"</h2>
                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin-bottom: 24px;">
                            <Card title="AxonML Framework">
                                <p class="text-muted" style="font-size: 13px; margin-bottom: 16px;">
                                    "Support the development of AxonML — a pure-Rust ML training framework powering Prometheus."
                                </p>
                                <div style="display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px;">
                                    {sponsorship_tier("Supporter", "$5/mo")}
                                    {sponsorship_tier("Backer", "$15/mo")}
                                    {sponsorship_tier("Sponsor", "$50/mo")}
                                    {sponsorship_tier("Champion", "$100/mo")}
                                </div>
                                <a
                                    href="https://billing.stripe.com/p/login/6oUfZg8qR1Rh8WggYp3cc00"
                                    target="_blank"
                                    class="btn btn-ghost"
                                    style="width: 100%; text-align: center; display: block; text-decoration: none;"
                                >
                                    "Subscribe via Stripe Portal"
                                </a>
                            </Card>
                            <Card title="Prometheus Development">
                                <p class="text-muted" style="font-size: 13px; margin-bottom: 16px;">
                                    "Help fund Prometheus development — AI-forged edge intelligence for everyone."
                                </p>
                                <div style="display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px;">
                                    {sponsorship_tier("Fan", "$5/mo")}
                                    {sponsorship_tier("Advocate", "$15/mo")}
                                    {sponsorship_tier("Patron", "$50/mo")}
                                    {sponsorship_tier("Benefactor", "$100/mo")}
                                </div>
                                <a
                                    href="https://billing.stripe.com/p/login/6oUfZg8qR1Rh8WggYp3cc00"
                                    target="_blank"
                                    class="btn btn-ghost"
                                    style="width: 100%; text-align: center; display: block; text-decoration: none;"
                                >
                                    "Subscribe via Stripe Portal"
                                </a>
                            </Card>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn feature_item(text: &str) -> impl IntoView {
    let text = text.to_string();
    view! {
        <li style="padding: 4px 0; font-size: 13px; color: #374151; display: flex; align-items: center; gap: 6px;">
            <span style="color: #14b8a6; font-weight: 700;">"✓"</span>
            {text}
        </li>
    }
}

fn sponsorship_tier(name: &str, price: &str) -> impl IntoView {
    let name = name.to_string();
    let price = price.to_string();
    view! {
        <div style="display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: #FFFDF7; border-radius: 8px; border: 1px solid #E8D4C4;">
            <span style="font-weight: 600; font-size: 13px; color: #111827;">{name}</span>
            <span style="font-size: 13px; color: #6b7280;">{price}</span>
        </div>
    }
}

fn format_limit(val: u64) -> String {
    if val >= 1_000_000_000 {
        "Unlimited".to_string()
    } else {
        val.to_string()
    }
}
