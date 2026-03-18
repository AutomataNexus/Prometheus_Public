// ============================================================================
// File: convert_view.rs
// Description: TUI convert tab — ONNX/HEF conversion status for models
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
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(10),
        ])
        .split(area);

    frame.render_widget(
        status_bar("Convert", &app.last_refresh, app.last_error.as_deref()),
        chunks[0],
    );

    let header = Paragraph::new(Line::from(vec![
        Span::styled("  Convert ", Style::default().fg(TEAL).bold()),
        Span::styled("ONNX / HEF Export", Style::default().fg(TEXT_MUTED)),
    ]))
    .style(Style::default().bg(PANEL_BG));
    frame.render_widget(header, chunks[1]);

    let header_row = Row::new(vec!["Model", "Architecture", "Size", "ONNX", "HEF", "Status"])
        .style(Style::default().fg(TEAL).bold())
        .bottom_margin(1);

    let rows: Vec<Row> = app.models.iter().filter(|m| {
        m.get("status").and_then(|v| v.as_str()) == Some("ready")
    }).map(|m| {
        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("-");
        let arch = m.get("architecture").and_then(|v| v.as_str()).unwrap_or("-");
        let size = m.get("file_size_bytes").and_then(|v| v.as_u64())
            .map(|b| if b > 1_048_576 { format!("{:.1} MB", b as f64 / 1_048_576.0) }
                 else { format!("{:.0} KB", b as f64 / 1024.0) })
            .unwrap_or_else(|| "-".into());
        let quantized = m.get("quantized").and_then(|v| v.as_bool()).unwrap_or(false);
        let onnx = if quantized { "N/A" } else { "Ready" };
        let hef = if quantized { "N/A" } else { "Ready" };

        Row::new(vec![
            Cell::from(name.to_string()).style(Style::default().fg(TEXT_PRIMARY)),
            Cell::from(arch.to_string()).style(Style::default().fg(RUSSET)),
            Cell::from(size),
            Cell::from(onnx).style(Style::default().fg(SUCCESS)),
            Cell::from(hef).style(Style::default().fg(TEAL)),
            Cell::from("ready").style(Style::default().fg(SUCCESS)),
        ])
    }).collect();

    let widths = [
        Constraint::Percentage(25), Constraint::Percentage(15), Constraint::Percentage(12),
        Constraint::Percentage(12), Constraint::Percentage(12), Constraint::Percentage(12),
    ];

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(nexus_block(" ONNX / HEF Conversion "))
        .style(Style::default().bg(DARK_BG));

    frame.render_widget(table, chunks[2]);
}
