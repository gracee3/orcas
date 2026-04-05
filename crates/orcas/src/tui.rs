use std::io::{self, IsTerminal, Read, Stdout, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use orcas_core::ipc;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::service::SupervisorService;

const HEADER_HEIGHT: u16 = 4;
const FOOTER_HEIGHT: u16 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusPane {
    Workstreams,
    Threads,
}

#[derive(Debug, Clone)]
enum SessionStatus {
    Starting,
    Running,
    Exited(String),
    Failed(String),
}

impl SessionStatus {
    fn label(&self) -> String {
        match self {
            Self::Starting => "starting".to_string(),
            Self::Running => "running".to_string(),
            Self::Exited(status) => format!("exited: {status}"),
            Self::Failed(error) => format!("failed: {error}"),
        }
    }

    fn is_live(&self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }
}

struct LiveSession {
    thread_id: String,
    workstream_id: String,
    workstream_title: String,
    cwd: PathBuf,
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send>>>,
    status: Arc<Mutex<SessionStatus>>,
    started_at: Instant,
}

impl LiveSession {
    fn launch(
        service: &SupervisorService,
        thread: &ipc::ThreadSummary,
        workstream: &ipc::WorkstreamSummary,
        cols: u16,
        rows: u16,
    ) -> Result<Self> {
        let _ = service.prepare_shared_app_server_auth()?;
        if !thread.cwd.is_empty() && Path::new(&thread.cwd).is_dir() {
            service.trust_shared_app_server_projects(&[Path::new(&thread.cwd)])?;
        }
        let workstream_title = workstream.title.clone();
        let cwd = if thread.cwd.is_empty() {
            PathBuf::from(".")
        } else {
            PathBuf::from(&thread.cwd)
        };

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows.max(12),
                cols: cols.max(40),
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("open pty for Codex TUI")?;

        let mut command = CommandBuilder::new(service.config.codex.binary_path.clone());
        command.arg("resume");
        command.arg(&thread.id);
        command.env("CODEX_HOME", service.shared_app_server_codex_home());
        command.env("CODEX_SQLITE_HOME", service.shared_app_server_sqlite_home());
        if Path::new(&thread.cwd).is_dir() {
            command.cwd(&thread.cwd);
        }

        let child = pair
            .slave
            .spawn_command(command)
            .context("spawn child Codex TUI")?;
        let reader = pair
            .master
            .try_clone_reader()
            .context("clone PTY reader")?;
        let writer = pair.master.take_writer().context("take PTY writer")?;
        let master = Arc::new(Mutex::new(pair.master));
        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows.max(12), cols.max(40), 0)));
        let status = Arc::new(Mutex::new(SessionStatus::Starting));

        Self::spawn_reader(reader, parser.clone(), status.clone());
        let session = Self {
            thread_id: thread.id.clone(),
            workstream_id: workstream.id.clone(),
            workstream_title,
            cwd,
            parser,
            writer: Arc::new(Mutex::new(writer)),
            master,
            child: Arc::new(Mutex::new(child)),
            status,
            started_at: Instant::now(),
        };
        session.set_status(SessionStatus::Running);
        Ok(session)
    }

    fn spawn_reader(
        mut reader: Box<dyn Read + Send>,
        parser: Arc<Mutex<vt100::Parser>>,
        status: Arc<Mutex<SessionStatus>>,
    ) {
        thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        if let Ok(mut guard) = status.lock() {
                            if guard.is_live() {
                                *guard = SessionStatus::Exited("pty closed".to_string());
                            }
                        }
                        break;
                    }
                    Ok(len) => {
                        if let Ok(mut guard) = parser.lock() {
                            guard.process(&buffer[..len]);
                        }
                    }
                    Err(error) => {
                        if let Ok(mut guard) = status.lock() {
                            *guard = SessionStatus::Failed(format!("pty read failed: {error}"));
                        }
                        break;
                    }
                }
            }
        });
    }

    fn short_label(&self) -> String {
        let short = self.thread_id.chars().take(8).collect::<String>();
        format!("{}:{}", self.workstream_title, short)
    }

    fn set_status(&self, status: SessionStatus) {
        if let Ok(mut guard) = self.status.lock() {
            *guard = status;
        }
    }

    fn status(&self) -> SessionStatus {
        self.status
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| SessionStatus::Failed("status lock poisoned".to_string()))
    }

    fn screen_text(&self) -> String {
        self.parser
            .lock()
            .map(|guard| guard.screen().contents())
            .unwrap_or_else(|_| "[session buffer unavailable]".to_string())
    }

    fn send_key(&self, key: KeyEvent) -> Result<()> {
        let bytes = key_to_bytes(key);
        if bytes.is_empty() {
            return Ok(());
        }
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("session writer lock poisoned"))?;
        writer
            .write_all(&bytes)
            .context("write key event to Codex PTY")?;
        writer.flush().ok();
        Ok(())
    }

    fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        let rows = rows.max(12);
        let cols = cols.max(40);
        if let Ok(mut master) = self.master.lock() {
            let _ = master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
        Ok(())
    }

    fn poll_exit(&self) {
        if let Ok(mut child) = self.child.lock() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.set_status(SessionStatus::Exited(status.to_string()));
                }
                Ok(None) => {}
                Err(error) => {
                    self.set_status(SessionStatus::Failed(format!("exit poll failed: {error}")));
                }
            }
        }
    }

    fn terminate(&self) -> Result<()> {
        if let Ok(mut child) = self.child.lock() {
            child.kill().context("terminate Codex child")?;
            self.set_status(SessionStatus::Exited("terminated".to_string()));
        }
        Ok(())
    }
}

struct SessionTab {
    session: LiveSession,
}

impl SessionTab {
    fn title(&self) -> String {
        self.session.short_label()
    }
}

struct DashboardState {
    snapshot: ipc::StateSnapshot,
    focus: FocusPane,
    workstream_index: usize,
    thread_index: usize,
    status: String,
    last_refresh: Instant,
    show_hud: bool,
    active_tab: Option<usize>,
    sessions: Vec<SessionTab>,
}

impl DashboardState {
    fn new(snapshot: ipc::StateSnapshot) -> Self {
        Self {
            snapshot,
            focus: FocusPane::Workstreams,
            workstream_index: 0,
            thread_index: 0,
            status: "dashboard ready".to_string(),
            last_refresh: Instant::now(),
            show_hud: true,
            active_tab: None,
            sessions: Vec::new(),
        }
    }

    fn workstreams(&self) -> &[ipc::WorkstreamSummary] {
        &self.snapshot.collaboration.workstreams
    }

    fn filtered_threads(&self) -> Vec<&ipc::ThreadSummary> {
        let selected = self.selected_workstream_id();
        self.snapshot
            .threads
            .iter()
            .filter(|thread| match selected {
                Some(workstream_id) => {
                    thread.owner_workstream_id.as_deref() == Some(workstream_id)
                        || thread.runtime_workstream_id.as_deref() == Some(workstream_id)
                }
                None => true,
            })
            .collect()
    }

    fn selected_workstream_id(&self) -> Option<&str> {
        self.workstreams()
            .get(self.workstream_index)
            .map(|workstream| workstream.id.as_str())
    }

    fn selected_workstream(&self) -> Option<&ipc::WorkstreamSummary> {
        self.workstreams().get(self.workstream_index)
    }

    fn selected_thread(&self) -> Option<&ipc::ThreadSummary> {
        self.filtered_threads().get(self.thread_index).copied()
    }

    fn normalize_selection(&mut self) {
        if self.workstream_index >= self.workstreams().len() {
            self.workstream_index = 0;
        }
        let thread_count = self.filtered_threads().len();
        if self.thread_index >= thread_count {
            self.thread_index = 0;
        }
        if matches!(self.focus, FocusPane::Threads) && thread_count == 0 {
            self.focus = FocusPane::Workstreams;
        }
        self.normalize_active_tab();
    }

    fn normalize_active_tab(&mut self) {
        if let Some(index) = self.active_tab && index >= self.sessions.len() {
            self.active_tab = None;
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            FocusPane::Workstreams => {
                let len = self.workstreams().len();
                if len > 0 {
                    self.workstream_index = self.workstream_index.saturating_sub(1);
                }
                self.thread_index = 0;
            }
            FocusPane::Threads => {
                let len = self.filtered_threads().len();
                if len > 0 {
                    self.thread_index = self.thread_index.saturating_sub(1);
                }
            }
        }
    }

    fn move_down(&mut self) {
        match self.focus {
            FocusPane::Workstreams => {
                let len = self.workstreams().len();
                if len > 0 && self.workstream_index + 1 < len {
                    self.workstream_index += 1;
                }
                self.thread_index = 0;
            }
            FocusPane::Threads => {
                let len = self.filtered_threads().len();
                if len > 0 && self.thread_index + 1 < len {
                    self.thread_index += 1;
                }
            }
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPane::Workstreams => FocusPane::Threads,
            FocusPane::Threads => FocusPane::Workstreams,
        };
        self.normalize_selection();
    }

    fn thread_count(&self) -> usize {
        self.filtered_threads().len()
    }

    fn active_session(&self) -> Option<&SessionTab> {
        self.active_tab.and_then(|index| self.sessions.get(index))
    }

    fn active_session_mut(&mut self) -> Option<&mut SessionTab> {
        self.active_tab.and_then(|index| self.sessions.get_mut(index))
    }

    fn focus_session(&mut self, thread_id: &str) -> Option<usize> {
        if let Some(index) = self
            .sessions
            .iter()
            .position(|tab| tab.session.thread_id == thread_id)
        {
            self.active_tab = Some(index);
            self.show_hud = false;
            return Some(index);
        }
        None
    }

    fn open_thread_session(
        &mut self,
        service: &SupervisorService,
        thread: &ipc::ThreadSummary,
        workstream: &ipc::WorkstreamSummary,
        rows: u16,
        cols: u16,
    ) -> Result<()> {
        if let Some(index) = self.focus_session(&thread.id) {
            self.status = format!("focused existing session {}", thread.id);
            self.active_tab = Some(index);
            return Ok(());
        }
        let session = LiveSession::launch(service, thread, workstream, cols, rows)?;
        self.sessions.push(SessionTab { session });
        self.active_tab = Some(self.sessions.len().saturating_sub(1));
        self.show_hud = false;
        self.status = format!("opened live Codex session for {}", thread.id);
        Ok(())
    }

    fn next_session(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let next = match self.active_tab {
            Some(index) => (index + 1) % self.sessions.len(),
            None => 0,
        };
        self.active_tab = Some(next);
        self.status = format!("focused tab {}", self.sessions[next].title());
    }

    fn previous_session(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let prev = match self.active_tab {
            Some(0) | None => self.sessions.len().saturating_sub(1),
            Some(index) => index.saturating_sub(1),
        };
        self.active_tab = Some(prev);
        self.status = format!("focused tab {}", self.sessions[prev].title());
    }

    fn send_key_to_active_session(&self, key: KeyEvent) -> Result<()> {
        if let Some(session) = self.active_session() {
            session.session.send_key(key)?;
        }
        Ok(())
    }

    fn terminate_active_session(&mut self) -> Result<()> {
        let Some(session) = self.active_session() else {
            self.status = "no active session to terminate".to_string();
            return Ok(());
        };
        session.session.terminate()?;
        self.status = format!("terminated session {}", session.session.thread_id);
        Ok(())
    }

    fn poll_session_statuses(&self) {
        for session in &self.sessions {
            session.session.poll_exit();
        }
    }

    fn resize_sessions(&self, rows: u16, cols: u16) {
        for session in &self.sessions {
            let _ = session.session.resize(rows, cols);
        }
    }
}

pub async fn run_dashboard(service: SupervisorService) -> Result<()> {
    if !io::stdout().is_terminal() {
        bail!("orcas tui requires an interactive terminal");
    }

    enable_raw_mode().context("enable terminal raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide).context("enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("create dashboard terminal")?;

    let result = run_dashboard_loop(&service, &mut terminal).await;

    cleanup_terminal(&mut terminal);
    result
}

async fn run_dashboard_loop(
    service: &SupervisorService,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<()> {
    let snapshot = service.dashboard_snapshot().await?;
    let mut state = DashboardState::new(snapshot);
    let refresh_interval = Duration::from_millis(750);

    loop {
        state.normalize_selection();
        state.poll_session_statuses();
        let size = terminal.size().context("read terminal size")?;
        let content_rows = size.height.saturating_sub(HEADER_HEIGHT + FOOTER_HEIGHT);
        state.resize_sessions(content_rows, size.width);

        terminal
            .draw(|frame| render_dashboard(frame, &state))
            .context("render Orcas dashboard")?;

        if state.last_refresh.elapsed() >= refresh_interval {
            state.snapshot = service.dashboard_snapshot().await?;
            state.last_refresh = Instant::now();
            continue;
        }

        if let Some(key) = poll_key_event(Duration::from_millis(125)).await? {
            match key.code {
                KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.status = "wrapper closed".to_string();
                    break;
                }
                KeyCode::F(2) => {
                    state.show_hud = !state.show_hud;
                    state.status = if state.show_hud {
                        "opened HUD".to_string()
                    } else {
                        "closed HUD".to_string()
                    };
                }
                KeyCode::F(5) => {
                    state.snapshot = service.dashboard_snapshot().await?;
                    state.last_refresh = Instant::now();
                    state.status = "refreshed daemon snapshot".to_string();
                }
                KeyCode::F(6) => {
                    state.next_session();
                }
                KeyCode::F(7) => {
                    state.previous_session();
                }
                KeyCode::F(8) => {
                    state.terminate_active_session()?;
                }
                _ if state.show_hud => match key.code {
                    KeyCode::Tab => {
                        state.toggle_focus();
                        state.status = match state.focus {
                            FocusPane::Workstreams => "focused workstreams".to_string(),
                            FocusPane::Threads => "focused threads".to_string(),
                        };
                    }
                    KeyCode::Up => state.move_up(),
                    KeyCode::Down => state.move_down(),
                    KeyCode::Enter => match state.focus {
                        FocusPane::Workstreams => {
                            if state.thread_count() > 0 {
                                state.focus = FocusPane::Threads;
                                state.thread_index = 0;
                                state.status = "focused threads".to_string();
                            } else {
                                state.status = "selected workstream has no threads".to_string();
                            }
                        }
                        FocusPane::Threads => {
                            let selected_thread = state.selected_thread().cloned();
                            let selected_workstream = state.selected_workstream().cloned();
                            if let (Some(thread), Some(workstream)) =
                                (selected_thread, selected_workstream)
                            {
                                state.open_thread_session(
                                    service,
                                    &thread,
                                    &workstream,
                                    content_rows,
                                    size.width,
                                )?;
                            } else {
                                state.status = "no thread selected".to_string();
                            }
                        }
                    },
                    KeyCode::Esc => {
                        state.show_hud = false;
                        state.status = "closed HUD".to_string();
                    }
                    _ => {}
                },
                _ => {
                    if state.active_session().is_some() {
                        state.send_key_to_active_session(key)?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn render_dashboard(frame: &mut ratatui::Frame<'_>, state: &DashboardState) {
    let root = frame.size();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Min(8),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(root);

    render_header(frame, layout[0], state);
    render_main(frame, layout[1], state);
    render_footer(frame, layout[2], state);

    if state.show_hud {
        render_hud_overlay(frame, layout[1], state);
    }
}

fn render_header(frame: &mut ratatui::Frame<'_>, area: Rect, state: &DashboardState) {
    let daemon = &state.snapshot.daemon;
    let active = state
        .snapshot
        .session
        .active_thread_id
        .as_deref()
        .unwrap_or("-");
    let tabs = if state.sessions.is_empty() {
        "sessions: none".to_string()
    } else {
        state
            .sessions
            .iter()
            .enumerate()
            .map(|(index, session)| {
                if Some(index) == state.active_tab {
                    format!("[{}]", session.title())
                } else {
                    session.title()
                }
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let title = Line::from(vec![
        Span::styled(" Orcas TUI ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" tabbed Codex shell "),
        Span::raw(" | "),
        Span::raw(format!("daemon={}", daemon.upstream.status)),
        Span::raw(" | "),
        Span::raw(format!("active_thread={active}")),
    ]);
    let session_tabs = Line::from(vec![
        Span::styled(" tabs ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(tabs),
    ]);
    let help = Line::from(vec![
        Span::raw("ctrl+q"),
        Span::raw(" quit  "),
        Span::raw("f2"),
        Span::raw(" hud  "),
        Span::raw("f5"),
        Span::raw(" refresh  "),
        Span::raw("f6/f7"),
        Span::raw(" switch tabs  "),
        Span::raw("f8"),
        Span::raw(" terminate session"),
    ]);
    let status = Line::from(vec![
        Span::styled(" status ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(state.status.clone()),
    ]);
    let lines = vec![title, session_tabs, help, status];
    let block = Block::default().borders(Borders::ALL).title("Orcas Dashboard");
    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn render_main(frame: &mut ratatui::Frame<'_>, area: Rect, state: &DashboardState) {
    if let Some(session) = state.active_session() {
        let text = session.session.screen_text();
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "Codex Session: {} ({})",
                session.session.workstream_title, session.session.thread_id
            ));
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);
    render_workstream_list(frame, chunks[0], state);
    render_thread_list(frame, chunks[1], state);
}

fn render_workstream_list(frame: &mut ratatui::Frame<'_>, area: Rect, state: &DashboardState) {
    let workstream_items: Vec<ListItem<'_>> = state
        .workstreams()
        .iter()
        .map(|workstream| {
            let mut lines = Vec::new();
            lines.push(Line::from(vec![Span::styled(
                workstream.title.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(format!("id: {}", workstream.id)));
            lines.push(Line::from(format!("status: {:?}", workstream.status)));
            lines.push(Line::from(format!("priority: {}", workstream.priority)));
            ListItem::new(lines)
        })
        .collect();
    let mut workstream_state = ListState::default();
    if !state.workstreams().is_empty() {
        workstream_state.select(Some(state.workstream_index));
    }
    let workstream_block = Block::default().borders(Borders::ALL).title(
        if matches!(state.focus, FocusPane::Workstreams) {
            "Workstreams (focused)"
        } else {
            "Workstreams"
        },
    );
    let workstream_list = List::new(workstream_items)
        .block(workstream_block)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(ratatui::style::Color::Blue),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(workstream_list, area, &mut workstream_state);
}

fn render_thread_list(frame: &mut ratatui::Frame<'_>, area: Rect, state: &DashboardState) {
    let thread_items: Vec<ListItem<'_>> = state
        .filtered_threads()
        .into_iter()
        .map(|thread| {
            let lines = vec![
                Line::from(vec![Span::styled(
                    thread.id.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(format!("status: {}", thread.status)),
                Line::from(format!(
                    "owner/runtime: {}/{}",
                    thread.owner_workstream_id.as_deref().unwrap_or("-"),
                    thread.runtime_workstream_id.as_deref().unwrap_or("-")
                )),
                Line::from(thread.preview.clone().replace('\n', " ")),
            ];
            ListItem::new(lines)
        })
        .collect();
    let mut thread_state = ListState::default();
    if !thread_items.is_empty() {
        thread_state.select(Some(state.thread_index));
    }
    let thread_block = Block::default().borders(Borders::ALL).title(
        if matches!(state.focus, FocusPane::Threads) {
            "Threads (focused)"
        } else {
            "Threads"
        },
    );
    let thread_list = List::new(thread_items)
        .block(thread_block)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(ratatui::style::Color::Blue),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(thread_list, area, &mut thread_state);
}

fn render_hud_overlay(frame: &mut ratatui::Frame<'_>, area: Rect, state: &DashboardState) {
    let popup = centered_rect(82, 80, area);
    let panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8), Constraint::Length(5)])
        .split(popup);

    let hud_title = Block::default()
        .borders(Borders::ALL)
        .title("Orcas HUD");
    let help = Line::from(vec![
        Span::raw("tab"),
        Span::raw(" switch lane  "),
        Span::raw("enter"),
        Span::raw(" open/focus  "),
        Span::raw("esc"),
        Span::raw(" close hud"),
    ]);
    frame.render_widget(Paragraph::new(help).block(hud_title), panel[0]);

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(panel[1]);
    render_workstream_list(frame, content[0], state);
    render_thread_list(frame, content[1], state);

    let selected_workstream = state
        .selected_workstream()
        .map(|workstream| {
            format!(
                "selected_workstream: {} | id={} | status={:?} | objective={}",
                workstream.title, workstream.id, workstream.status, workstream.objective
            )
        })
        .unwrap_or_else(|| "selected_workstream: -".to_string());
    let selected_thread = state
        .selected_thread()
        .map(|thread| {
            format!(
                "selected_thread: {} | cwd={} | model_provider={} | recent_event={}",
                thread.id,
                thread.cwd,
                thread.model_provider,
                thread.recent_event.as_deref().unwrap_or("-")
            )
        })
        .unwrap_or_else(|| "selected_thread: -".to_string());
    let active_session = state
        .active_session()
        .map(|session| {
            format!(
                "active_session: {} | status={} | cwd={} | age={}s",
                session.session.thread_id,
                session.session.status().label(),
                session.session.cwd.display(),
                session.session.started_at.elapsed().as_secs()
            )
        })
        .unwrap_or_else(|| "active_session: -".to_string());
    let footer = vec![
        Line::from(selected_workstream),
        Line::from(selected_thread),
        Line::from(active_session),
    ];
    let block = Block::default().borders(Borders::ALL).title("HUD Details");
    frame.render_widget(Paragraph::new(footer).block(block).wrap(Wrap { trim: true }), panel[2]);
}

fn render_footer(frame: &mut ratatui::Frame<'_>, area: Rect, state: &DashboardState) {
    let selected_workstream = state
        .selected_workstream()
        .map(|workstream| {
            format!(
                "selected_workstream: {} | id={} | status={:?} | objective={}",
                workstream.title, workstream.id, workstream.status, workstream.objective
            )
        })
        .unwrap_or_else(|| "selected_workstream: -".to_string());
    let selected_thread = state
        .selected_thread()
        .map(|thread| {
            format!(
                "selected_thread: {} | cwd={} | model_provider={} | recent_event={}",
                thread.id,
                thread.cwd,
                thread.model_provider,
                thread.recent_event.as_deref().unwrap_or("-")
            )
        })
        .unwrap_or_else(|| "selected_thread: -".to_string());
    let active_session = state
        .active_session()
        .map(|session| {
            format!(
                "active_session: {} | status={} | cwd={} | age={}s",
                session.session.thread_id,
                session.session.status().label(),
                session.session.cwd.display(),
                session.session.started_at.elapsed().as_secs()
            )
        })
        .unwrap_or_else(|| "active_session: -".to_string());
    let lines = vec![
        Line::from(state.status.clone()),
        Line::from(selected_workstream),
        Line::from(selected_thread),
        Line::from(active_session),
    ];
    let block = Block::default().borders(Borders::ALL).title("Details");
    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

async fn poll_key_event(timeout: Duration) -> Result<Option<KeyEvent>> {
    tokio::task::spawn_blocking(move || -> Result<Option<KeyEvent>> {
        if event::poll(timeout).context("poll dashboard input")? {
            match event::read().context("read dashboard input")? {
                Event::Key(key) => Ok(Some(key)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    })
    .await
    .context("join dashboard input task")?
}

fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let byte = match c {
                    '@' => 0x00,
                    ' ' => 0x00,
                    '[' => 0x1b,
                    '\\' => 0x1c,
                    ']' => 0x1d,
                    '^' => 0x1e,
                    '_' => 0x1f,
                    'a'..='z' => (c as u8 - b'a') + 1,
                    'A'..='Z' => (c as u8 - b'A') + 1,
                    _ => c as u8,
                };
                vec![byte]
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                let mut bytes = vec![0x1b];
                bytes.extend(c.encode_utf8(&mut [0; 4]).as_bytes());
                bytes
            } else {
                let mut buf = [0; 4];
                c.encode_utf8(&mut buf).as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        _ => Vec::new(),
    }
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen, Show);
    let _ = terminal.show_cursor();
}
