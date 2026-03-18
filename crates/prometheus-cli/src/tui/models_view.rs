// ============================================================================
// File: models_view.rs
// Description: TUI models tab showing trained models with metrics
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 17, 2026
// ============================================================================

use ratatui::prelude::*;
use ratatui::widgets::{Cell, Row, Table, Paragraph};
use super::app::App;
use super::widgets::*;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Status bar
            Constraint::Length(3),  // Header
            Constraint::Min(10),   // Table
        ])
        .split(area);

    // Status bar
    frame.render_widget(
        status_bar("Models", &app.last_refresh, app.last_error.as_deref()),
        chunks[0],
    );

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled("  Models ", Style::default().fg(TEAL).bold()),
        Span::styled(
            format!("({} total)", app.models.len()),
            Style::default().fg(TEXT_MUTED),
        ),
    ]))
    .style(Style::default().bg(PANEL_BG));
    frame.render_widget(header, chunks[1]);

    // Table
    let header_row = Row::new(vec!["Name", "Architecture", "F1", "Precision", "Recall", "Val Loss", "Status"])
        .style(Style::default().fg(TEAL).bold())
        .bottom_margin(1);

    let rows: Vec<Row> = app.models.iter().map(|m| {
        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("-");
        let arch = m.get("architecture").and_then(|v| v.as_str()).unwrap_or("-");
        let metrics = m.get("metrics");
        let f1 = metrics.and_then(|m| m.get("f1")).and_then(|v| v.as_f64())
            .map(|v| format!("{v:.3}")).unwrap_or_else(|| "-".into());
        let precision = metrics.and_then(|m| m.get("precision")).and_then(|v| v.as_f64())
            .map(|v| format!("{v:.3}")).unwrap_or_else(|| "-".into());
        let recall = metrics.and_then(|m| m.get("recall")).and_then(|v| v.as_f64())
            .map(|v| format!("{v:.3}")).unwrap_or_else(|| "-".into());
        let val_loss = metrics.and_then(|m| m.get("val_loss")).and_then(|v| v.as_f64())
            .map(|v| format!("{v:.4}")).unwrap_or_else(|| "-".into());
        let status = m.get("status").and_then(|v| v.as_str()).unwrap_or("pending");

        let status_style = match status {
            "ready" => Style::default().fg(SUCCESS),
            "failed" => Style::default().fg(ERROR),
            _ => Style::default().fg(TEXT_MUTED),
        };

        Row::new(vec![
            Cell::from(name.to_string()).style(Style::default().fg(TEXT_PRIMARY)),
            Cell::from(arch.to_string()).style(Style::default().fg(RUSSET)),
            Cell::from(f1).style(Style::default().fg(TEAL)),
            Cell::from(precision),
            Cell::from(recall),
            Cell::from(val_loss).style(Style::default().fg(RUSSET)),
            Cell::from(status.to_string()).style(status_style),
        ])
    }).collect();

    let widths = [
        Constraint::Percentage(22),
        Constraint::Percentage(15),
        Constraint::Percentage(10),
        Constraint::Percentage(10),
        Constraint::Percentage(10),
        Constraint::Percentage(12),
        Constraint::Percentage(10),
    ];

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(nexus_block(" Models "))
        .style(Style::default().bg(DARK_BG));

    frame.render_widget(table, chunks[2]);
}
