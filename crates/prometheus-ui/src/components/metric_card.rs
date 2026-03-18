// ============================================================================
// File: metric_card.rs
// Description: Dashboard metric card component with trend indicators and tooltips
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 16, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Trend {
    Up,
    Down,
    Neutral,
}

#[component]
pub fn MetricCard(
    label: &'static str,
    value: Signal<String>,
    #[prop(optional)] trend: Option<Signal<Trend>>,
    #[prop(optional)] trend_text: Option<Signal<String>>,
    #[prop(optional)] icon_name: Option<&'static str>,
    #[prop(optional)] tooltip: Option<&'static str>,
) -> impl IntoView {
    view! {
        <div class="metric-card">
            <div style="display: flex; align-items: center; justify-content: space-between;">
                <div style="display: flex; align-items: center; gap: 4px;">
                    <span class="label">{label}</span>
                    {tooltip.map(|tip| view! {
                        <InfoTip text=tip />
                    })}
                </div>
                {icon_name.map(|name| view! {
                    <div style="width: 40px; height: 40px; border-radius: 8px; background: #F5EDE8; display: flex; align-items: center; justify-content: center; color: #C4A484;">
                        <crate::components::icon::Icon name=name.to_string() size=20 />
                    </div>
                })}
            </div>
            <div class="value">{value}</div>
            {move || trend.map(|t| {
                let trend_val = t.get();
                let (arrow, class) = match trend_val {
                    Trend::Up => ("\u{2191}", "trend trend-up"),
                    Trend::Down => ("\u{2193}", "trend trend-down"),
                    Trend::Neutral => ("\u{2192}", "trend text-muted"),
                };
                view! {
                    <div class=class>
                        <span>{arrow}</span>
                        {trend_text.map(|tt| tt.get())}
                    </div>
                }
            })}
        </div>
    }
}

/// Small "?" icon with hover tooltip for explaining metrics to users.
#[component]
pub fn InfoTip(
    text: &'static str,
) -> impl IntoView {
    view! {
        <span
            class="info-tip"
            title=text
            style="display: inline-flex; align-items: center; justify-content: center; width: 14px; height: 14px; border-radius: 50%; background: #e5e7eb; color: #6b7280; font-size: 9px; font-weight: 700; cursor: help; flex-shrink: 0; user-select: none;"
        >
            "?"
        </span>
    }
}
