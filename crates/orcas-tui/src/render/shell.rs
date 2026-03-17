use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{AppState, TopLevelView};
use crate::view_model;
use crate::view_model::shared::{daemon_lifecycle_label, daemon_phase_label};

use super::shared::{status_style, title_case_view_label};

pub(super) fn render_shell_status(state: &AppState) -> Paragraph<'static> {
    let connection = view_model::connection_status(state);
    let mut lines = vec![
        Line::styled(
            format!(
                "Orcas Operator Console [{}]",
                title_case_view_label(state.current_view)
            ),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!(
                "daemon lifecycle: {}",
                daemon_lifecycle_label(state.daemon_lifecycle)
            ),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from(format!(
            "daemon: {}  upstream: {}  clients: {}  threads: {}  reconnect: {}",
            daemon_phase_label(connection.daemon_phase),
            connection.upstream_status,
            connection.client_count,
            connection.known_threads,
            connection.reconnect_attempt
        )),
        Line::styled(
            format!("socket: {}", connection.socket_path),
            Style::default().fg(Color::DarkGray),
        ),
    ];

    if let Some(detail) = connection.upstream_detail {
        lines.push(Line::styled(
            format!("upstream detail: {detail}"),
            Style::default().fg(Color::LightYellow),
        ));
    } else if let Some(banner) = view_model::status_banner(state) {
        let color = status_style(banner.level);
        lines.push(Line::styled(banner.message, color));
        if let Some(lifecycle_error) = state.daemon_lifecycle_error.as_deref() {
            lines.push(Line::styled(
                format!("daemon: {lifecycle_error}"),
                Style::default().fg(Color::DarkGray),
            ));
        }
    } else {
        lines.push(Line::from(selection_summary(state)));
        if let Some(error) = state.daemon_lifecycle_error.as_deref() {
            lines.push(Line::styled(
                format!("daemon: {error}"),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    Paragraph::new(Text::from(lines)).block(Block::default().title("Shell").borders(Borders::ALL))
}

pub(super) fn render_footer(state: &AppState) -> Paragraph<'static> {
    let mut lines = Vec::new();
    if state.show_help {
        lines.push(Line::from(
            "views: 1 overview  2 threads  3 collaboration  4 supervisor  tab next view",
        ));
        lines.push(Line::from(help_navigation_line(state.current_view)));
    } else {
        lines.push(Line::styled(
            format!(
                "keys: 1/2/3/4 views  tab next  {}  r refresh  ? help  q quit",
                match state.current_view {
                    TopLevelView::Overview => "j/k no-op",
                    TopLevelView::Threads => "j/k threads",
                    TopLevelView::Collaboration => "j/k selection  h/l list focus",
                    TopLevelView::Supervisor => {
                        "m refresh models  s start daemon  x stop daemon  R restart daemon"
                    }
                }
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        if let Some(error) = state.daemon_lifecycle_error.as_deref() {
            lines.push(Line::styled(
                format!("daemon: {error}"),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::styled(
            format!("focus: {}", title_case_view_label(state.current_view)),
            Style::default().fg(Color::DarkGray),
        ));
    }

    Paragraph::new(Text::from(lines))
        .block(Block::default().title("Keys").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

fn selection_summary(state: &AppState) -> String {
    match state.current_view {
        TopLevelView::Overview => format!(
            "selected thread={}  selected stream={}  selected unit={}",
            state.selected_thread_id.as_deref().unwrap_or("-"),
            state.selected_workstream_id.as_deref().unwrap_or("-"),
            state.selected_work_unit_id.as_deref().unwrap_or("-")
        ),
        TopLevelView::Threads => format!(
            "selected thread={}  recent events={}",
            state.selected_thread_id.as_deref().unwrap_or("-"),
            state.recent_events.len()
        ),
        TopLevelView::Collaboration => format!(
            "collaboration focus={}  selected stream={}  selected unit={}",
            view_model::collaboration_focus_label(state.collaboration_focus),
            state.selected_workstream_id.as_deref().unwrap_or("-"),
            state.selected_work_unit_id.as_deref().unwrap_or("-")
        ),
        TopLevelView::Supervisor => format!(
            "models={}  selected_thread={}",
            state.daemon_models.len(),
            state.selected_thread_id.as_deref().unwrap_or("-"),
        ),
    }
}

fn help_navigation_line(view: TopLevelView) -> &'static str {
    match view {
        TopLevelView::Overview => "nav: overview is read-heavy  r refresh  ? help  q quit",
        TopLevelView::Threads => "nav: j/k thread selection  r refresh  ? help  q quit",
        TopLevelView::Collaboration => {
            "nav: j/k move selected list  h/l switch workstreams/work_units  r refresh  ? help  q quit"
        }
        TopLevelView::Supervisor => {
            "nav: m reload models  s start daemon  x request daemon stop  R restart daemon  r refresh  ? help  q quit"
        }
    }
}
