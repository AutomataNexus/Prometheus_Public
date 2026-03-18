// ============================================================================
// File: chart.rs
// Description: SVG-based line chart component for visualizing time-series data
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;

#[derive(Clone)]
pub struct DataPoint {
    pub x: f64,
    pub y: f64,
}

#[component]
pub fn LineChart(
    data: Signal<Vec<DataPoint>>,
    #[prop(optional)] width: Option<u32>,
    #[prop(optional)] height: Option<u32>,
    #[prop(optional)] color: Option<&'static str>,
    #[prop(optional)] x_label: Option<&'static str>,
    #[prop(optional)] y_label: Option<&'static str>,
    #[prop(optional)] show_grid: Option<bool>,
) -> impl IntoView {
    let w = width.unwrap_or(600) as f64;
    let h = height.unwrap_or(300) as f64;
    let stroke_color = color.unwrap_or("#14b8a6");
    let padding = 40.0;

    let path = move || {
        let points = data.get();
        if points.is_empty() {
            return String::new();
        }
        let x_min = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let x_max = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let y_min = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let y_max = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

        let x_range = if (x_max - x_min).abs() < 1e-10 { 1.0 } else { x_max - x_min };
        let y_range = if (y_max - y_min).abs() < 1e-10 { 1.0 } else { y_max - y_min };

        let chart_w = w - 2.0 * padding;
        let chart_h = h - 2.0 * padding;

        points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let px = padding + (p.x - x_min) / x_range * chart_w;
                let py = padding + chart_h - (p.y - y_min) / y_range * chart_h;
                if i == 0 {
                    format!("M{px:.1},{py:.1}")
                } else {
                    format!("L{px:.1},{py:.1}")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    let grid_lines = move || {
        if !show_grid.unwrap_or(true) {
            return vec![];
        }
        let chart_h = h - 2.0 * padding;
        (0..=4)
            .map(|i| {
                let y = padding + (i as f64 / 4.0) * chart_h;
                (padding, w - padding, y)
            })
            .collect::<Vec<_>>()
    };

    let viewbox = format!("0 0 {w} {h}");

    view! {
        <div class="chart-container">
            <svg viewBox=viewbox xmlns="http://www.w3.org/2000/svg">
                // Grid
                {move || grid_lines().into_iter().map(|(x1, x2, y)| {
                    view! {
                        <line x1=x1.to_string() y1=y.to_string() x2=x2.to_string() y2=y.to_string()
                            stroke="#E8D4C4" stroke-width="0.5" stroke-dasharray="4,4" />
                    }
                }).collect_view()}

                // Axes
                <line x1=padding.to_string() y1=padding.to_string()
                      x2=padding.to_string() y2=(h - padding).to_string()
                      stroke="#E8D4C4" stroke-width="1" />
                <line x1=padding.to_string() y1=(h - padding).to_string()
                      x2=(w - padding).to_string() y2=(h - padding).to_string()
                      stroke="#E8D4C4" stroke-width="1" />

                // Data line
                <path d=path fill="none" stroke=stroke_color stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />

                // Labels
                {x_label.map(|label| view! {
                    <text x=(w / 2.0).to_string() y=(h - 5.0).to_string()
                          text-anchor="middle" fill="#6b7280" font-size="12" font-family="Inter, sans-serif">
                        {label}
                    </text>
                })}
                {y_label.map(|label| view! {
                    <text x="12" y=(h / 2.0).to_string()
                          text-anchor="middle" fill="#6b7280" font-size="12" font-family="Inter, sans-serif"
                          transform=format!("rotate(-90, 12, {})", h / 2.0)>
                        {label}
                    </text>
                })}
            </svg>
        </div>
    }
}

#[component]
pub fn BarChart(
    labels: Vec<String>,
    values: Signal<Vec<f64>>,
    #[prop(optional)] color: Option<&'static str>,
    #[prop(optional)] height: Option<u32>,
) -> impl IntoView {
    let h = height.unwrap_or(200) as f64;
    let bar_color = color.unwrap_or("#14b8a6");

    view! {
        <div class="chart-container">
            <div style=format!("display: flex; align-items: flex-end; gap: 8px; height: {h}px; padding: 0 8px;")>
                {move || {
                    let vals = values.get();
                    let max_val = vals.iter().cloned().fold(0.0f64, f64::max);
                    let max_val = if max_val < 1e-10 { 1.0 } else { max_val };
                    labels.iter().zip(vals.iter()).map(|(label, val)| {
                        let pct = val / max_val * 100.0;
                        let label = label.clone();
                        view! {
                            <div style="flex: 1; display: flex; flex-direction: column; align-items: center; gap: 4px;">
                                <span class="text-xs text-muted">{format!("{val:.1}")}</span>
                                <div style=format!(
                                    "width: 100%; height: {pct}%; background: {bar_color}; border-radius: 4px 4px 0 0; min-height: 2px; transition: height 0.3s;"
                                )></div>
                                <span class="text-xs text-muted">{label}</span>
                            </div>
                        }
                    }).collect_view()
                }}
            </div>
        </div>
    }
}
