use crate::app::AppState;
use orcas_core::{
    CodexThreadAssignmentStatus, CodexThreadBootstrapState, CodexThreadSendPolicy, ipc,
};

use super::shared::{PanelViewModel, abbreviate, compact_line, lifecycle_label, timestamp_label};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadRowViewModel {
    pub id: String,
    pub status: String,
    pub turn_badge: Option<String>,
    pub assignment_badge: Option<String>,
    pub preview: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadListViewModel {
    pub rows: Vec<ThreadRowViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadDetailViewModel {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadsViewModel {
    pub list: ThreadListViewModel,
    pub summary: PanelViewModel,
    pub detail: ThreadDetailViewModel,
}

pub fn thread_list(state: &AppState) -> ThreadListViewModel {
    ThreadListViewModel {
        rows: state
            .threads
            .iter()
            .map(|thread| ThreadRowViewModel {
                id: thread.id.clone(),
                status: thread_status_label(state, thread),
                turn_badge: thread_turn_badge(state, &thread.id),
                assignment_badge: thread_assignment_badge(state, &thread.id),
                preview: abbreviate(&thread.preview.replace('\n', " "), 40),
                selected: state.selected_thread_id.as_deref() == Some(thread.id.as_str()),
            })
            .collect(),
    }
}

pub fn thread_summary(state: &AppState) -> PanelViewModel {
    let Some(thread_id) = state.selected_thread_id.as_ref() else {
        return PanelViewModel {
            title: "Selected Thread".to_string(),
            lines: vec!["No thread selected.".to_string()],
        };
    };

    let Some(summary) = state.threads.iter().find(|thread| thread.id == *thread_id) else {
        return PanelViewModel {
            title: format!("Selected Thread {thread_id}"),
            lines: vec!["Selected thread is no longer present.".to_string()],
        };
    };

    let mut lines = vec![
        format!("status: {}", thread_status_label(state, summary)),
        format!("loaded: {}", loaded_status_label(summary.loaded_status)),
        format!("monitor: {}", monitor_state_label(summary.monitor_state)),
        format!("cwd: {}", summary.cwd),
        format!("provider: {}", summary.model_provider),
        format!("scope: {}", summary.scope),
    ];

    if let Some(source_kind) = summary.source_kind.as_ref() {
        lines.push(format!("source: {source_kind}"));
    }
    if let Some(turn_id) = summary.active_turn_id.as_ref() {
        lines.push(format!("active turn: {turn_id}"));
    }
    if let Some(turn_id) = summary.last_seen_turn_id.as_ref() {
        lines.push(format!("last seen turn: {turn_id}"));
    }
    if let Some(assignment) = thread_assignment_for_display(state, thread_id) {
        lines.push(format!(
            "assignment: {} [{}]",
            assignment.assignment_id,
            codex_assignment_status_label(assignment.status)
        ));
        lines.push(format!(
            "binding: stream={} unit={} supervisor={}",
            assignment.workstream_id, assignment.work_unit_id, assignment.supervisor_id
        ));
        lines.push(format!(
            "policy: {}  bootstrap: {}",
            codex_send_policy_label(assignment.send_policy),
            codex_bootstrap_state_label(assignment.bootstrap_state)
        ));
    } else {
        lines.push("assignment: unassigned".to_string());
    }

    if let Some(turn_state) = latest_turn_state_for_thread(state, thread_id) {
        lines.push(format!(
            "latest turn: {} [{}] attachable={} terminal={}",
            turn_state.turn_id,
            lifecycle_label(&turn_state.lifecycle),
            turn_state.attachable,
            turn_state.terminal
        ));
        if let Some(event) = turn_state.recent_event.as_ref() {
            lines.push(format!("event: {}", abbreviate(&compact_line(event), 88)));
        }
        if let Some(output) = turn_state.recent_output.as_ref() {
            lines.push(format!("output: {}", abbreviate(&compact_line(output), 88)));
        }
    } else {
        lines.push("latest turn: no active lifecycle state loaded".to_string());
    }

    if let Some(output) = summary.recent_output.as_ref() {
        lines.push(format!(
            "recent output: {}",
            abbreviate(&compact_line(output), 88)
        ));
    }
    if let Some(event) = summary.recent_event.as_ref() {
        lines.push(format!(
            "recent event: {}",
            abbreviate(&compact_line(event), 88)
        ));
    }

    lines.push(format!(
        "detail: {}",
        state
            .thread_details
            .get(thread_id)
            .map(|thread| {
                let history = if thread.history_loaded {
                    "history loaded"
                } else {
                    "summary only"
                };
                format!("{} turns cached, {history}", thread.turns.len())
            })
            .unwrap_or_else(|| "loading on demand".to_string())
    ));

    PanelViewModel {
        title: format!("Selected Thread {}", summary.id),
        lines,
    }
}

pub fn thread_detail(state: &AppState) -> ThreadDetailViewModel {
    let Some(thread_id) = state.selected_thread_id.as_ref() else {
        return ThreadDetailViewModel {
            title: "Thread Activity".to_string(),
            lines: vec!["No thread selected.".to_string()],
        };
    };

    let Some(thread) = state.thread_details.get(thread_id) else {
        return ThreadDetailViewModel {
            title: format!("Thread Activity {thread_id}"),
            lines: vec!["Loading thread details...".to_string()],
        };
    };

    let mut lines = Vec::new();
    if let Some(assignment) = thread_assignment_for_display(state, thread_id) {
        lines.push(format!(
            "assignment {} [{}]",
            assignment.assignment_id,
            codex_assignment_status_label(assignment.status)
        ));
        lines.push(format!(
            "  workstream={}  work_unit={}  supervisor={}",
            assignment.workstream_id, assignment.work_unit_id, assignment.supervisor_id
        ));
        lines.push(format!(
            "  policy={}  bootstrap={}",
            codex_send_policy_label(assignment.send_policy),
            codex_bootstrap_state_label(assignment.bootstrap_state)
        ));
        lines.push(format!(
            "  assigned by {} at {}",
            assignment.assigned_by,
            timestamp_label(assignment.assigned_at)
        ));
        if let Some(turn_id) = assignment.latest_basis_turn_id.as_ref() {
            lines.push(format!("  latest basis turn {turn_id}"));
        }
        if let Some(notes) = assignment.notes.as_ref() {
            lines.push(format!("  notes {}", abbreviate(&compact_line(notes), 84)));
        }
        lines.push(String::new());
    } else {
        lines.push("Assignment: unassigned".to_string());
        lines.push(String::new());
    }

    if thread.turns.is_empty() {
        lines.push("No turns loaded.".to_string());
    } else {
        for turn in thread.turns.iter().rev().take(4) {
            lines.push(format!("turn {} [{}]", turn.id, turn.status));
            if let Some(diff) = turn.latest_diff.as_ref() {
                lines.push(format!("  diff {}", abbreviate(&compact_line(diff), 84)));
            }
            if let Some(turn_state) = turn_state_for_turn(state, thread_id, &turn.id) {
                lines.push(format!(
                    "  lifecycle={} attachable={} live_stream={} terminal={}",
                    lifecycle_label(&turn_state.lifecycle),
                    turn_state.attachable,
                    turn_state.live_stream,
                    turn_state.terminal
                ));
                if let Some(event) = turn_state.recent_event.as_ref() {
                    lines.push(format!("  event {}", abbreviate(&compact_line(event), 84)));
                }
                if let Some(output) = turn_state.recent_output.as_ref() {
                    lines.push(format!(
                        "  output {}",
                        abbreviate(&compact_line(output), 84)
                    ));
                }
            }

            if turn.items.is_empty() {
                lines.push("  no items".to_string());
                continue;
            }

            for item in turn.items.iter().rev().take(3) {
                let status = item.status.clone().unwrap_or_else(|| "unknown".to_string());
                let text = item
                    .text
                    .as_ref()
                    .or(item.summary.as_ref())
                    .map(|text| abbreviate(&compact_line(text), 84))
                    .unwrap_or_else(|| "-".to_string());
                lines.push(format!("  {} [{}] {}", item.item_type, status, text));
            }
        }
    }

    ThreadDetailViewModel {
        title: format!("Thread Activity {}", thread.summary.id),
        lines,
    }
}

pub fn threads_view(state: &AppState) -> ThreadsViewModel {
    ThreadsViewModel {
        list: thread_list(state),
        summary: thread_summary(state),
        detail: thread_detail(state),
    }
}

fn thread_status_label(state: &AppState, thread: &ipc::ThreadSummary) -> String {
    latest_turn_state_for_thread(state, &thread.id)
        .map(|turn| lifecycle_label(&turn.lifecycle).to_string())
        .unwrap_or_else(|| thread.status.clone())
}

fn thread_turn_badge(state: &AppState, thread_id: &str) -> Option<String> {
    latest_turn_state_for_thread(state, thread_id).map(|turn| {
        if turn.attachable && turn.live_stream {
            format!("{} attachable", lifecycle_label(&turn.lifecycle))
        } else {
            lifecycle_label(&turn.lifecycle).to_string()
        }
    })
}

fn thread_assignment_badge(state: &AppState, thread_id: &str) -> Option<String> {
    let assignment = current_thread_assignment(state, thread_id)?;
    Some(codex_assignment_status_label(assignment.status).to_string())
}

fn loaded_status_label(status: ipc::ThreadLoadedStatus) -> &'static str {
    match status {
        ipc::ThreadLoadedStatus::NotLoaded => "not loaded",
        ipc::ThreadLoadedStatus::Idle => "idle",
        ipc::ThreadLoadedStatus::Active => "active",
        ipc::ThreadLoadedStatus::SystemError => "system error",
        ipc::ThreadLoadedStatus::Unknown => "unknown",
    }
}

fn monitor_state_label(state: ipc::ThreadMonitorState) -> &'static str {
    match state {
        ipc::ThreadMonitorState::Detached => "history only",
        ipc::ThreadMonitorState::Attaching => "attaching",
        ipc::ThreadMonitorState::Attached => "live attached",
        ipc::ThreadMonitorState::Errored => "attach errored",
    }
}

fn codex_assignment_status_label(status: CodexThreadAssignmentStatus) -> &'static str {
    match status {
        CodexThreadAssignmentStatus::Proposed => "proposed",
        CodexThreadAssignmentStatus::Active => "assigned",
        CodexThreadAssignmentStatus::Paused => "paused",
        CodexThreadAssignmentStatus::Completed => "completed",
        CodexThreadAssignmentStatus::Released => "released",
    }
}

fn codex_send_policy_label(policy: CodexThreadSendPolicy) -> &'static str {
    match policy {
        CodexThreadSendPolicy::HumanApprovalRequired => "human approval required",
        CodexThreadSendPolicy::SupervisorMaySend => "supervisor may send",
    }
}

fn codex_bootstrap_state_label(state: CodexThreadBootstrapState) -> &'static str {
    match state {
        CodexThreadBootstrapState::NotNeeded => "not needed",
        CodexThreadBootstrapState::Pending => "pending",
        CodexThreadBootstrapState::Proposed => "proposed",
        CodexThreadBootstrapState::Sent => "sent",
    }
}

fn current_thread_assignment<'a>(
    state: &'a AppState,
    thread_id: &str,
) -> Option<&'a ipc::CodexThreadAssignmentSummary> {
    state
        .collaboration
        .codex_thread_assignments
        .iter()
        .find(|assignment| {
            assignment.codex_thread_id == thread_id
                && matches!(
                    assignment.status,
                    CodexThreadAssignmentStatus::Proposed
                        | CodexThreadAssignmentStatus::Active
                        | CodexThreadAssignmentStatus::Paused
                )
        })
}

fn latest_thread_assignment<'a>(
    state: &'a AppState,
    thread_id: &str,
) -> Option<&'a ipc::CodexThreadAssignmentSummary> {
    state
        .collaboration
        .codex_thread_assignments
        .iter()
        .filter(|assignment| assignment.codex_thread_id == thread_id)
        .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
}

fn thread_assignment_for_display<'a>(
    state: &'a AppState,
    thread_id: &str,
) -> Option<&'a ipc::CodexThreadAssignmentSummary> {
    current_thread_assignment(state, thread_id)
        .or_else(|| latest_thread_assignment(state, thread_id))
}

fn latest_turn_state_for_thread<'a>(
    state: &'a AppState,
    thread_id: &str,
) -> Option<&'a ipc::TurnStateView> {
    state
        .turn_states
        .values()
        .filter(|turn| turn.thread_id == thread_id)
        .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
}

fn turn_state_for_turn<'a>(
    state: &'a AppState,
    thread_id: &str,
    turn_id: &str,
) -> Option<&'a ipc::TurnStateView> {
    state
        .turn_states
        .values()
        .find(|turn| turn.thread_id == thread_id && turn.turn_id == turn_id)
}
