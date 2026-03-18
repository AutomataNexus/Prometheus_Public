// ============================================================================
// File: datasets_view.rs
// Description: TUI datasets tab showing uploaded datasets with stats
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 17, 2026
// ============================================================================

use ratatui::prelude::*;
use ratatui::widgets::{Row, Table, Paragraph};
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
        status_bar("Datasets", &app.last_refresh, app.last_error.as_deref()),
        chunks[0],
    );

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled("  Datasets ", Style::default().fg(TEAL).bold()),
        Span::styled(
            format!("({} total)", app.datasets.len()),
            Style::default().fg(TEXT_MUTED),
        ),
    ]))
    .style(Style::default().bg(PANEL_BG));
    frame.render_widget(header, chunks[1]);

    // Table
    let header_row = Row::new(vec!["Name", "Domain", "Rows", "Columns", "Size", "Status"])
        .style(Style::default().fg(TEAL).bold())
        .bottom_margin(1);

    let rows: Vec<Row> = app.datasets.iter().map(|ds| {
        let name = ds.get("name").and_then(|v| v.as_str()).unwrap_or("-");
        let domain = ds.get("domain").and_then(|v| v.as_str()).unwrap_or("-");
        let rows_count = ds.get("row_count").and_then(|v| v.as_u64())
            .map(|v| format!("{v}")).unwrap_or_else(|| "-".into());
        let cols = ds.get("columns").and_then(|v| v.as_array())
            .map(|a| format!("{}", a.len())).unwrap_or_else(|| "-".into());
        let size = ds.get("file_size_bytes").and_then(|v| v.as_u64())
            .map(|b| if b > 1_048_576 { format!("{:.1} MB", b as f64 / 1_048_576.0) }
                 else { format!("{:.0} KB", b as f64 / 1024.0) })
            .unwrap_or_else(|| "-".into());
        let status = ds.get("is_validated").and_then(|v| v.as_bool())
            .map(|v| if v { "Validated" } else { "Pending" }).unwrap_or("Pending");

        Row::new(vec![
            name.to_string(), domain.to_string(), rows_count, cols, size, status.to_string(),
        ]).style(Style::default().fg(TEXT_PRIMARY))
    }).collect();

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(12),
        Constraint::Percentage(12),
        Constraint::Percentage(12),
        Constraint::Percentage(12),
    ];

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(nexus_block(" Datasets "))
        .style(Style::default().bg(DARK_BG));

    frame.render_widget(table, chunks[2]);
}
