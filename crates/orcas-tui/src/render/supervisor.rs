use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{AppState, DaemonLifecycleState};

use crate::view_model::shared::{daemon_lifecycle_label, daemon_phase_label};

use super::shared::{
    emphasis_style, focus_block_style, lifecycle_style, metadata_style, row_style, status_style,
};

pub(super) fn render_view(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let compact = area.width < 130 || area.height < 30;

    let layout = if compact {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Min(8),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(11),
                Constraint::Length(12),
                Constraint::Min(10),
            ])
            .split(area)
    };

    frame.render_widget(render_daemon_status(state), layout[0]);
    frame.render_widget(render_models(state), layout[1]);
    frame.render_widget(render_controls(state), layout[2]);
}

fn render_daemon_status(state: &AppState) -> Paragraph<'static> {
    let daemon = state.daemon.as_ref();
    let has_focus = state.current_view == crate::app::TopLevelView::Supervisor;
    let mut lines = Vec::new();
    let lifecycle = daemon_lifecycle_label(state.daemon_lifecycle);
    lines.push(Line::styled(
        format!("Daemon lifecycle: {lifecycle}"),
        lifecycle_style(state.daemon_lifecycle),
    ));
    lines.push(Line::styled(
        format!(
            "daemon: {}  upstream: {}  clients: {}  threads: {}  reconnect: {}",
            daemon_phase_label(state.daemon_phase),
            daemon
                .map(|daemon| daemon.upstream.status.clone())
                .unwrap_or_else(|| "disconnected".to_string()),
            daemon.map_or(0, |daemon| daemon.client_count),
            daemon.map_or(state.threads.len(), |daemon| daemon.known_threads),
            state.reconnect_attempt,
        ),
        metadata_style(),
    ));
    lines.push(Line::styled(
        format!(
            "socket: {}",
            daemon
                .map(|daemon| daemon.socket_path.clone())
                .unwrap_or_else(|| "unavailable".to_string()),
        ),
        metadata_style(),
    ));
    if let Some(detail) = daemon.and_then(|daemon| daemon.upstream.detail.as_ref()) {
        lines.push(Line::styled(
            format!("upstream detail: {detail}"),
            Style::default().fg(Color::LightYellow),
        ));
    }
    if let Some(error) = state.daemon_lifecycle_error.as_deref() {
        lines.push(Line::styled(
            format!("status detail: {error}"),
            status_style(
                if matches!(state.daemon_lifecycle, DaemonLifecycleState::Failed) {
                    crate::app::BannerLevel::Error
                } else {
                    crate::app::BannerLevel::Warning
                },
            ),
        ));
    }
    if let Some(runtime) = daemon.map(|daemon| &daemon.runtime) {
        lines.push(Line::styled(
            format!(
                "runtime: {} {} {}",
                runtime.version, runtime.build_fingerprint, runtime.binary_path
            ),
            metadata_style(),
        ));
        lines.push(Line::styled(
            format!("metadata path: {}", runtime.metadata_path),
            metadata_style(),
        ));
    }

    Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Supervisor Daemon")
            .borders(Borders::ALL)
            .border_style(focus_block_style(has_focus)),
    )
}

fn render_models(state: &AppState) -> Paragraph<'static> {
    let mut lines = Vec::new();
    let is_inflight = matches!(
        state.daemon_lifecycle,
        DaemonLifecycleState::Starting
            | DaemonLifecycleState::Stopping
            | DaemonLifecycleState::Restarting
    );

    if is_inflight {
        lines.push(Line::styled(
            "model update deferred during daemon transition",
            emphasis_style(),
        ));
    }
    if state.models_loading {
        lines.push(Line::styled("loading models...", emphasis_style()));
    }

    if state.daemon_lifecycle == DaemonLifecycleState::Stopped {
        lines.push(Line::styled(
            "No models loaded. Start daemon to refresh model list.".to_string(),
            Style::default().fg(Color::DarkGray),
        ));
    } else if state.daemon_models.is_empty() {
        if state.models_loading {
            lines.push(Line::styled(
                "no models loaded yet".to_string(),
                metadata_style(),
            ));
        } else {
            lines.push(Line::styled(
                "No models loaded. Press m to refresh models.".to_string(),
                metadata_style(),
            ));
        }
    } else {
        for model in state.daemon_models.iter().take(18) {
            let mut prefix = if model.is_default { "* " } else { "  " }.to_string();
            if model.hidden {
                prefix.push_str("h ");
            }
            let row = Line::styled(
                format!(
                    "{prefix}{} [{}]{}",
                    model.id,
                    model.display_name,
                    if model.is_default { " (default)" } else { "" }
                ),
                row_style(false),
            );
            lines.push(row);
        }
        if state.daemon_models.len() > 18 {
            lines.push(Line::styled(
                format!("+ {} more models", state.daemon_models.len() - 18),
                emphasis_style(),
            ));
        }
    }

    Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title("Available Models")
                .borders(Borders::ALL)
                .border_style(focus_block_style(matches!(
                    state.current_view,
                    crate::app::TopLevelView::Supervisor
                ))),
        )
        .wrap(Wrap { trim: true })
}

fn render_controls(state: &AppState) -> Paragraph<'static> {
    let mut lines = Vec::new();
    let in_flight = matches!(
        state.daemon_lifecycle,
        DaemonLifecycleState::Starting
            | DaemonLifecycleState::Stopping
            | DaemonLifecycleState::Restarting
    );
    let lifecycle = daemon_lifecycle_label(state.daemon_lifecycle);
    lines.push(Line::styled(
        format!("status: {lifecycle}"),
        lifecycle_style(state.daemon_lifecycle),
    ));
    lines.push(Line::styled(
        "actions: m refresh models  s start daemon  x stop daemon  R restart daemon",
        metadata_style(),
    ));
    lines.push(Line::styled(
        if in_flight {
            "daemon command in progress: repeated lifecycle keys are ignored".to_string()
        } else if state.daemon_lifecycle == DaemonLifecycleState::Failed {
            "daemon failure state: use restart (R) or stop/start again once fixed".to_string()
        } else {
            "lifecycle commands can be triggered at any time".to_string()
        },
        if in_flight {
            emphasis_style()
        } else if state.daemon_lifecycle == DaemonLifecycleState::Failed {
            status_style(crate::app::BannerLevel::Error)
        } else {
            metadata_style()
        },
    ));
    if state.daemon_lifecycle == DaemonLifecycleState::Failed {
        if let Some(error) = state.daemon_lifecycle_error.as_deref() {
            lines.push(Line::styled(
                format!("last failure: {error}"),
                status_style(crate::app::BannerLevel::Error),
            ));
        }
    } else if let Some(daemon) = state.daemon.as_ref() {
        if in_flight {
            lines.push(Line::styled(
                format!(
                    "endpoint={}  codex={}",
                    daemon.upstream.endpoint, daemon.codex_endpoint
                ),
                metadata_style(),
            ));
        } else {
            lines.push(Line::styled(
                format!(
                    "runtime pid={}  codex={}",
                    daemon.runtime.pid, daemon.codex_endpoint
                ),
                metadata_style(),
            ));
        }
    } else {
        lines.push(Line::styled(
            "daemon metadata not loaded yet.".to_string(),
            metadata_style(),
        ));
    }

    Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title("Controls")
                .borders(Borders::ALL)
                .border_style(focus_block_style(true)),
        )
        .wrap(Wrap { trim: true })
}
