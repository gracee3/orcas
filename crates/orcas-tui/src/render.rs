use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{AppState, BannerLevel, DaemonConnectionPhase, NavigationFocus};
use crate::view_model;

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    let compact = frame.area().width < 150 || frame.area().height < 40;
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(layout[1]);

    let thread_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(main[0]);

    let collaboration = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(if compact { 8 } else { 7 }),
            Constraint::Length(if compact { 10 } else { 9 }),
            Constraint::Min(8),
        ])
        .split(main[1]);

    let collaboration_top = Layout::default()
        .direction(if compact {
            Direction::Vertical
        } else {
            Direction::Horizontal
        })
        .constraints(if compact {
            vec![Constraint::Length(4), Constraint::Min(4)]
        } else {
            vec![Constraint::Length(34), Constraint::Min(30)]
        })
        .split(collaboration[1]);

    let collaboration_middle = Layout::default()
        .direction(if compact {
            Direction::Vertical
        } else {
            Direction::Horizontal
        })
        .constraints(if compact {
            vec![Constraint::Length(5), Constraint::Min(4)]
        } else {
            vec![Constraint::Percentage(60), Constraint::Percentage(40)]
        })
        .split(collaboration[2]);

    let collaboration_bottom = Layout::default()
        .direction(if compact {
            Direction::Vertical
        } else {
            Direction::Horizontal
        })
        .constraints(if compact {
            vec![Constraint::Length(8), Constraint::Min(8)]
        } else {
            vec![Constraint::Percentage(44), Constraint::Percentage(56)]
        })
        .split(collaboration[3]);

    frame.render_widget(render_status(state), layout[0]);
    frame.render_widget(render_threads(state), thread_panels[0]);
    frame.render_widget(render_thread_detail(state), thread_panels[1]);
    frame.render_widget(render_collaboration_status(state), collaboration[0]);
    frame.render_widget(render_workstreams(state), collaboration_top[0]);
    frame.render_widget(render_workstream_detail(state), collaboration_top[1]);
    frame.render_widget(render_work_units(state), collaboration_middle[0]);
    frame.render_widget(render_assignments(state), collaboration_middle[1]);
    frame.render_widget(render_collaboration_detail(state), collaboration_bottom[0]);
    frame.render_widget(render_collaboration_history(state), collaboration_bottom[1]);
    frame.render_widget(render_event_log(state), layout[2]);
    frame.render_widget(render_prompt(state), layout[3]);
}

fn render_status(state: &AppState) -> Paragraph<'static> {
    let status = view_model::connection_status(state);
    let mut lines = vec![
        Line::styled("Orcas TUI", Style::default().add_modifier(Modifier::BOLD)),
        Line::from(format!("socket: {}", status.socket_path)),
        Line::from(format!(
            "daemon: {}  upstream: {}  clients: {}  threads: {}",
            match status.daemon_phase {
                DaemonConnectionPhase::Connected => "connected",
                DaemonConnectionPhase::Reconnecting => "reconnecting",
                DaemonConnectionPhase::Disconnected => "disconnected",
            },
            status.upstream_status,
            status.client_count,
            status.known_threads
        )),
    ];

    if let Some(detail) = status.upstream_detail {
        lines.push(Line::from(format!("detail: {detail}")));
    } else if let Some(banner) = view_model::status_banner(state) {
        let color = match banner.level {
            BannerLevel::Info => Color::Green,
            BannerLevel::Warning => Color::Yellow,
            BannerLevel::Error => Color::Red,
        };
        lines.push(Line::styled(
            banner.message,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    } else if state.show_help {
        lines.push(Line::from(
            "keys: q quit, r refresh, tab cycles panels, j/k move selected panel, i prompt",
        ));
    } else {
        lines.push(Line::from(
            "keys: q quit, r refresh, tab cycles panels, j/k move selected panel, ? help",
        ));
    }

    Paragraph::new(Text::from(lines)).block(Block::default().title("Daemon").borders(Borders::ALL))
}

fn render_threads(state: &AppState) -> Paragraph<'static> {
    let rows = view_model::thread_list(state).rows;
    let lines = if rows.is_empty() {
        vec![Line::from("No threads loaded.")]
    } else {
        rows.into_iter()
            .take(12)
            .map(|row| {
                let prefix = if row.selected { ">" } else { " " };
                let badge = row
                    .turn_badge
                    .as_ref()
                    .map(|badge| format!(" {{{badge}}}"))
                    .unwrap_or_default();
                Line::from(format!(
                    "{prefix} {} [{}{}] {}",
                    row.id, row.status, badge, row.preview
                ))
            })
            .collect()
    };
    Paragraph::new(Text::from(lines))
        .block(Block::default().title("Threads").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

fn render_thread_detail(state: &AppState) -> Paragraph<'static> {
    let detail = view_model::thread_detail(state);
    Paragraph::new(Text::from(
        detail.lines.into_iter().map(Line::from).collect::<Vec<_>>(),
    ))
    .block(Block::default().title(detail.title).borders(Borders::ALL))
    .wrap(Wrap { trim: false })
}

fn render_collaboration_status(state: &AppState) -> Paragraph<'static> {
    let status = view_model::collaboration_status(state);
    let focus = match status.focus {
        NavigationFocus::Threads => "threads",
        NavigationFocus::Workstreams => "workstreams",
        NavigationFocus::WorkUnits => "work_units",
    };
    Paragraph::new(Text::from(vec![Line::from(format!(
        "focus={}  ws={}  wu={}  active={}  review={}  history=selected work unit",
        focus,
        status.workstream_count,
        status.work_unit_count,
        status.active_assignment_count,
        status.review_count
    ))]))
    .block(
        Block::default()
            .title("Collaboration")
            .borders(Borders::ALL),
    )
    .wrap(Wrap { trim: true })
}

fn render_workstreams(state: &AppState) -> Paragraph<'static> {
    let rows = view_model::workstream_list(state).rows;
    let lines = if rows.is_empty() {
        vec![Line::from("No workstreams loaded.")]
    } else {
        rows.into_iter()
            .take(6)
            .map(|row| {
                let prefix = if row.selected { ">" } else { " " };
                Line::from(format!(
                    "{prefix} {} [{}] {}",
                    row.title, row.status, row.counts
                ))
            })
            .collect()
    };
    Paragraph::new(Text::from(lines))
        .block(Block::default().title("Workstreams").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

fn render_workstream_detail(state: &AppState) -> Paragraph<'static> {
    let detail = view_model::workstream_detail(state);
    Paragraph::new(Text::from(
        detail.lines.into_iter().map(Line::from).collect::<Vec<_>>(),
    ))
    .block(Block::default().title(detail.title).borders(Borders::ALL))
    .wrap(Wrap { trim: true })
}

fn render_work_units(state: &AppState) -> Paragraph<'static> {
    let rows = view_model::work_unit_list(state).rows;
    let lines = if rows.is_empty() {
        vec![Line::from("No work units loaded.")]
    } else {
        rows.into_iter()
            .take(8)
            .map(|row| {
                let prefix = if row.selected { ">" } else { " " };
                let review = if row.needs_supervisor_review {
                    " review"
                } else {
                    ""
                };
                Line::from(format!(
                    "{prefix} {} [{}] a={} parse={}{} decision={}",
                    row.title,
                    row.status,
                    row.current_assignment,
                    row.latest_report_parse_result,
                    review,
                    row.latest_decision
                ))
            })
            .collect()
    };
    Paragraph::new(Text::from(lines))
        .block(Block::default().title("Work Units").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

fn render_assignments(state: &AppState) -> Paragraph<'static> {
    let rows = view_model::assignment_list(state).rows;
    let lines = if rows.is_empty() {
        vec![Line::from("No active or pending assignments.")]
    } else {
        rows.into_iter()
            .take(8)
            .map(|row| {
                Line::from(format!(
                    "{} [{}] unit={} worker={} session={}",
                    row.id, row.status, row.work_unit_title, row.worker_id, row.worker_session_id
                ))
            })
            .collect()
    };
    Paragraph::new(Text::from(lines))
        .block(Block::default().title("Assignments").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

fn render_collaboration_detail(state: &AppState) -> Paragraph<'static> {
    let detail = view_model::collaboration_detail(state);
    Paragraph::new(Text::from(
        detail.lines.into_iter().map(Line::from).collect::<Vec<_>>(),
    ))
    .block(Block::default().title(detail.title).borders(Borders::ALL))
    .wrap(Wrap { trim: true })
}

fn render_collaboration_history(state: &AppState) -> Paragraph<'static> {
    let detail = view_model::collaboration_history(state);
    Paragraph::new(Text::from(
        detail.lines.into_iter().map(Line::from).collect::<Vec<_>>(),
    ))
    .block(Block::default().title(detail.title).borders(Borders::ALL))
    .wrap(Wrap { trim: false })
}

fn render_event_log(state: &AppState) -> Paragraph<'static> {
    let lines = view_model::event_log(state).lines;
    let text = if lines.is_empty() {
        vec![Line::from("No events yet.")]
    } else {
        lines
            .into_iter()
            .rev()
            .take(8)
            .rev()
            .map(Line::from)
            .collect()
    };
    Paragraph::new(Text::from(text))
        .block(Block::default().title("Event Log").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

fn render_prompt(state: &AppState) -> Paragraph<'static> {
    let prompt = view_model::prompt_box(state);
    let prefix = if prompt.active { "prompt>" } else { "prompt " };
    let suffix = if prompt.in_flight {
        " [waiting]"
    } else if prompt.active {
        " [editing]"
    } else {
        " [press i]"
    };
    Paragraph::new(Text::from(vec![Line::from(format!(
        "{prefix} {}{suffix}",
        prompt.text
    ))]))
    .block(Block::default().title("Prompt").borders(Borders::ALL))
}
