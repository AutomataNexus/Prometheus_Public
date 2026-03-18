// ============================================================================
// File: widgets.rs
// Description: NexusEdge branded ratatui widgets with themed colors and blocks
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! NexusEdge branded TUI widgets.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

// NexusEdge color palette — matches WASM UI light theme
pub const TEAL: Color = Color::Rgb(20, 184, 166);
pub const DARK_BG: Color = Color::Rgb(255, 253, 247);    // #FFFDF7 cream (was dark)
pub const PANEL_BG: Color = Color::Rgb(250, 248, 245);   // #FAF8F5 surface (was dark)
pub const CREAM: Color = Color::Rgb(255, 253, 247);
pub const BORDER_TAN: Color = Color::Rgb(232, 212, 196);
pub const TERRACOTTA: Color = Color::Rgb(196, 164, 132);
pub const RUSSET: Color = Color::Rgb(194, 113, 79);
pub const SUCCESS: Color = Color::Rgb(34, 197, 94);
pub const WARNING: Color = Color::Rgb(249, 115, 22);
pub const ERROR: Color = Color::Rgb(220, 38, 38);
pub const INFO: Color = Color::Rgb(59, 130, 246);
pub const TEXT_PRIMARY: Color = Color::Rgb(17, 24, 39);   // #111827
pub const TEXT_MUTED: Color = Color::Rgb(107, 114, 128);  // #6b7280

pub fn nexus_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_TAN))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(TEAL).bold(),
        ))
        .style(Style::default().bg(DARK_BG))
}

pub fn status_color(status: &str) -> Color {
    match status {
        "running" | "active" => TEAL,
        "completed" | "ready" | "deployed" => SUCCESS,
        "failed" | "error" => ERROR,
        "stopping" | "cancelled" | "pending" => WARNING,
        _ => TERRACOTTA,
    }
}

pub fn status_bar<'a>(tab: &str, last_refresh: &str, error: Option<&str>) -> Paragraph<'a> {
    let status = if let Some(err) = error {
        Span::styled(format!(" Error: {err} "), Style::default().fg(ERROR))
    } else {
        Span::styled(
            format!(" Last refresh: {last_refresh} "),
            Style::default().fg(TERRACOTTA),
        )
    };

    Paragraph::new(Line::from(vec![
        Span::styled(" PROMETHEUS ", Style::default().fg(DARK_BG).bg(TEAL).bold()),
        Span::raw(" "),
        Span::styled(format!(" {tab} "), Style::default().fg(TEXT_PRIMARY).bg(RUSSET)),
        Span::raw(" "),
        status,
        Span::styled(
            " q:Quit  Tab:Switch  r:Refresh  ↑↓:Navigate  Enter:Select ",
            Style::default().fg(BORDER_TAN),
        ),
    ]))
    .style(Style::default().bg(Color::Rgb(250, 248, 245)))
}

pub fn metric_span(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {label:<16}"),
            Style::default().fg(TERRACOTTA),
        ),
        Span::styled(
            value.to_string(),
            Style::default().fg(TEXT_PRIMARY),
        ),
    ])
}
