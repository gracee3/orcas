use crate::app::{AppState, ProgramView, ReviewSelection, review_queue_selections};
use orcas_core::{
    ReportConfidence, ReportDisposition, ReportParseResult, SupervisorProposalFailureStage,
    SupervisorProposalStatus, ipc,
};

use super::main::{MainStatusSegmentViewModel, ProgramTabViewModel};
use super::shared::{
    PanelViewModel, abbreviate, compact_line, connection_status, event_log, status_banner,
    timestamp_label,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewViewModel {
    pub header: ReviewHeaderViewModel,
    pub queue: ReviewQueueViewModel,
    pub detail_panel: PanelViewModel,
    pub footer: ReviewFooterViewModel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewHeaderViewModel {
    pub status_segments: Vec<MainStatusSegmentViewModel>,
    pub program_tabs: Vec<ProgramTabViewModel>,
    pub summary_lines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewRowKind {
    Proposal,
    Decision,
    Failure,
    ReviewRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewQueueRowViewModel {
    pub kind: ReviewRowKind,
    pub selection: ReviewSelection,
    pub label: String,
    pub badges: Vec<String>,
    pub secondary: Option<String>,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewQueueViewModel {
    pub rows: Vec<ReviewQueueRowViewModel>,
    pub scroll_offset: usize,
    pub selected_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFooterViewModel {
    pub title: String,
    pub lines: Vec<String>,
    pub hint_line: String,
}

pub fn review_view(state: &AppState) -> ReviewViewModel {
    ReviewViewModel {
        header: review_header(state),
        queue: review_queue(state),
        detail_panel: review_detail_panel(state),
        footer: review_footer(state),
    }
}

pub fn review_queue(state: &AppState) -> ReviewQueueViewModel {
    let rows = review_queue_selections(state)
        .into_iter()
        .map(|selection| review_queue_row(state, selection))
        .collect::<Vec<_>>();
    let selected_index = state
        .review_view
        .selected
        .as_ref()
        .and_then(|selected| rows.iter().position(|row| &row.selection == selected));
    ReviewQueueViewModel {
        rows,
        scroll_offset: state.review_view.scroll_offset,
        selected_index,
    }
}

fn review_header(state: &AppState) -> ReviewHeaderViewModel {
    let connection = connection_status(state);
    let open_proposals = state
        .collaboration
        .work_units
        .iter()
        .filter(|work_unit| {
            work_unit
                .proposal
                .as_ref()
                .is_some_and(|proposal| proposal.latest_status == SupervisorProposalStatus::Open)
        })
        .count();
    let failures = state
        .collaboration
        .work_units
        .iter()
        .filter(|work_unit| {
            work_unit.proposal.as_ref().is_some_and(|proposal| {
                proposal.latest_status == SupervisorProposalStatus::GenerationFailed
            })
        })
        .count();
    let pending_decisions = state
        .collaboration
        .supervisor_turn_decisions
        .iter()
        .filter(|decision| {
            decision.status == orcas_core::SupervisorTurnDecisionStatus::ProposedToHuman
                || decision.open
        })
        .count();
    let review_required = state
        .collaboration
        .reports
        .iter()
        .filter(|report| report.needs_supervisor_review)
        .count();

    let mut summary_lines = vec![format!(
        "queue={} pending_decisions={} open_proposals={} failures={} needs_human={}",
        review_queue_selections(state).len(),
        pending_decisions,
        open_proposals,
        failures,
        review_required
    )];
    if let Some(banner) = status_banner(state) {
        summary_lines.push(format!("update: {}", banner.message));
    } else if let Some(recent) = event_log(state).lines.last() {
        summary_lines.push(format!("update: {recent}"));
    }

    ReviewHeaderViewModel {
        status_segments: vec![
            MainStatusSegmentViewModel {
                label: "daemon".to_string(),
                value: format!("{:?}", connection.daemon_phase).to_ascii_lowercase(),
            },
            MainStatusSegmentViewModel {
                label: "upstream".to_string(),
                value: connection.upstream_status,
            },
            MainStatusSegmentViewModel {
                label: "reconnect".to_string(),
                value: connection.reconnect_attempt.to_string(),
            },
            MainStatusSegmentViewModel {
                label: "threads".to_string(),
                value: connection.known_threads.to_string(),
            },
            MainStatusSegmentViewModel {
                label: "turns".to_string(),
                value: if state.prompt_in_flight {
                    "in_flight".to_string()
                } else {
                    state.session.active_turns.len().to_string()
                },
            },
        ],
        program_tabs: vec![
            ProgramTabViewModel {
                program_view: ProgramView::Main,
                label: "Main".to_string(),
                selected: state.main_view.program_view == ProgramView::Main,
                placeholder: false,
            },
            ProgramTabViewModel {
                program_view: ProgramView::Review,
                label: "Review".to_string(),
                selected: state.main_view.program_view == ProgramView::Review,
                placeholder: false,
            },
        ],
        summary_lines,
    }
}

fn review_queue_row(state: &AppState, selection: ReviewSelection) -> ReviewQueueRowViewModel {
    let selected = state.review_view.selected.as_ref() == Some(&selection);
    match selection.clone() {
        ReviewSelection::Proposal {
            work_unit_id,
            proposal_id,
        } => {
            let work_unit = work_unit_summary(state, &work_unit_id);
            let workstream = work_unit
                .map(|work_unit| workstream_summary(state, &work_unit.workstream_id))
                .flatten();
            let proposal = work_unit.and_then(|work_unit| work_unit.proposal.as_ref());
            ReviewQueueRowViewModel {
                kind: ReviewRowKind::Proposal,
                selection,
                label: work_unit
                    .map(|work_unit| work_unit.title.clone())
                    .unwrap_or_else(|| proposal_id.clone()),
                badges: vec![
                    "proposal".to_string(),
                    proposal
                        .map(|proposal| proposal_status_label(proposal.latest_status).to_string())
                        .unwrap_or_else(|| "open".to_string()),
                    proposal
                        .and_then(|proposal| proposal.open_proposed_decision_type)
                        .map(decision_type_label)
                        .unwrap_or("-")
                        .to_string(),
                ],
                secondary: Some(match (workstream, proposal) {
                    (Some(workstream), Some(proposal)) => format!(
                        "{}  created={}  edits={}",
                        workstream.title,
                        timestamp_label(proposal.latest_created_at),
                        proposal.latest_has_approval_edits
                    ),
                    (Some(workstream), None) => workstream.title.clone(),
                    _ => format!("work_unit={work_unit_id}"),
                }),
                selected,
            }
        }
        ReviewSelection::Decision { decision_id } => {
            let decision = decision_summary(state, &decision_id);
            let work_unit = decision
                .and_then(|decision| decision.work_unit_id.as_deref())
                .and_then(|work_unit_id| work_unit_summary(state, work_unit_id));
            ReviewQueueRowViewModel {
                kind: ReviewRowKind::Decision,
                selection,
                label: work_unit
                    .map(|work_unit| work_unit.title.clone())
                    .unwrap_or_else(|| decision_id.clone()),
                badges: vec![
                    "decision".to_string(),
                    decision
                        .map(|decision| supervisor_decision_status_label(decision).to_string())
                        .unwrap_or_else(|| "pending".to_string()),
                    decision
                        .map(|decision| supervisor_decision_kind_label(decision.kind).to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ],
                secondary: decision
                    .map(|decision| abbreviate(&compact_line(&decision.rationale_summary), 72)),
                selected,
            }
        }
        ReviewSelection::Failure {
            work_unit_id,
            proposal_id,
        } => {
            let work_unit = work_unit_summary(state, &work_unit_id);
            let record = proposal_record(state, &work_unit_id, &proposal_id);
            ReviewQueueRowViewModel {
                kind: ReviewRowKind::Failure,
                selection,
                label: work_unit
                    .map(|work_unit| work_unit.title.clone())
                    .unwrap_or_else(|| proposal_id.clone()),
                badges: vec![
                    "failure".to_string(),
                    record
                        .and_then(|record| record.generation_failure.as_ref())
                        .map(|failure| proposal_failure_stage_label(failure.stage).to_string())
                        .unwrap_or_else(|| "generation_failed".to_string()),
                ],
                secondary: record
                    .and_then(|record| record.generation_failure.as_ref())
                    .map(|failure| abbreviate(&compact_line(&failure.message), 72))
                    .or_else(|| Some(format!("work_unit={work_unit_id}"))),
                selected,
            }
        }
        ReviewSelection::ReviewRequired {
            work_unit_id,
            report_id,
        } => {
            let work_unit = work_unit_summary(state, &work_unit_id);
            let report = report_summary(state, &report_id);
            ReviewQueueRowViewModel {
                kind: ReviewRowKind::ReviewRequired,
                selection,
                label: work_unit
                    .map(|work_unit| work_unit.title.clone())
                    .unwrap_or_else(|| report_id.clone()),
                badges: vec![
                    "review".to_string(),
                    report
                        .map(|report| report_parse_result_label(report.parse_result).to_string())
                        .unwrap_or_else(|| "required".to_string()),
                    report
                        .map(|report| report_confidence_label(report.confidence).to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ],
                secondary: report.map(|report| abbreviate(&compact_line(&report.summary), 72)),
                selected,
            }
        }
    }
}

fn review_detail_panel(state: &AppState) -> PanelViewModel {
    let Some(selection) = state.review_view.selected.as_ref() else {
        return PanelViewModel {
            title: "Review Detail".to_string(),
            lines: vec!["No review item selected.".to_string()],
        };
    };

    match selection {
        ReviewSelection::Proposal {
            work_unit_id,
            proposal_id,
        } => proposal_detail_panel(state, work_unit_id, proposal_id),
        ReviewSelection::Decision { decision_id } => decision_detail_panel(state, decision_id),
        ReviewSelection::Failure {
            work_unit_id,
            proposal_id,
        } => failure_detail_panel(state, work_unit_id, proposal_id),
        ReviewSelection::ReviewRequired {
            work_unit_id,
            report_id,
        } => review_required_detail_panel(state, work_unit_id, report_id),
    }
}

fn proposal_detail_panel(
    state: &AppState,
    work_unit_id: &str,
    proposal_id: &str,
) -> PanelViewModel {
    let work_unit = work_unit_summary(state, work_unit_id);
    let workstream = work_unit
        .map(|work_unit| workstream_summary(state, &work_unit.workstream_id))
        .flatten();
    let Some(record) = proposal_record(state, work_unit_id, proposal_id) else {
        return PanelViewModel {
            title: format!(
                "Proposal {}",
                work_unit
                    .map(|work_unit| work_unit.title.as_str())
                    .unwrap_or(work_unit_id)
            ),
            lines: vec![
                "Loading detailed proposal context...".to_string(),
                format!("work_unit: {work_unit_id}"),
                format!("proposal: {proposal_id}"),
            ],
        };
    };
    let Some(proposal) = record
        .proposal
        .as_ref()
        .or(record.approved_proposal.as_ref())
    else {
        return PanelViewModel {
            title: format!(
                "Proposal {}",
                work_unit
                    .map(|work_unit| work_unit.title.as_str())
                    .unwrap_or(work_unit_id)
            ),
            lines: vec!["Proposal payload is unavailable.".to_string()],
        };
    };

    let mut lines = vec![
        format!("headline: {}", proposal.summary.headline),
        format!(
            "workstream: {}",
            workstream
                .map(|workstream| workstream.title.clone())
                .unwrap_or_else(|| record.workstream_id.clone())
        ),
        format!(
            "work_unit: {}",
            work_unit
                .map(|work_unit| work_unit.title.clone())
                .unwrap_or_else(|| work_unit_id.to_string())
        ),
        format!("status: {}", proposal_status_label(record.status)),
        format!(
            "recommended_action: {}",
            proposal.summary.recommended_action
        ),
        format!(
            "situation: {}",
            abbreviate(&compact_line(&proposal.summary.situation), 120)
        ),
        format!(
            "decision: {}",
            decision_type_label(proposal.proposed_decision.decision_type)
        ),
        format!(
            "rationale: {}",
            abbreviate(&compact_line(&proposal.proposed_decision.rationale), 120)
        ),
        format!(
            "confidence: {}",
            report_confidence_label(proposal.confidence)
        ),
        format!(
            "approval_edits: {}",
            record
                .approval_edits
                .as_ref()
                .map(|edits| if edits.is_empty() { "none" } else { "present" })
                .unwrap_or("none")
        ),
    ];
    if !proposal.summary.key_evidence.is_empty() {
        lines.push("key_evidence:".to_string());
        lines.extend(
            proposal
                .summary
                .key_evidence
                .iter()
                .take(4)
                .map(|item| format!("  - {}", abbreviate(&compact_line(item), 116))),
        );
    }
    if !proposal.summary.risks.is_empty() {
        lines.push("risks:".to_string());
        lines.extend(
            proposal
                .summary
                .risks
                .iter()
                .take(4)
                .map(|item| format!("  - {}", abbreviate(&compact_line(item), 116))),
        );
    }
    if !proposal.open_questions.is_empty() {
        lines.push("open_questions:".to_string());
        lines.extend(
            proposal
                .open_questions
                .iter()
                .take(4)
                .map(|item| format!("  - {}", abbreviate(&compact_line(item), 116))),
        );
    }
    lines.push(format!("created: {}", timestamp_label(record.created_at)));
    if let Some(reviewed_at) = record.reviewed_at {
        lines.push(format!("reviewed: {}", timestamp_label(reviewed_at)));
    }

    PanelViewModel {
        title: format!(
            "Proposal {}",
            work_unit
                .map(|work_unit| work_unit.title.clone())
                .unwrap_or_else(|| work_unit_id.to_string())
        ),
        lines,
    }
}

fn decision_detail_panel(state: &AppState, decision_id: &str) -> PanelViewModel {
    let Some(decision) = decision_summary(state, decision_id) else {
        return PanelViewModel {
            title: format!("Decision {decision_id}"),
            lines: vec!["Selected decision is no longer present.".to_string()],
        };
    };
    let workstream = decision
        .workstream_id
        .as_deref()
        .and_then(|workstream_id| workstream_summary(state, workstream_id));
    let work_unit = decision
        .work_unit_id
        .as_deref()
        .and_then(|work_unit_id| work_unit_summary(state, work_unit_id));
    let thread = thread_summary(state, &decision.codex_thread_id);

    let mut lines = vec![
        format!("kind: {}", supervisor_decision_kind_label(decision.kind)),
        format!("status: {}", supervisor_decision_status_label(decision)),
        format!(
            "proposal_kind: {}",
            supervisor_proposal_kind_label(decision.proposal_kind)
        ),
        format!("thread: {}", decision.codex_thread_id),
        format!(
            "workstream: {}",
            workstream
                .map(|workstream| workstream.title.clone())
                .unwrap_or_else(|| decision
                    .workstream_id
                    .clone()
                    .unwrap_or_else(|| "-".to_string()))
        ),
        format!(
            "work_unit: {}",
            work_unit
                .map(|work_unit| work_unit.title.clone())
                .unwrap_or_else(|| decision
                    .work_unit_id
                    .clone()
                    .unwrap_or_else(|| "-".to_string()))
        ),
        format!(
            "rationale: {}",
            abbreviate(&compact_line(&decision.rationale_summary), 120)
        ),
        format!(
            "proposed_text: {}",
            decision
                .proposed_text
                .as_ref()
                .map(|text| abbreviate(&compact_line(text), 120))
                .unwrap_or_else(|| "-".to_string())
        ),
        format!("open: {}", decision.open),
        format!("created: {}", timestamp_label(decision.created_at)),
    ];
    if let Some(approved_at) = decision.approved_at {
        lines.push(format!("approved: {}", timestamp_label(approved_at)));
    }
    if let Some(rejected_at) = decision.rejected_at {
        lines.push(format!("rejected: {}", timestamp_label(rejected_at)));
    }
    if let Some(sent_at) = decision.sent_at {
        lines.push(format!("sent: {}", timestamp_label(sent_at)));
    }
    if let Some(thread) = thread {
        lines.push(format!("thread_status: {}", thread.status));
        lines.push(format!(
            "thread_monitor: {}",
            thread_monitor_label(thread.monitor_state)
        ));
        lines.push(format!("thread_provider: {}", thread.model_provider));
        if let Some(output) = thread.recent_output.as_ref() {
            lines.push(format!(
                "recent_output: {}",
                abbreviate(&compact_line(output), 120)
            ));
        }
    }

    PanelViewModel {
        title: format!(
            "Decision {}",
            work_unit
                .map(|work_unit| work_unit.title.clone())
                .unwrap_or_else(|| decision.decision_id.clone())
        ),
        lines,
    }
}

fn failure_detail_panel(state: &AppState, work_unit_id: &str, proposal_id: &str) -> PanelViewModel {
    let work_unit = work_unit_summary(state, work_unit_id);
    let workstream = work_unit
        .map(|work_unit| workstream_summary(state, &work_unit.workstream_id))
        .flatten();
    let Some(record) = proposal_record(state, work_unit_id, proposal_id) else {
        return PanelViewModel {
            title: format!(
                "Failure {}",
                work_unit
                    .map(|work_unit| work_unit.title.clone())
                    .unwrap_or_else(|| work_unit_id.to_string())
            ),
            lines: vec![
                "Loading failure context...".to_string(),
                format!("work_unit: {work_unit_id}"),
                format!("proposal: {proposal_id}"),
            ],
        };
    };
    let failure = record.generation_failure.as_ref();
    let mut lines = vec![
        format!(
            "workstream: {}",
            workstream
                .map(|workstream| workstream.title.clone())
                .unwrap_or_else(|| record.workstream_id.clone())
        ),
        format!(
            "work_unit: {}",
            work_unit
                .map(|work_unit| work_unit.title.clone())
                .unwrap_or_else(|| work_unit_id.to_string())
        ),
        format!("status: {}", proposal_status_label(record.status)),
        format!(
            "failure_stage: {}",
            failure
                .map(|failure| proposal_failure_stage_label(failure.stage).to_string())
                .unwrap_or_else(|| "generation_failed".to_string())
        ),
        format!(
            "failure_message: {}",
            failure
                .map(|failure| abbreviate(&compact_line(&failure.message), 120))
                .unwrap_or_else(|| "No failure detail is cached.".to_string())
        ),
        format!("source_report: {}", record.source_report_id),
        format!("created: {}", timestamp_label(record.created_at)),
        "next_action: inspect failure context, then regenerate or adjust supervisor inputs."
            .to_string(),
    ];
    if let Some(output) = record.reasoner_output_text.as_ref() {
        lines.push(format!(
            "reasoner_output: {}",
            abbreviate(&compact_line(output), 120)
        ));
    }

    PanelViewModel {
        title: format!(
            "Failure {}",
            work_unit
                .map(|work_unit| work_unit.title.clone())
                .unwrap_or_else(|| work_unit_id.to_string())
        ),
        lines,
    }
}

fn review_required_detail_panel(
    state: &AppState,
    work_unit_id: &str,
    report_id: &str,
) -> PanelViewModel {
    let work_unit = work_unit_summary(state, work_unit_id);
    let workstream = work_unit
        .map(|work_unit| workstream_summary(state, &work_unit.workstream_id))
        .flatten();
    let report = report_summary(state, report_id);
    let detailed_report = state
        .work_unit_details
        .get(work_unit_id)
        .and_then(|detail| detail.reports.iter().find(|report| report.id == report_id));

    let mut lines = vec![
        format!(
            "workstream: {}",
            workstream
                .map(|workstream| workstream.title.clone())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "work_unit: {}",
            work_unit
                .map(|work_unit| work_unit.title.clone())
                .unwrap_or_else(|| work_unit_id.to_string())
        ),
        format!("report: {report_id}"),
        format!(
            "parse_result: {}",
            report
                .map(|report| report_parse_result_label(report.parse_result).to_string())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "confidence: {}",
            report
                .map(|report| report_confidence_label(report.confidence).to_string())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "reason: {}",
            report
                .map(|report| abbreviate(&compact_line(&report.summary), 120))
                .unwrap_or_else(|| "Loading report context...".to_string())
        ),
    ];
    if let Some(detailed_report) = detailed_report {
        if !detailed_report.findings.is_empty() {
            lines.push("findings:".to_string());
            lines.extend(
                detailed_report
                    .findings
                    .iter()
                    .take(3)
                    .map(|item| format!("  - {}", abbreviate(&compact_line(item), 116))),
            );
        }
        if !detailed_report.questions.is_empty() {
            lines.push("questions:".to_string());
            lines.extend(
                detailed_report
                    .questions
                    .iter()
                    .take(3)
                    .map(|item| format!("  - {}", abbreviate(&compact_line(item), 116))),
            );
        }
        if !detailed_report.recommended_next_actions.is_empty() {
            lines.push("next_actions:".to_string());
            lines.extend(
                detailed_report
                    .recommended_next_actions
                    .iter()
                    .take(3)
                    .map(|item| format!("  - {}", abbreviate(&compact_line(item), 116))),
            );
        }
    } else {
        lines.push("detailed report context is loading on demand.".to_string());
    }
    lines.push(
        "next_action: inspect the report, then confirm whether it needs proposal generation or manual intervention."
            .to_string(),
    );

    PanelViewModel {
        title: format!(
            "Review {}",
            work_unit
                .map(|work_unit| work_unit.title.clone())
                .unwrap_or_else(|| work_unit_id.to_string())
        ),
        lines,
    }
}

fn review_footer(state: &AppState) -> ReviewFooterViewModel {
    let lines = match state.review_view.selected.as_ref() {
        Some(ReviewSelection::Decision { .. }) => vec![
            "Selected item is a supervisor decision awaiting human review.".to_string(),
            "This surface is read-mostly in this pass; approval and rejection remain scaffolded context."
                .to_string(),
        ],
        Some(ReviewSelection::Proposal { .. }) => vec![
            "Selected item is an open supervisor proposal.".to_string(),
            "Approval-edit and approve/reject actions can land here in the next slice.".to_string(),
        ],
        Some(ReviewSelection::Failure { .. }) => vec![
            "Selected item is a proposal generation failure.".to_string(),
            "Use this detail to triage why generation failed before retrying elsewhere.".to_string(),
        ],
        Some(ReviewSelection::ReviewRequired { .. }) => vec![
            "Selected item is a report that still needs supervisor/human review.".to_string(),
            "This area is reserved for future review notes and disposition actions.".to_string(),
        ],
        None => vec!["No review item selected.".to_string()],
    };

    ReviewFooterViewModel {
        title: "Review Actions".to_string(),
        lines,
        hint_line: "up/down move  tab switch tabs  r refresh  ? help".to_string(),
    }
}

fn workstream_summary<'a>(
    state: &'a AppState,
    workstream_id: &str,
) -> Option<&'a ipc::WorkstreamSummary> {
    state
        .collaboration
        .workstreams
        .iter()
        .find(|workstream| workstream.id == workstream_id)
}

fn work_unit_summary<'a>(
    state: &'a AppState,
    work_unit_id: &str,
) -> Option<&'a ipc::WorkUnitSummary> {
    state
        .collaboration
        .work_units
        .iter()
        .find(|work_unit| work_unit.id == work_unit_id)
}

fn report_summary<'a>(state: &'a AppState, report_id: &str) -> Option<&'a ipc::ReportSummary> {
    state
        .collaboration
        .reports
        .iter()
        .find(|report| report.id == report_id)
}

fn decision_summary<'a>(
    state: &'a AppState,
    decision_id: &str,
) -> Option<&'a ipc::SupervisorTurnDecisionSummary> {
    state
        .collaboration
        .supervisor_turn_decisions
        .iter()
        .find(|decision| decision.decision_id == decision_id)
}

fn proposal_record<'a>(
    state: &'a AppState,
    work_unit_id: &str,
    proposal_id: &str,
) -> Option<&'a orcas_core::SupervisorProposalRecord> {
    state
        .work_unit_details
        .get(work_unit_id)
        .and_then(|detail| {
            detail
                .proposals
                .iter()
                .find(|proposal| proposal.id == proposal_id)
        })
}

fn thread_summary<'a>(state: &'a AppState, thread_id: &str) -> Option<&'a ipc::ThreadSummary> {
    state.threads.iter().find(|thread| thread.id == thread_id)
}

fn decision_type_label(decision_type: orcas_core::DecisionType) -> &'static str {
    match decision_type {
        orcas_core::DecisionType::Accept => "accept",
        orcas_core::DecisionType::Continue => "continue",
        orcas_core::DecisionType::Redirect => "redirect",
        orcas_core::DecisionType::MarkComplete => "mark_complete",
        orcas_core::DecisionType::EscalateToHuman => "escalate_to_human",
    }
}

fn proposal_status_label(status: SupervisorProposalStatus) -> &'static str {
    match status {
        SupervisorProposalStatus::Open => "open",
        SupervisorProposalStatus::Approved => "approved",
        SupervisorProposalStatus::Rejected => "rejected",
        SupervisorProposalStatus::Superseded => "superseded",
        SupervisorProposalStatus::Stale => "stale",
        SupervisorProposalStatus::GenerationFailed => "generation_failed",
    }
}

fn proposal_failure_stage_label(stage: SupervisorProposalFailureStage) -> &'static str {
    match stage {
        SupervisorProposalFailureStage::Backend => "backend",
        SupervisorProposalFailureStage::ResponseMalformed => "response_malformed",
        SupervisorProposalFailureStage::ProposalMalformed => "proposal_malformed",
        SupervisorProposalFailureStage::ProposalValidation => "proposal_validation",
    }
}

fn report_parse_result_label(result: ReportParseResult) -> &'static str {
    match result {
        ReportParseResult::Parsed => "parsed",
        ReportParseResult::Ambiguous => "ambiguous",
        ReportParseResult::Invalid => "invalid",
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

#[allow(dead_code)]
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

fn supervisor_decision_kind_label(kind: orcas_core::SupervisorTurnDecisionKind) -> &'static str {
    match kind {
        orcas_core::SupervisorTurnDecisionKind::NextTurn => "next_turn",
        orcas_core::SupervisorTurnDecisionKind::SteerActiveTurn => "steer",
        orcas_core::SupervisorTurnDecisionKind::InterruptActiveTurn => "interrupt",
        orcas_core::SupervisorTurnDecisionKind::NoAction => "no_action",
    }
}

fn supervisor_proposal_kind_label(kind: orcas_core::SupervisorTurnProposalKind) -> &'static str {
    match kind {
        orcas_core::SupervisorTurnProposalKind::Bootstrap => "bootstrap",
        orcas_core::SupervisorTurnProposalKind::ContinueAfterTurn => "continue_after_turn",
        orcas_core::SupervisorTurnProposalKind::ManualRefresh => "manual_refresh",
        orcas_core::SupervisorTurnProposalKind::OperatorSteer => "operator_steer",
        orcas_core::SupervisorTurnProposalKind::OperatorInterrupt => "operator_interrupt",
    }
}

fn supervisor_decision_status_label(decision: &ipc::SupervisorTurnDecisionSummary) -> &'static str {
    match (decision.kind, decision.status) {
        (
            orcas_core::SupervisorTurnDecisionKind::SteerActiveTurn,
            orcas_core::SupervisorTurnDecisionStatus::ProposedToHuman,
        ) => "pending_steer_approval",
        (
            orcas_core::SupervisorTurnDecisionKind::InterruptActiveTurn,
            orcas_core::SupervisorTurnDecisionStatus::ProposedToHuman,
        ) => "pending_interrupt_approval",
        (
            orcas_core::SupervisorTurnDecisionKind::NoAction,
            orcas_core::SupervisorTurnDecisionStatus::Recorded,
        ) => "recorded_no_action",
        (_, orcas_core::SupervisorTurnDecisionStatus::Draft) => "draft",
        (_, orcas_core::SupervisorTurnDecisionStatus::ProposedToHuman) => "pending_human",
        (_, orcas_core::SupervisorTurnDecisionStatus::Approved) => "approved",
        (_, orcas_core::SupervisorTurnDecisionStatus::Rejected) => "rejected",
        (_, orcas_core::SupervisorTurnDecisionStatus::Recorded) => "recorded",
        (_, orcas_core::SupervisorTurnDecisionStatus::Sent) => "sent",
        (_, orcas_core::SupervisorTurnDecisionStatus::Superseded) => "superseded",
        (_, orcas_core::SupervisorTurnDecisionStatus::Stale) => "stale",
    }
}

fn thread_monitor_label(state: ipc::ThreadMonitorState) -> &'static str {
    match state {
        ipc::ThreadMonitorState::Detached => "history_only",
        ipc::ThreadMonitorState::Attaching => "attaching",
        ipc::ThreadMonitorState::Attached => "attached",
        ipc::ThreadMonitorState::Errored => "errored",
    }
}
