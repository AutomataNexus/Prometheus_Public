// ============================================================================
// File: quantize_view.rs
// Description: TUI quantize tab — Q8/Q4/F16 quantization status
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
            Constraint::Length(5),
            Constraint::Min(8),
        ])
        .split(area);

    frame.render_widget(
        status_bar("Quantize", &app.last_refresh, app.last_error.as_deref()),
        chunks[0],
    );

    let header = Paragraph::new(Line::from(vec![
        Span::styled("  Quantize ", Style::default().fg(TEAL).bold()),
        Span::styled("Q8_0 · Q4_0 · Q4_1 · F16", Style::default().fg(TEXT_MUTED)),
    ]))
    .style(Style::default().bg(PANEL_BG));
    frame.render_widget(header, chunks[1]);

    // Compression ratios info
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" Q8_0 ", Style::default().fg(TEAL).bold()),
            Span::styled("3.8x  ", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" Q4_0 ", Style::default().fg(WARNING).bold()),
            Span::styled("7.1x  ", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" Q4_1 ", Style::default().fg(INFO).bold()),
            Span::styled("6.5x  ", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" F16 ", Style::default().fg(RUSSET).bold()),
            Span::styled("2.0x", Style::default().fg(TEXT_PRIMARY)),
        ]),
    ])
    .block(nexus_block(" Compression Ratios "))
    .style(Style::default().bg(DARK_BG));
    frame.render_widget(info, chunks[2]);

    // Models table
    let header_row = Row::new(vec!["Model", "Architecture", "Original", "Quantized", "Type", "Ratio"])
        .style(Style::default().fg(TEAL).bold())
        .bottom_margin(1);

    let rows: Vec<Row> = app.models.iter().map(|m| {
        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("-");
        let arch = m.get("architecture").and_then(|v| v.as_str()).unwrap_or("-");
        let size = m.get("file_size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
        let size_str = if size > 1_048_576 { format!("{:.1} MB", size as f64 / 1_048_576.0) }
             else { format!("{:.0} KB", size as f64 / 1024.0) };
        let quantized = m.get("quantized").and_then(|v| v.as_bool()).unwrap_or(false);
        let qtype = m.get("quant_type").and_then(|v| v.as_str()).unwrap_or(if quantized { "Q8" } else { "-" });
        let ratio = m.get("compression_ratio").and_then(|v| v.as_f64())
            .map(|r| format!("{r:.1}x")).unwrap_or_else(|| "-".into());

        Row::new(vec![
            Cell::from(name.to_string()).style(Style::default().fg(TEXT_PRIMARY)),
            Cell::from(arch.to_string()).style(Style::default().fg(RUSSET)),
            Cell::from(size_str),
            Cell::from(if quantized { "Yes" } else { "No" }).style(
                if quantized { Style::default().fg(SUCCESS) } else { Style::default().fg(TEXT_MUTED) }
            ),
            Cell::from(qtype.to_string()).style(Style::default().fg(TEAL)),
            Cell::from(ratio),
        ])
    }).collect();

    let widths = [
        Constraint::Percentage(25), Constraint::Percentage(15), Constraint::Percentage(12),
        Constraint::Percentage(12), Constraint::Percentage(12), Constraint::Percentage(12),
    ];

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(nexus_block(" Models "))
        .style(Style::default().bg(DARK_BG));

    frame.render_widget(table, chunks[3]);
}
