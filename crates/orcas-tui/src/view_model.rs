use crate::app::{AppState, BannerLevel, DaemonConnectionPhase, NavigationFocus};
use orcas_core::{
    AssignmentStatus, DecisionType, ReportConfidence, ReportDisposition, ReportParseResult,
    WorkUnitStatus, WorkstreamStatus, ipc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionStatusViewModel {
    pub socket_path: String,
    pub daemon_phase: DaemonConnectionPhase,
    pub upstream_status: String,
    pub upstream_detail: Option<String>,
    pub client_count: usize,
    pub known_threads: usize,
    pub reconnect_attempt: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadRowViewModel {
    pub id: String,
    pub status: String,
    pub turn_badge: Option<String>,
    pub preview: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadListViewModel {
    pub rows: Vec<ThreadRowViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLogViewModel {
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptBoxViewModel {
    pub text: String,
    pub active: bool,
    pub in_flight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusBannerViewModel {
    pub level: BannerLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadDetailViewModel {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollaborationStatusViewModel {
    pub focus: NavigationFocus,
    pub workstream_count: usize,
    pub work_unit_count: usize,
    pub active_assignment_count: usize,
    pub review_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkstreamRowViewModel {
    pub id: String,
    pub title: String,
    pub status: String,
    pub counts: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkstreamListViewModel {
    pub rows: Vec<WorkstreamRowViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkstreamDetailViewModel {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkUnitRowViewModel {
    pub id: String,
    pub title: String,
    pub status: String,
    pub current_assignment: String,
    pub latest_report_parse_result: String,
    pub needs_supervisor_review: bool,
    pub latest_decision: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkUnitListViewModel {
    pub rows: Vec<WorkUnitRowViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentRowViewModel {
    pub id: String,
    pub work_unit_title: String,
    pub worker_id: String,
    pub worker_session_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentListViewModel {
    pub rows: Vec<AssignmentRowViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollaborationDetailViewModel {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollaborationHistoryViewModel {
    pub title: String,
    pub lines: Vec<String>,
}

pub fn connection_status(state: &AppState) -> ConnectionStatusViewModel {
    let daemon = state.daemon.as_ref();
    ConnectionStatusViewModel {
        socket_path: daemon
            .map(|status| status.socket_path.clone())
            .unwrap_or_else(|| "unavailable".to_string()),
        daemon_phase: state.daemon_phase,
        upstream_status: daemon
            .map(|status| status.upstream.status.clone())
            .unwrap_or_else(|| "disconnected".to_string()),
        upstream_detail: daemon.and_then(|status| status.upstream.detail.clone()),
        client_count: daemon.map_or(0, |status| status.client_count),
        known_threads: daemon.map_or(state.threads.len(), |status| status.known_threads),
        reconnect_attempt: state.reconnect_attempt,
    }
}

pub fn collaboration_status(state: &AppState) -> CollaborationStatusViewModel {
    CollaborationStatusViewModel {
        focus: state.navigation_focus,
        workstream_count: state.collaboration.workstreams.len(),
        work_unit_count: state.collaboration.work_units.len(),
        active_assignment_count: state
            .collaboration
            .assignments
            .iter()
            .filter(|assignment| {
                matches!(
                    assignment.status,
                    AssignmentStatus::Created
                        | AssignmentStatus::Running
                        | AssignmentStatus::AwaitingDecision
                )
            })
            .count(),
        review_count: state
            .collaboration
            .reports
            .iter()
            .filter(|report| report.needs_supervisor_review)
            .count(),
    }
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
                preview: abbreviate(&thread.preview.replace('\n', " "), 24),
                selected: state.selected_thread_id.as_deref() == Some(thread.id.as_str()),
            })
            .collect(),
    }
}

pub fn workstream_list(state: &AppState) -> WorkstreamListViewModel {
    WorkstreamListViewModel {
        rows: state
            .collaboration
            .workstreams
            .iter()
            .map(|workstream| {
                let work_units = state
                    .collaboration
                    .work_units
                    .iter()
                    .filter(|work_unit| work_unit.workstream_id == workstream.id)
                    .collect::<Vec<_>>();
                let review_count = work_units
                    .iter()
                    .filter(|work_unit| {
                        latest_report_for_work_unit(state, &work_unit.id)
                            .is_some_and(|report| report.needs_supervisor_review)
                    })
                    .count();
                WorkstreamRowViewModel {
                    id: workstream.id.clone(),
                    title: abbreviate(&workstream.title, 24),
                    status: collaboration_status_label(workstream.status),
                    counts: format!("units={} review={review_count}", work_units.len()),
                    selected: state.selected_workstream_id.as_deref()
                        == Some(workstream.id.as_str()),
                }
            })
            .collect(),
    }
}

pub fn event_log(state: &AppState) -> EventLogViewModel {
    EventLogViewModel {
        lines: state
            .recent_events
            .iter()
            .map(|event| match (&event.thread_id, &event.turn_id) {
                (Some(thread_id), Some(turn_id)) => {
                    format!("[{}] {thread_id}/{turn_id} {}", event.kind, event.message)
                }
                (Some(thread_id), None) => {
                    format!("[{}] {thread_id} {}", event.kind, event.message)
                }
                _ => format!("[{}] {}", event.kind, event.message),
            })
            .collect(),
    }
}

pub fn prompt_box(state: &AppState) -> PromptBoxViewModel {
    PromptBoxViewModel {
        text: state.prompt_input.clone(),
        active: state.prompt_mode,
        in_flight: state.prompt_in_flight,
    }
}

pub fn status_banner(state: &AppState) -> Option<StatusBannerViewModel> {
    state.banner.as_ref().map(|banner| StatusBannerViewModel {
        level: banner.level,
        message: banner.message.clone(),
    })
}

pub fn thread_detail(state: &AppState) -> ThreadDetailViewModel {
    let Some(thread_id) = state.selected_thread_id.as_ref() else {
        return ThreadDetailViewModel {
            title: "Thread".to_string(),
            lines: vec!["No thread selected.".to_string()],
        };
    };

    let Some(thread) = state.thread_details.get(thread_id) else {
        return ThreadDetailViewModel {
            title: format!("Thread {thread_id}"),
            lines: vec!["Loading thread details...".to_string()],
        };
    };

    let mut lines = Vec::new();
    lines.push(format!("status: {}", thread.summary.status));
    lines.push(format!("cwd: {}", thread.summary.cwd));
    if let Some(turn_state) = latest_turn_state_for_thread(state, thread_id) {
        lines.push(format!(
            "turn_state: {}  attachable={}  live_stream={}",
            lifecycle_label(&turn_state.lifecycle),
            turn_state.attachable,
            turn_state.live_stream
        ));
        if let Some(event) = turn_state.recent_event.as_ref() {
            lines.push(format!("turn_event: {event}"));
        }
        if let Some(output) = turn_state.recent_output.as_ref() {
            lines.push(format!("turn_output: {output}"));
        }
    }
    lines.push(format!(
        "preview: {}",
        thread.summary.preview.replace('\n', " ")
    ));
    lines.push(String::new());

    if thread.turns.is_empty() {
        lines.push("No turns loaded.".to_string());
    } else {
        for turn in &thread.turns {
            lines.push(format!("turn {} [{}]", turn.id, turn.status));
            for item in &turn.items {
                let status = item.status.clone().unwrap_or_else(|| "unknown".to_string());
                let text = item.text.clone().unwrap_or_default().replace('\n', "\\n");
                lines.push(format!("  {} {} {}", item.item_type, status, text));
            }
        }
    }

    ThreadDetailViewModel {
        title: format!("Thread {}", thread.summary.id),
        lines,
    }
}

pub fn workstream_detail(state: &AppState) -> WorkstreamDetailViewModel {
    let Some(workstream_id) = state.selected_workstream_id.as_ref() else {
        return WorkstreamDetailViewModel {
            title: "Workstream".to_string(),
            lines: vec!["No workstream selected.".to_string()],
        };
    };
    let Some(workstream) = state
        .collaboration
        .workstreams
        .iter()
        .find(|workstream| workstream.id == *workstream_id)
    else {
        return WorkstreamDetailViewModel {
            title: format!("Workstream {workstream_id}"),
            lines: vec!["Selected workstream is no longer present.".to_string()],
        };
    };

    let work_units = state
        .collaboration
        .work_units
        .iter()
        .filter(|work_unit| work_unit.workstream_id == workstream.id)
        .collect::<Vec<_>>();
    let completed_count = work_units
        .iter()
        .filter(|work_unit| matches!(work_unit.status, WorkUnitStatus::Completed))
        .count();
    let review_count = work_units
        .iter()
        .filter(|work_unit| {
            latest_report_for_work_unit(state, &work_unit.id)
                .is_some_and(|report| report.needs_supervisor_review)
        })
        .count();

    WorkstreamDetailViewModel {
        title: format!("Workstream {}", workstream.title),
        lines: vec![
            format!("id: {}", workstream.id),
            format!("status: {}", collaboration_status_label(workstream.status)),
            format!("priority: {}", workstream.priority),
            format!("objective: {}", compact_line(&workstream.objective)),
            format!(
                "units: total={} completed={} review={}",
                work_units.len(),
                completed_count,
                review_count
            ),
        ],
    }
}

pub fn work_unit_list(state: &AppState) -> WorkUnitListViewModel {
    let Some(workstream_id) = state.selected_workstream_id.as_ref() else {
        return WorkUnitListViewModel { rows: Vec::new() };
    };

    WorkUnitListViewModel {
        rows: state
            .collaboration
            .work_units
            .iter()
            .filter(|work_unit| work_unit.workstream_id == *workstream_id)
            .map(|work_unit| {
                let latest_report = latest_report_for_work_unit(state, &work_unit.id);
                let latest_decision = latest_decision_for_work_unit(state, &work_unit.id);
                WorkUnitRowViewModel {
                    id: work_unit.id.clone(),
                    title: abbreviate(&work_unit.title, 24),
                    status: work_unit_status_label(work_unit.status),
                    current_assignment: work_unit
                        .current_assignment_id
                        .clone()
                        .map(|id| short_id(&id))
                        .unwrap_or_else(|| "-".to_string()),
                    latest_report_parse_result: latest_report
                        .map(|report| report_parse_result_label(report.parse_result).to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    needs_supervisor_review: latest_report
                        .is_some_and(|report| report.needs_supervisor_review),
                    latest_decision: latest_decision
                        .map(|decision| decision_type_label(decision.decision_type).to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    selected: state.selected_work_unit_id.as_deref() == Some(work_unit.id.as_str()),
                }
            })
            .collect(),
    }
}

pub fn assignment_list(state: &AppState) -> AssignmentListViewModel {
    let workstream_work_units = state
        .selected_workstream_id
        .as_ref()
        .map(|workstream_id| {
            state
                .collaboration
                .work_units
                .iter()
                .filter(|work_unit| work_unit.workstream_id == *workstream_id)
                .map(|work_unit| work_unit.id.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    AssignmentListViewModel {
        rows: state
            .collaboration
            .assignments
            .iter()
            .filter(|assignment| {
                matches!(
                    assignment.status,
                    AssignmentStatus::Created
                        | AssignmentStatus::Running
                        | AssignmentStatus::AwaitingDecision
                ) && workstream_work_units.contains(&assignment.work_unit_id.as_str())
            })
            .map(|assignment| AssignmentRowViewModel {
                id: short_id(&assignment.id),
                work_unit_title: state
                    .collaboration
                    .work_units
                    .iter()
                    .find(|work_unit| work_unit.id == assignment.work_unit_id)
                    .map(|work_unit| work_unit.title.clone())
                    .map(|title| abbreviate(&title, 18))
                    .unwrap_or_else(|| short_id(&assignment.work_unit_id)),
                worker_id: abbreviate(&assignment.worker_id, 12),
                worker_session_id: short_id(&assignment.worker_session_id),
                status: assignment_status_label(assignment.status),
            })
            .collect(),
    }
}

pub fn collaboration_detail(state: &AppState) -> CollaborationDetailViewModel {
    let Some(work_unit_id) = state.selected_work_unit_id.as_ref() else {
        return CollaborationDetailViewModel {
            title: "Report / Decision".to_string(),
            lines: vec!["No work unit selected.".to_string()],
        };
    };
    let Some(work_unit) = state
        .collaboration
        .work_units
        .iter()
        .find(|work_unit| work_unit.id == *work_unit_id)
    else {
        return CollaborationDetailViewModel {
            title: "Report / Decision".to_string(),
            lines: vec!["Selected work unit is no longer present.".to_string()],
        };
    };

    let latest_report = latest_report_for_work_unit(state, work_unit_id);
    let latest_decision = latest_decision_for_work_unit(state, work_unit_id);
    let assignment = work_unit
        .current_assignment_id
        .as_ref()
        .and_then(|assignment_id| {
            state
                .collaboration
                .assignments
                .iter()
                .find(|assignment| assignment.id == *assignment_id)
        });

    let mut lines = vec![
        format!("work unit: {}", work_unit.title),
        format!("status: {}", work_unit_status_label(work_unit.status)),
    ];

    if let Some(assignment) = assignment {
        lines.push(format!(
            "assignment: {} [{}] worker={} session={}",
            assignment.id,
            assignment_status_label(assignment.status),
            assignment.worker_id,
            assignment.worker_session_id
        ));
    } else {
        lines.push("assignment: -".to_string());
    }

    if let Some(report) = latest_report {
        lines.push(format!("report: {}", report.id));
        lines.push(format!(
            "report_summary: {}",
            abbreviate(&compact_line(&report.summary), 84)
        ));
        lines.push(format!(
            "parse_result: {}",
            report_parse_result_label(report.parse_result)
        ));
        lines.push(format!(
            "needs_supervisor_review: {}",
            report.needs_supervisor_review
        ));
        lines.push(format!(
            "disposition: {}  confidence: {}",
            report_disposition_label(report.disposition),
            report_confidence_label(report.confidence)
        ));
    } else {
        lines.push("report: -".to_string());
    }

    if let Some(decision) = latest_decision {
        lines.push(format!(
            "decision: {} [{}]",
            decision.id,
            decision_type_label(decision.decision_type)
        ));
        lines.push(format!(
            "decision_rationale: {}",
            abbreviate(&compact_line(&decision.rationale), 84)
        ));
    } else {
        lines.push("decision: -".to_string());
    }

    CollaborationDetailViewModel {
        title: format!("Work Unit {}", work_unit.id),
        lines,
    }
}

pub fn collaboration_history(state: &AppState) -> CollaborationHistoryViewModel {
    let Some(work_unit_id) = state.selected_work_unit_id.as_ref() else {
        return CollaborationHistoryViewModel {
            title: "History".to_string(),
            lines: vec!["No work unit selected.".to_string()],
        };
    };
    let Some(detail) = state.work_unit_details.get(work_unit_id) else {
        return CollaborationHistoryViewModel {
            title: format!("History {}", short_id(work_unit_id)),
            lines: vec!["Loading history...".to_string()],
        };
    };

    let mut lines = Vec::new();
    lines.push("Assignments".to_string());
    if detail.assignments.is_empty() {
        lines.push("  none".to_string());
    } else {
        for assignment in detail.assignments.iter().rev().take(6) {
            let current = if detail.work_unit.current_assignment_id.as_deref()
                == Some(assignment.id.as_str())
            {
                " current"
            } else {
                ""
            };
            lines.push(format!(
                "  {} [{}] attempt={} worker={} session={}{} @ {}",
                short_id(&assignment.id),
                assignment_status_label(assignment.status),
                assignment.attempt_number,
                abbreviate(&assignment.worker_id, 12),
                short_id(&assignment.worker_session_id),
                current,
                timestamp_label(assignment.updated_at)
            ));
        }
    }

    lines.push(String::new());
    lines.push("Reports".to_string());
    if detail.reports.is_empty() {
        lines.push("  none".to_string());
    } else {
        for report in detail.reports.iter().rev().take(6) {
            let review = if report.needs_supervisor_review {
                " review=true"
            } else {
                " review=false"
            };
            lines.push(format!(
                "  {} [{} {}{}] conf={} @ {}",
                short_id(&report.id),
                report_disposition_label(report.disposition),
                report_parse_result_label(report.parse_result),
                review,
                report_confidence_label(report.confidence),
                timestamp_label(report.created_at)
            ));
            lines.push(format!(
                "    {}",
                abbreviate(&compact_line(&report.summary), 88)
            ));
        }
    }

    lines.push(String::new());
    lines.push("Decisions".to_string());
    if detail.decisions.is_empty() {
        lines.push("  none".to_string());
    } else {
        for decision in detail.decisions.iter().rev().take(6) {
            lines.push(format!(
                "  {} [{}] @ {}",
                short_id(&decision.id),
                decision_type_label(decision.decision_type),
                timestamp_label(decision.created_at)
            ));
            lines.push(format!(
                "    {}",
                abbreviate(&compact_line(&decision.rationale), 88)
            ));
        }
    }

    CollaborationHistoryViewModel {
        title: format!("History {}", abbreviate(&detail.work_unit.title, 24)),
        lines,
    }
}

fn thread_status_label(state: &AppState, thread: &orcas_core::ipc::ThreadSummary) -> String {
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

fn latest_turn_state_for_thread<'a>(
    state: &'a AppState,
    thread_id: &str,
) -> Option<&'a orcas_core::ipc::TurnStateView> {
    state
        .turn_states
        .values()
        .filter(|turn| turn.thread_id == thread_id)
        .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
}

fn lifecycle_label(lifecycle: &orcas_core::ipc::TurnLifecycleState) -> &'static str {
    match lifecycle {
        orcas_core::ipc::TurnLifecycleState::Active => "active",
        orcas_core::ipc::TurnLifecycleState::Completed => "completed",
        orcas_core::ipc::TurnLifecycleState::Failed => "failed",
        orcas_core::ipc::TurnLifecycleState::Interrupted => "interrupted",
        orcas_core::ipc::TurnLifecycleState::Lost => "lost",
        orcas_core::ipc::TurnLifecycleState::Unknown => "unknown",
    }
}

fn latest_report_for_work_unit<'a>(
    state: &'a AppState,
    work_unit_id: &str,
) -> Option<&'a ipc::ReportSummary> {
    state
        .collaboration
        .reports
        .iter()
        .filter(|report| report.work_unit_id == work_unit_id)
        .max_by(|left, right| left.created_at.cmp(&right.created_at))
}

fn latest_decision_for_work_unit<'a>(
    state: &'a AppState,
    work_unit_id: &str,
) -> Option<&'a ipc::DecisionSummary> {
    state
        .collaboration
        .decisions
        .iter()
        .filter(|decision| decision.work_unit_id == work_unit_id)
        .max_by(|left, right| left.created_at.cmp(&right.created_at))
}

fn compact_line(text: &str) -> String {
    text.replace('\n', " ")
}

fn abbreviate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    if max_chars <= 1 {
        return "…".to_string();
    }
    let truncated = text
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{truncated}…")
}

fn short_id(id: &str) -> String {
    if id.len() <= 18 {
        id.to_string()
    } else {
        format!("{}…", &id[..18])
    }
}

fn timestamp_label(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    timestamp.format("%H:%M:%S").to_string()
}

fn collaboration_status_label(status: WorkstreamStatus) -> String {
    match status {
        WorkstreamStatus::Active => "active".to_string(),
        WorkstreamStatus::Blocked => "blocked".to_string(),
        WorkstreamStatus::Completed => "completed".to_string(),
    }
}

fn work_unit_status_label(status: WorkUnitStatus) -> String {
    match status {
        WorkUnitStatus::Ready => "ready".to_string(),
        WorkUnitStatus::Blocked => "blocked".to_string(),
        WorkUnitStatus::Running => "running".to_string(),
        WorkUnitStatus::AwaitingDecision => "awaiting_decision".to_string(),
        WorkUnitStatus::Accepted => "accepted".to_string(),
        WorkUnitStatus::NeedsHuman => "needs_human".to_string(),
        WorkUnitStatus::Completed => "completed".to_string(),
    }
}

fn assignment_status_label(status: AssignmentStatus) -> String {
    match status {
        AssignmentStatus::Created => "created".to_string(),
        AssignmentStatus::Running => "running".to_string(),
        AssignmentStatus::AwaitingDecision => "awaiting_decision".to_string(),
        AssignmentStatus::Failed => "failed".to_string(),
        AssignmentStatus::Closed => "closed".to_string(),
        AssignmentStatus::Interrupted => "interrupted".to_string(),
        AssignmentStatus::Lost => "lost".to_string(),
    }
}

fn report_parse_result_label(result: ReportParseResult) -> &'static str {
    match result {
        ReportParseResult::Parsed => "parsed",
        ReportParseResult::Ambiguous => "ambiguous",
        ReportParseResult::Invalid => "invalid",
    }
}

fn report_disposition_label(disposition: ReportDisposition) -> &'static str {
    match disposition {
        ReportDisposition::Completed => "completed",
        ReportDisposition::Partial => "partial",
        ReportDisposition::Blocked => "blocked",
        ReportDisposition::Failed => "failed",
        ReportDisposition::Interrupted => "interrupted",
        ReportDisposition::Unknown => "unknown",
    }
}

fn report_confidence_label(confidence: ReportConfidence) -> &'static str {
    match confidence {
        ReportConfidence::Low => "low",
        ReportConfidence::Medium => "medium",
        ReportConfidence::High => "high",
        ReportConfidence::Unknown => "unknown",
    }
}

fn decision_type_label(decision_type: DecisionType) -> &'static str {
    match decision_type {
        DecisionType::Accept => "accept",
        DecisionType::Continue => "continue",
        DecisionType::Redirect => "redirect",
        DecisionType::MarkComplete => "mark_complete",
        DecisionType::EscalateToHuman => "escalate_to_human",
    }
}
