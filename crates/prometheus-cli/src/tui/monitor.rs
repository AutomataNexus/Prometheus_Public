// ============================================================================
// File: monitor.rs
// Description: TUI training monitor view with live loss chart and epoch metrics
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Training monitor view — live loss visualization and metrics.

use ratatui::prelude::*;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph};
use super::app::App;
use super::widgets::*;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // status bar
            Constraint::Length(7),  // run info
            Constraint::Min(12),   // loss chart
            Constraint::Length(6), // epoch metrics
        ])
        .split(frame.area());

    // Status bar
    let tab_name = match &app.focus_run_id {
        Some(id) => format!("Monitor -- {id}"),
        None => "Monitor".to_string(),
    };
    frame.render_widget(
        status_bar(&tab_name, &app.last_refresh, app.last_error.as_deref()),
        chunks[0],
    );

    let run = match &app.selected_run {
        Some(r) => r,
        None => {
            let msg = Paragraph::new("No training run selected. Press Tab to go to Dashboard and select one.")
                .style(Style::default().fg(TERRACOTTA))
                .block(nexus_block("Training Monitor"));
            frame.render_widget(msg, chunks[1]);
            return;
        }
    };

    // Run info panel
    let id = run.get("id").and_then(|v| v.as_str()).unwrap_or("--");
    let arch = run.get("architecture").and_then(|v| v.as_str()).unwrap_or("--");
    let status = run.get("status").and_then(|v| v.as_str()).unwrap_or("--");
    let epoch = run.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0);
    let total = run.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(0);
    let best_loss = run.get("best_val_loss").and_then(|v| v.as_f64())
        .map(|v| format!("{v:.6}")).unwrap_or_else(|| "--".into());
    let dataset = run.get("dataset_id").and_then(|v| v.as_str()).unwrap_or("--");

    let info_text = vec![
        metric_span("Run ID", id),
        metric_span("Architecture", arch),
        metric_span("Dataset", dataset),
        metric_span("Status", status),
        metric_span("Progress", &format!("{epoch}/{total} epochs")),
        metric_span("Best Val Loss", &best_loss),
    ];

    let info = Paragraph::new(info_text)
        .block(nexus_block("Training Run"));
    frame.render_widget(info, chunks[1]);

    // Loss chart
    let epoch_metrics = run.get("epoch_metrics")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let train_data: Vec<(f64, f64)> = epoch_metrics.iter()
        .filter_map(|m| {
            let e = m.get("epoch").and_then(|v| v.as_u64())? as f64;
            let l = m.get("train_loss").and_then(|v| v.as_f64())?;
            Some((e, l))
        })
        .collect();

    let val_data: Vec<(f64, f64)> = epoch_metrics.iter()
        .filter_map(|m| {
            let e = m.get("epoch").and_then(|v| v.as_u64())? as f64;
            let l = m.get("val_loss").and_then(|v| v.as_f64())?;
            Some((e, l))
        })
        .collect();

    if train_data.is_empty() && val_data.is_empty() {
        let msg = Paragraph::new("  Waiting for training data...")
            .style(Style::default().fg(INFO))
            .block(nexus_block("Loss Curve"));
        frame.render_widget(msg, chunks[2]);
    } else {
        // Compute bounds
        let max_epoch = total.max(epoch) as f64;
        let all_losses: Vec<f64> = train_data.iter().map(|d| d.1)
            .chain(val_data.iter().map(|d| d.1))
            .collect();
        let max_loss = all_losses.iter().cloned().fold(0.0f64, f64::max) * 1.1;
        let min_loss = all_losses.iter().cloned().fold(f64::MAX, f64::min) * 0.9;
        let max_loss = if max_loss <= min_loss { min_loss + 0.1 } else { max_loss };

        let mut datasets = Vec::new();

        if !train_data.is_empty() {
            datasets.push(
                Dataset::default()
                    .name("Train Loss")
                    .marker(ratatui::symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(TEAL))
                    .data(&train_data)
            );
        }

        if !val_data.is_empty() {
            datasets.push(
                Dataset::default()
                    .name("Val Loss")
                    .marker(ratatui::symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(RUSSET))
                    .data(&val_data)
            );
        }

        let x_labels = vec![
            Span::styled("0", Style::default().fg(TERRACOTTA)),
            Span::styled(format!("{}", max_epoch as u64 / 2), Style::default().fg(TERRACOTTA)),
            Span::styled(format!("{}", max_epoch as u64), Style::default().fg(TERRACOTTA)),
        ];
        let y_labels = vec![
            Span::styled(format!("{min_loss:.4}"), Style::default().fg(TERRACOTTA)),
            Span::styled(format!("{:.4}", (min_loss + max_loss) / 2.0), Style::default().fg(TERRACOTTA)),
            Span::styled(format!("{max_loss:.4}"), Style::default().fg(TERRACOTTA)),
        ];

        let chart = Chart::new(datasets)
            .block(nexus_block("Loss Curve"))
            .x_axis(
                Axis::default()
                    .title(Span::styled("Epoch", Style::default().fg(TEXT_PRIMARY)))
                    .bounds([0.0, max_epoch])
                    .labels(x_labels),
            )
            .y_axis(
                Axis::default()
                    .title(Span::styled("Loss", Style::default().fg(TEXT_PRIMARY)))
                    .bounds([min_loss, max_loss])
                    .labels(y_labels),
            );

        frame.render_widget(chart, chunks[2]);
    }

    // Recent epoch metrics
    let recent: Vec<Line> = epoch_metrics.iter()
        .rev()
        .take(4)
        .rev()
        .map(|m| {
            let e = m.get("epoch").and_then(|v| v.as_u64()).unwrap_or(0);
            let tl = m.get("train_loss").and_then(|v| v.as_f64())
                .map(|v| format!("{v:.6}")).unwrap_or_else(|| "--".into());
            let vl = m.get("val_loss").and_then(|v| v.as_f64())
                .map(|v| format!("{v:.6}")).unwrap_or_else(|| "--".into());

            Line::from(vec![
                Span::styled(format!("  Epoch {e:<5}"), Style::default().fg(TEAL)),
                Span::styled(format!("  train: {tl:<12}"), Style::default().fg(TEXT_PRIMARY)),
                Span::styled(format!("  val: {vl}"), Style::default().fg(RUSSET)),
            ])
        })
        .collect();

    let epoch_panel = Paragraph::new(recent)
        .block(nexus_block("Recent Epochs"));
    frame.render_widget(epoch_panel, chunks[3]);
}
