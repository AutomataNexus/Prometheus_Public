// ============================================================================
// File: deploy_view.rs
// Description: TUI deploy tab — edge deployment targets and status
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
            Constraint::Length(7),
            Constraint::Min(8),
        ])
        .split(area);

    frame.render_widget(
        status_bar("Deploy", &app.last_refresh, app.last_error.as_deref()),
        chunks[0],
    );

    let header = Paragraph::new(Line::from(vec![
        Span::styled("  Deploy ", Style::default().fg(TEAL).bold()),
        Span::styled("Edge Controllers", Style::default().fg(TEXT_MUTED)),
    ]))
    .style(Style::default().bg(PANEL_BG));
    frame.render_widget(header, chunks[1]);

    // Deployment info
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" LSTM/GRU/RNN ", Style::default().fg(TEAL).bold()),
            Span::styled("→ Run natively on Raspberry Pi (~1.8 MB RSS)", Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled(" CNN/MLP      ", Style::default().fg(RUSSET).bold()),
            Span::styled("→ Accelerated via Hailo-8 NPU (HEF format)", Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled(" Credentials  ", Style::default().fg(WARNING).bold()),
            Span::styled("→ AES-256-GCM encrypted via Shield vault", Style::default().fg(TEXT_PRIMARY)),
        ]),
    ])
    .block(nexus_block(" Edge Deployment "))
    .style(Style::default().bg(DARK_BG));
    frame.render_widget(info, chunks[2]);

    // Models ready for deployment
    let header_row = Row::new(vec!["Model", "Architecture", "Size", "Quantized", "Format"])
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
        let format = if quantized { "AXQT" } else { "AxonML" };

        Row::new(vec![
            Cell::from(name.to_string()).style(Style::default().fg(TEXT_PRIMARY)),
            Cell::from(arch.to_string()).style(Style::default().fg(RUSSET)),
            Cell::from(size),
            Cell::from(if quantized { "Yes" } else { "No" }).style(
                if quantized { Style::default().fg(SUCCESS) } else { Style::default().fg(TEXT_MUTED) }
            ),
            Cell::from(format).style(Style::default().fg(TEAL)),
        ])
    }).collect();

    let widths = [
        Constraint::Percentage(25), Constraint::Percentage(20), Constraint::Percentage(15),
        Constraint::Percentage(15), Constraint::Percentage(15),
    ];

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(nexus_block(" Deployable Models "))
        .style(Style::default().bg(DARK_BG));

    frame.render_widget(table, chunks[3]);
}
