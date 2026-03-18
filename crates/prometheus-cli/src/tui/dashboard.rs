// ============================================================================
// File: dashboard.rs
// Description: TUI dashboard view rendering training runs and model summaries
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Dashboard view — overview of training runs and models.

use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, Paragraph, Row, Table};
use super::app::App;
use super::widgets::*;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // status bar
            Constraint::Length(3),  // summary
            Constraint::Min(8),    // training runs
            Constraint::Length(8), // models
        ])
        .split(frame.area());

    // Status bar
    frame.render_widget(
        status_bar("Dashboard", &app.last_refresh, app.last_error.as_deref()),
        chunks[0],
    );

    // Summary stats
    let running = app.training_runs.iter()
        .filter(|r| r.get("status").and_then(|v| v.as_str()) == Some("running"))
        .count();
    let completed = app.training_runs.iter()
        .filter(|r| r.get("status").and_then(|v| v.as_str()) == Some("completed"))
        .count();

    let summary = Paragraph::new(Line::from(vec![
        Span::styled("  Training: ", Style::default().fg(TERRACOTTA)),
        Span::styled(format!("{running} running"), Style::default().fg(if running > 0 { TEAL } else { CREAM })),
        Span::styled(format!("  {completed} completed"), Style::default().fg(SUCCESS)),
        Span::styled(format!("  {} total", app.training_runs.len()), Style::default().fg(TEXT_PRIMARY)),
        Span::styled("    Models: ", Style::default().fg(TERRACOTTA)),
        Span::styled(format!("{}", app.models.len()), Style::default().fg(TEXT_PRIMARY)),
    ]))
    .block(nexus_block("Overview"))
    .style(Style::default().bg(DARK_BG));

    frame.render_widget(summary, chunks[1]);

    // Training runs table
    let header = Row::new(vec!["ID", "Architecture", "Epoch", "Val Loss", "Status"])
        .style(Style::default().fg(TEAL).bold())
        .bottom_margin(1);

    let rows: Vec<Row> = app.training_runs.iter().enumerate().map(|(i, run)| {
        let id = run.get("id").and_then(|v| v.as_str()).unwrap_or("--");
        let arch = run.get("architecture").and_then(|v| v.as_str()).unwrap_or("--");
        let epoch = run.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0);
        let total = run.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(0);
        let val_loss = run.get("best_val_loss").and_then(|v| v.as_f64())
            .map(|v| format!("{v:.6}")).unwrap_or_else(|| "--".into());
        let status = run.get("status").and_then(|v| v.as_str()).unwrap_or("--");

        let style = if i == app.selected_index {
            Style::default().bg(PANEL_BG).fg(TEXT_PRIMARY)
        } else {
            Style::default().fg(TEXT_PRIMARY)
        };

        Row::new(vec![
            id.to_string(),
            arch.to_string(),
            format!("{epoch}/{total}"),
            val_loss,
            status.to_string(),
        ]).style(style)
    }).collect();

    let widths = [
        Constraint::Length(14),
        Constraint::Length(18),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(nexus_block("Training Runs"))
        .highlight_style(Style::default().bg(PANEL_BG));

    frame.render_widget(table, chunks[2]);

    // Models list
    let model_items: Vec<ListItem> = app.models.iter().map(|m| {
        let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("--");
        let arch = m.get("architecture").and_then(|v| v.as_str()).unwrap_or("--");
        let f1 = m.get("metrics").and_then(|v| v.get("f1")).and_then(|v| v.as_f64())
            .map(|v| format!("{v:.3}")).unwrap_or_else(|| "--".into());
        let status = m.get("status").and_then(|v| v.as_str()).unwrap_or("--");

        ListItem::new(Line::from(vec![
            Span::styled(format!("{id:<14}"), Style::default().fg(TEAL)),
            Span::raw("  "),
            Span::styled(format!("{arch:<18}"), Style::default().fg(TEXT_PRIMARY)),
            Span::raw("  F1: "),
            Span::styled(f1, Style::default().fg(SUCCESS)),
            Span::raw("  "),
            Span::styled(status.to_string(), Style::default().fg(status_color(status))),
        ]))
    }).collect();

    let models_list = List::new(model_items)
        .block(nexus_block("Models"));

    frame.render_widget(models_list, chunks[3]);
}
