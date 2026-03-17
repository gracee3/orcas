use crate::app::{BannerLevel, DaemonLifecycleState, TopLevelView};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::view_model::PanelViewModel;

pub(super) fn render_panel(panel: PanelViewModel, trim: bool) -> Paragraph<'static> {
    Paragraph::new(Text::from(
        panel.lines.into_iter().map(Line::from).collect::<Vec<_>>(),
    ))
    .block(Block::default().title(panel.title).borders(Borders::ALL))
    .wrap(Wrap { trim })
}

pub(super) fn focus_title(base: &str, focused: bool) -> String {
    if focused {
        format!("{base} <focus>")
    } else {
        base.to_string()
    }
}

pub(super) fn focus_block_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    }
}

pub(super) fn row_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub(super) fn metadata_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub(super) fn emphasis_style() -> Style {
    Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn status_style(level: BannerLevel) -> Style {
    match level {
        BannerLevel::Info => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        BannerLevel::Warning => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        BannerLevel::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    }
}

pub(super) fn lifecycle_style(state: DaemonLifecycleState) -> Style {
    match state {
        DaemonLifecycleState::Running => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        DaemonLifecycleState::Starting | DaemonLifecycleState::Restarting => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        DaemonLifecycleState::Stopping => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        DaemonLifecycleState::Failed => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
        DaemonLifecycleState::Stopped | DaemonLifecycleState::Unknown => {
            Style::default().fg(Color::DarkGray)
        }
    }
}

pub(super) fn title_case_view_label(view: TopLevelView) -> &'static str {
    match view {
        TopLevelView::Overview => "Overview",
        TopLevelView::Threads => "Threads",
        TopLevelView::Collaboration => "Collaboration",
        TopLevelView::Supervisor => "Supervisor",
    }
}
