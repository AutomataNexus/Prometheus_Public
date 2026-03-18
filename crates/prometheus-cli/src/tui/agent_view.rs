// ============================================================================
// File: agent_view.rs
// Description: TUI PrometheusForge agent tab — chat history and info
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 17, 2026
// ============================================================================

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use super::app::App;
use super::widgets::*;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Min(8),
        ])
        .split(area);

    frame.render_widget(
        status_bar("Agent", &app.last_refresh, app.last_error.as_deref()),
        chunks[0],
    );

    let header = Paragraph::new(Line::from(vec![
        Span::styled("  PrometheusForge ", Style::default().fg(TEAL).bold()),
        Span::styled("Gradient AI Agent", Style::default().fg(TEXT_MUTED)),
    ]))
    .style(Style::default().bg(PANEL_BG));
    frame.render_widget(header, chunks[1]);

    // Agent info
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" Model     ", Style::default().fg(TEXT_MUTED)),
            Span::styled("Anthropic Claude Sonnet 4.6 via DigitalOcean Gradient AI", Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled(" KB        ", Style::default().fg(TEXT_MUTED)),
            Span::styled("24 documents indexed (RAG retrieval)", Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled(" Supports  ", Style::default().fg(TEXT_MUTED)),
            Span::styled("Architecture recommendations, data analysis, training config", Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled(" CLI       ", Style::default().fg(TEXT_MUTED)),
            Span::styled("prometheus agent 'your question here'", Style::default().fg(TEAL)),
        ]),
        Line::from(vec![
            Span::styled(" REPL      ", Style::default().fg(TEXT_MUTED)),
            Span::styled("/agent <message>  or  prometheus agent (interactive)", Style::default().fg(TEAL)),
        ]),
    ])
    .block(nexus_block(" PrometheusForge "))
    .style(Style::default().bg(DARK_BG));
    frame.render_widget(info, chunks[2]);

    // Chat history
    let mut lines: Vec<Line> = Vec::new();
    for (role, content) in &app.agent_messages {
        let prefix_style = if role == "user" {
            Style::default().fg(RUSSET).bold()
        } else {
            Style::default().fg(TEAL).bold()
        };
        let prefix = if role == "user" { " you> " } else { " forge> " };

        // Wrap long messages
        let max_width = area.width.saturating_sub(10) as usize;
        for (i, line) in content.lines().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![
                    Span::styled(prefix, prefix_style),
                    Span::styled(
                        if line.len() > max_width { format!("{}...", &line[..max_width]) } else { line.to_string() },
                        Style::default().fg(TEXT_PRIMARY),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("         "),
                    Span::styled(
                        if line.len() > max_width { format!("{}...", &line[..max_width]) } else { line.to_string() },
                        Style::default().fg(TEXT_PRIMARY),
                    ),
                ]));
            }
        }
        lines.push(Line::from(""));
    }

    let chat = Paragraph::new(lines)
        .block(nexus_block(" Chat History "))
        .style(Style::default().bg(DARK_BG));
    frame.render_widget(chat, chunks[3]);
}
