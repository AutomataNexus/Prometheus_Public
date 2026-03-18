// ============================================================================
// File: mod.rs
// Description: TUI module entry point with terminal setup and main event loop
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Prometheus TUI — real-time training monitor with NexusEdge theme.

mod agent_view;
mod app;
mod convert_view;
mod dashboard;
mod datasets_view;
mod deploy_view;
mod models_view;
mod monitor;
mod quantize_view;
mod widgets;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

use crate::config::Config;

pub async fn run_tui(cfg: &Config, focus_run_id: Option<String>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = app::App::new(cfg.server_url.clone(), cfg.load_token(), focus_run_id);

    // Initial data fetch
    app.refresh().await;

    // Main loop
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    let tick_rate = std::time::Duration::from_millis(250);
    let refresh_interval = std::time::Duration::from_secs(3);
    let mut last_refresh = std::time::Instant::now();

    loop {
        terminal.draw(|frame| {
            match app.current_tab {
                app::Tab::Dashboard => dashboard::render(frame, app),
                app::Tab::Datasets => datasets_view::render(frame, app),
                app::Tab::Models => models_view::render(frame, app),
                app::Tab::Monitor => monitor::render(frame, app),
                app::Tab::Agent => agent_view::render(frame, app),
                app::Tab::Convert => convert_view::render(frame, app),
                app::Tab::Quantize => quantize_view::render(frame, app),
                app::Tab::Deploy => deploy_view::render(frame, app),
            }
        })?;

        // Handle input with timeout
        if crossterm::event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::Char('r') => app.refresh().await,
                    KeyCode::Up | KeyCode::Char('k') => app.previous_item(),
                    KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                    KeyCode::Enter => app.select_item(),
                    KeyCode::Char('1') => app.current_tab = app::Tab::Dashboard,
                    KeyCode::Char('2') => app.current_tab = app::Tab::Datasets,
                    KeyCode::Char('3') => app.current_tab = app::Tab::Models,
                    KeyCode::Char('4') => app.current_tab = app::Tab::Monitor,
                    KeyCode::Char('5') => app.current_tab = app::Tab::Agent,
                    KeyCode::Char('6') => app.current_tab = app::Tab::Convert,
                    KeyCode::Char('7') => app.current_tab = app::Tab::Quantize,
                    KeyCode::Char('8') => app.current_tab = app::Tab::Deploy,
                    _ => {}
                }
            }
        }

        // Auto-refresh
        if last_refresh.elapsed() >= refresh_interval {
            app.refresh().await;
            last_refresh = std::time::Instant::now();
        }
    }
}
