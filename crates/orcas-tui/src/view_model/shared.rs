use crate::app::{
    AppState, BannerLevel, CollaborationFocus, DaemonConnectionPhase, DaemonLifecycleState,
};
use orcas_core::ipc;
use orcas_core::planning::{
    PlanRevisionApplyFailureKind, PlanRevisionApplyPhase, PlanRevisionProposal,
    PlanRevisionProposalStatus,
};
use orcas_core::SupervisorProposalRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelViewModel {
    pub title: String,
    pub lines: Vec<String>,
}

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
pub struct EventLogViewModel {
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusBannerViewModel {
    pub level: BannerLevel,
    pub message: String,
}

pub fn collaboration_focus_label(focus: CollaborationFocus) -> &'static str {
    match focus {
        CollaborationFocus::Workstreams => "workstreams",
        CollaborationFocus::WorkUnits => "work_units",
    }
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

pub fn status_banner(state: &AppState) -> Option<StatusBannerViewModel> {
    state.banner.as_ref().map(|banner| StatusBannerViewModel {
        level: banner.level,
        message: banner.message.clone(),
    })
}

pub(crate) fn daemon_phase_label(phase: DaemonConnectionPhase) -> &'static str {
    match phase {
        DaemonConnectionPhase::Connected => "connected",
        DaemonConnectionPhase::Reconnecting => "reconnecting",
        DaemonConnectionPhase::Disconnected => "disconnected",
    }
}

pub(crate) fn daemon_lifecycle_label(lifecycle: DaemonLifecycleState) -> &'static str {
    match lifecycle {
        DaemonLifecycleState::Unknown => "unknown",
        DaemonLifecycleState::Stopped => "stopped",
        DaemonLifecycleState::Starting => "starting",
        DaemonLifecycleState::Stopping => "stopping",
        DaemonLifecycleState::Restarting => "restarting",
        DaemonLifecycleState::Running => "running",
        DaemonLifecycleState::Failed => "failed",
    }
}

pub(crate) fn lifecycle_label(lifecycle: &ipc::TurnLifecycleState) -> &'static str {
    match lifecycle {
        ipc::TurnLifecycleState::Active => "active",
        ipc::TurnLifecycleState::Completed => "completed",
        ipc::TurnLifecycleState::Failed => "failed",
        ipc::TurnLifecycleState::Interrupted => "interrupted",
        ipc::TurnLifecycleState::Lost => "lost",
        ipc::TurnLifecycleState::Unknown => "unknown",
    }
}

pub(crate) fn compact_line(text: &str) -> String {
    text.replace('\n', " ")
}

pub(crate) fn abbreviate(text: &str, max_chars: usize) -> String {
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

pub(crate) fn short_id(id: &str) -> String {
    if id.len() <= 18 {
        id.to_string()
    } else {
        format!("{}…", &id[..18])
    }
}

pub(crate) fn timestamp_label(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    timestamp.format("%H:%M:%S").to_string()
}

pub(crate) fn proposal_plan_revision(
    proposal: &SupervisorProposalRecord,
) -> Option<&PlanRevisionProposal> {
    proposal
        .approved_proposal
        .as_ref()
        .or(proposal.proposal.as_ref())
        .and_then(|proposal| proposal.plan_revision_proposal.as_ref())
}

pub(crate) fn plan_revision_status_label(status: PlanRevisionProposalStatus) -> &'static str {
    match status {
        PlanRevisionProposalStatus::Pending => "pending",
        PlanRevisionProposalStatus::Approved => "approved",
        PlanRevisionProposalStatus::Applying => "applying",
        PlanRevisionProposalStatus::ApplyFailed => "apply_failed",
        PlanRevisionProposalStatus::Rejected => "rejected",
        PlanRevisionProposalStatus::Applied => "applied",
        PlanRevisionProposalStatus::Superseded => "superseded",
    }
}

pub(crate) fn plan_revision_apply_phase_label(phase: PlanRevisionApplyPhase) -> &'static str {
    match phase {
        PlanRevisionApplyPhase::NotStarted => "not_started",
        PlanRevisionApplyPhase::DownstreamApplying => "downstream_applying",
        PlanRevisionApplyPhase::AwaitingFinalization => "awaiting_finalization",
        PlanRevisionApplyPhase::Applied => "applied",
        PlanRevisionApplyPhase::FailedBeforeDownstream => "failed_before_downstream",
        PlanRevisionApplyPhase::FailedDuringDownstream => "failed_during_downstream",
        PlanRevisionApplyPhase::FailedAfterDownstream => "failed_after_downstream",
        PlanRevisionApplyPhase::Rejected => "rejected",
        PlanRevisionApplyPhase::Superseded => "superseded",
    }
}

pub(crate) fn plan_revision_failure_kind_label(
    failure_kind: PlanRevisionApplyFailureKind,
) -> &'static str {
    match failure_kind {
        PlanRevisionApplyFailureKind::RetryableInfrastructure => "retryable_infrastructure",
        PlanRevisionApplyFailureKind::ValidationFailure => "validation_failure",
        PlanRevisionApplyFailureKind::StaleBasePlan => "stale_base_plan",
        PlanRevisionApplyFailureKind::DownstreamUnknown => "downstream_unknown",
        PlanRevisionApplyFailureKind::FinalizationFailure => "finalization_failure",
        PlanRevisionApplyFailureKind::OperatorRequired => "operator_required",
    }
}

pub(crate) fn plan_revision_next_action_label(
    revision: &PlanRevisionProposal,
) -> &'static str {
    use PlanRevisionProposalStatus::*;

    match revision.status {
        Pending => "operator approval required",
        Approved => "ready to apply",
        Applying => {
            if revision.recovery.downstream_apply_completed {
                "finalize canonical plan version"
            } else if revision.recovery.downstream_apply_started {
                "downstream apply in progress"
            } else {
                "apply in progress"
            }
        }
        ApplyFailed => {
            if revision.recovery.can_reconcile() {
                "reconcile available"
            } else if revision.recovery.can_retry() {
                "retry available"
            } else if revision.recovery.operator_intervention_required {
                "operator review required"
            } else if revision.recovery.downstream_apply_started {
                "unsafe to retry"
            } else {
                "retry blocked"
            }
        }
        Rejected => "rejected; no action",
        Applied => "applied; no action",
        Superseded => "superseded; no action",
    }
}

pub(crate) fn plan_revision_recovery_badge(revision: &PlanRevisionProposal) -> String {
    match revision.status {
        PlanRevisionProposalStatus::ApplyFailed => {
            if revision.recovery.can_reconcile() {
                "reconcile".to_string()
            } else if revision.recovery.can_retry() {
                "retry".to_string()
            } else if revision.recovery.operator_intervention_required {
                "review".to_string()
            } else if revision.recovery.downstream_apply_started {
                "unsafe".to_string()
            } else {
                "blocked".to_string()
            }
        }
        PlanRevisionProposalStatus::Applying => "applying".to_string(),
        PlanRevisionProposalStatus::Applied => "applied".to_string(),
        PlanRevisionProposalStatus::Rejected => "rejected".to_string(),
        PlanRevisionProposalStatus::Superseded => "superseded".to_string(),
        PlanRevisionProposalStatus::Approved => "approved".to_string(),
        PlanRevisionProposalStatus::Pending => "pending".to_string(),
    }
}

pub(crate) fn plan_revision_recovery_lines(revision: &PlanRevisionProposal) -> Vec<String> {
    let mut lines = vec![
        format!(
            "status: {}",
            plan_revision_status_label(revision.status)
        ),
        format!(
            "phase: {}",
            plan_revision_apply_phase_label(revision.recovery.phase)
        ),
        format!(
            "failure_kind: {}",
            revision
                .recovery
                .failure_kind
                .map(plan_revision_failure_kind_label)
                .unwrap_or("-")
        ),
        format!(
            "downstream_started: {}",
            revision.recovery.downstream_apply_started
        ),
        format!(
            "downstream_completed: {}",
            revision.recovery.downstream_apply_completed
        ),
        format!("retry_safe: {}", revision.recovery.can_retry()),
        format!(
            "reconcile_available: {}",
            revision.recovery.can_reconcile()
        ),
        format!(
            "operator_intervention_required: {}",
            revision.recovery.operator_intervention_required
        ),
        format!(
            "next_action: {}",
            plan_revision_next_action_label(revision)
        ),
    ];
    if let Some(error) = revision.apply_error.as_deref() {
        lines.push(format!("apply_error: {}", abbreviate(&compact_line(error), 96)));
    }
    if let Some(error) = revision.recovery.failure_message.as_deref()
        && revision.apply_error.as_deref() != Some(error)
    {
        lines.push(format!(
            "failure_message: {}",
            abbreviate(&compact_line(error), 96)
        ));
    }
    if let Some(plan_id) = revision.applied_plan_id.as_ref() {
        lines.push(format!(
            "applied_plan: {} v{}",
            short_id(plan_id.as_str()),
            revision
                .applied_plan_version
                .map(|version| version.to_string())
                .unwrap_or_else(|| "-".to_string())
        ));
    }
    lines
}
