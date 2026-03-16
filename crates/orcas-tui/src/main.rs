use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use orcas_core::AppPaths;

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let paths = AppPaths::discover()?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &paths);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, paths: &AppPaths) -> Result<()> {
    loop {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5),
                    Constraint::Length(7),
                    Constraint::Min(8),
                ])
                .split(frame.area());

            let header = Paragraph::new(Text::from(vec![
                Line::styled("Orcas TUI Placeholder", Style::default().add_modifier(Modifier::BOLD)),
                Line::from("Boundary only for now: connection shell, thread list placeholder, event log placeholder."),
                Line::from("Keys: q quit, ? help."),
            ]))
            .block(Block::default().title("Status").borders(Borders::ALL));

            let threads = Paragraph::new(Text::from(vec![
                Line::from("No live client wired yet."),
                Line::from(format!("Config: {}", paths.config_file.display())),
                Line::from(format!("State: {}", paths.state_file.display())),
                Line::from("Future: live thread registry, selection, approvals, streaming transcript."),
            ]))
            .block(Block::default().title("Threads").borders(Borders::ALL));

            let log = Paragraph::new(Text::from(vec![
                Line::from("event> placeholder frontend boundary established"),
                Line::from("event> supervisor and browser bridge can reuse Orcas core/codex crates"),
                Line::from("event> current pass intentionally avoids full interaction model"),
            ]))
            .block(Block::default().title("Event Log").borders(Borders::ALL));

            frame.render_widget(header, layout[0]);
            frame.render_widget(threads, layout[1]);
            frame.render_widget(log, layout[2]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('?') => {}
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
