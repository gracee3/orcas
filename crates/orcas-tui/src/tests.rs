use chrono::Utc;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app::{BannerLevel, DaemonConnectionPhase, NavigationFocus, UserAction};
use crate::backend::BackendCommand;
use crate::render;
use crate::test_harness::AppHarness;
use orcas_core::{
    Assignment, AssignmentStatus, ConnectionState, Decision, DecisionType, Report,
    ReportConfidence, ReportDisposition, ReportParseResult, WorkUnit, WorkUnitStatus,
    WorkstreamStatus, ipc,
};

fn sample_thread_summary(id: &str, preview: &str, updated_at: i64) -> ipc::ThreadSummary {
    ipc::ThreadSummary {
        id: id.to_string(),
        preview: preview.to_string(),
        name: None,
        model_provider: "openai".to_string(),
        cwd: "/tmp/orcas".to_string(),
        status: "idle".to_string(),
        created_at: updated_at - 10,
        updated_at,
        scope: "orcas_managed".to_string(),
        recent_output: Some(preview.to_string()),
        recent_event: Some("thread idle".to_string()),
        turn_in_flight: false,
    }
}

fn sample_thread_view(id: &str, preview: &str, output: &str) -> ipc::ThreadView {
    ipc::ThreadView {
        summary: sample_thread_summary(id, preview, 200),
        turns: vec![ipc::TurnView {
            id: "turn-1".to_string(),
            status: "completed".to_string(),
            error_message: None,
            items: vec![ipc::ItemView {
                id: "item-1".to_string(),
                item_type: "agent_message".to_string(),
                status: Some("completed".to_string()),
                text: Some(output.to_string()),
            }],
        }],
    }
}

fn sample_turn_state(
    thread_id: &str,
    turn_id: &str,
    lifecycle: ipc::TurnLifecycleState,
    status: &str,
    attachable: bool,
) -> ipc::TurnStateView {
    ipc::TurnStateView {
        thread_id: thread_id.to_string(),
        turn_id: turn_id.to_string(),
        lifecycle,
        status: status.to_string(),
        attachable,
        live_stream: attachable,
        terminal: !matches!(lifecycle, ipc::TurnLifecycleState::Active),
        recent_output: Some("turn output".to_string()),
        recent_event: Some(format!("turn {status}")),
        updated_at: Utc::now(),
        error_message: None,
    }
}

fn sample_collaboration_snapshot() -> ipc::CollaborationSnapshot {
    ipc::CollaborationSnapshot {
        workstreams: vec![
            ipc::WorkstreamSummary {
                id: "ws-1".to_string(),
                title: "Collaboration hardening".to_string(),
                objective: "Harden collaboration snapshot semantics.".to_string(),
                status: WorkstreamStatus::Active,
                priority: "high".to_string(),
                updated_at: Utc::now(),
            },
            ipc::WorkstreamSummary {
                id: "ws-2".to_string(),
                title: "Deferred work".to_string(),
                objective: "Hold future scope.".to_string(),
                status: WorkstreamStatus::Blocked,
                priority: "low".to_string(),
                updated_at: Utc::now(),
            },
        ],
        work_units: vec![
            ipc::WorkUnitSummary {
                id: "wu-1".to_string(),
                workstream_id: "ws-1".to_string(),
                title: "Snapshot wiring".to_string(),
                status: WorkUnitStatus::AwaitingDecision,
                dependency_count: 0,
                current_assignment_id: Some("assignment-2".to_string()),
                latest_report_id: Some("report-2".to_string()),
                updated_at: Utc::now(),
            },
            ipc::WorkUnitSummary {
                id: "wu-2".to_string(),
                workstream_id: "ws-1".to_string(),
                title: "Event wiring".to_string(),
                status: WorkUnitStatus::Ready,
                dependency_count: 1,
                current_assignment_id: Some("assignment-3".to_string()),
                latest_report_id: Some("report-3".to_string()),
                updated_at: Utc::now(),
            },
            ipc::WorkUnitSummary {
                id: "wu-3".to_string(),
                workstream_id: "ws-2".to_string(),
                title: "Out of scope".to_string(),
                status: WorkUnitStatus::Blocked,
                dependency_count: 2,
                current_assignment_id: None,
                latest_report_id: None,
                updated_at: Utc::now(),
            },
        ],
        assignments: vec![
            ipc::AssignmentSummary {
                id: "assignment-2".to_string(),
                work_unit_id: "wu-1".to_string(),
                worker_id: "worker-a".to_string(),
                worker_session_id: "session-1".to_string(),
                status: AssignmentStatus::AwaitingDecision,
                attempt_number: 2,
                updated_at: Utc::now(),
            },
            ipc::AssignmentSummary {
                id: "assignment-3".to_string(),
                work_unit_id: "wu-2".to_string(),
                worker_id: "worker-a".to_string(),
                worker_session_id: "session-1".to_string(),
                status: AssignmentStatus::Created,
                attempt_number: 3,
                updated_at: Utc::now(),
            },
        ],
        reports: vec![
            ipc::ReportSummary {
                id: "report-2".to_string(),
                work_unit_id: "wu-1".to_string(),
                assignment_id: "assignment-2".to_string(),
                worker_id: "worker-a".to_string(),
                disposition: ReportDisposition::Partial,
                summary: "Snapshot path is implemented, review is required.".to_string(),
                confidence: ReportConfidence::Medium,
                parse_result: ReportParseResult::Ambiguous,
                needs_supervisor_review: true,
                created_at: Utc::now(),
            },
            ipc::ReportSummary {
                id: "report-3".to_string(),
                work_unit_id: "wu-2".to_string(),
                assignment_id: "assignment-3".to_string(),
                worker_id: "worker-a".to_string(),
                disposition: ReportDisposition::Completed,
                summary: "Clean report for event wiring.".to_string(),
                confidence: ReportConfidence::High,
                parse_result: ReportParseResult::Parsed,
                needs_supervisor_review: false,
                created_at: Utc::now(),
            },
        ],
        decisions: vec![ipc::DecisionSummary {
            id: "decision-1".to_string(),
            work_unit_id: "wu-1".to_string(),
            report_id: Some("report-2".to_string()),
            decision_type: DecisionType::Continue,
            rationale: "Need one more bounded pass.".to_string(),
            created_at: Utc::now(),
        }],
    }
}

fn sample_snapshot() -> ipc::StateSnapshot {
    ipc::StateSnapshot {
        daemon: ipc::DaemonStatusResponse {
            socket_path: "/tmp/orcasd.sock".to_string(),
            metadata_path: "/tmp/orcasd.json".to_string(),
            codex_endpoint: "ws://127.0.0.1:4500".to_string(),
            codex_binary_path: "/home/emmy/git/codex/codex-rs/target/debug/codex".to_string(),
            upstream: ConnectionState {
                endpoint: "ws://127.0.0.1:4500".to_string(),
                status: "connected".to_string(),
                detail: None,
            },
            client_count: 1,
            known_threads: 2,
            runtime: ipc::DaemonRuntimeMetadata {
                pid: 4242,
                started_at: Utc::now(),
                version: "0.1.0".to_string(),
                build_fingerprint: "abc123".to_string(),
                binary_path: "/tmp/orcasd".to_string(),
                socket_path: "/tmp/orcasd.sock".to_string(),
                metadata_path: "/tmp/orcasd.json".to_string(),
                git_commit: None,
            },
        },
        session: ipc::SessionState {
            active_thread_id: Some("thread-1".to_string()),
            active_turns: Vec::new(),
        },
        threads: vec![
            sample_thread_summary("thread-1", "hello", 200),
            sample_thread_summary("thread-2", "later", 150),
        ],
        active_thread: Some(sample_thread_view("thread-1", "hello", "world")),
        collaboration: sample_collaboration_snapshot(),
        recent_events: vec![ipc::EventSummary {
            timestamp: Utc::now(),
            kind: "thread".to_string(),
            message: "loaded thread-1".to_string(),
            thread_id: Some("thread-1".to_string()),
            turn_id: None,
        }],
    }
}

fn sample_workunit_detail(work_unit_id: &str) -> ipc::WorkunitGetResponse {
    let now = Utc::now();
    match work_unit_id {
        "wu-1" => ipc::WorkunitGetResponse {
            work_unit: WorkUnit {
                id: "wu-1".to_string(),
                workstream_id: "ws-1".to_string(),
                title: "Snapshot wiring".to_string(),
                task_statement: "Wire collaboration summaries into the snapshot.".to_string(),
                status: WorkUnitStatus::AwaitingDecision,
                dependencies: Vec::new(),
                latest_report_id: Some("report-2".to_string()),
                current_assignment_id: Some("assignment-2".to_string()),
                created_at: now,
                updated_at: now,
            },
            assignments: vec![
                Assignment {
                    id: "assignment-1".to_string(),
                    work_unit_id: "wu-1".to_string(),
                    worker_id: "worker-a".to_string(),
                    worker_session_id: "session-1".to_string(),
                    instructions: "Initial snapshot pass".to_string(),
                    status: AssignmentStatus::Closed,
                    attempt_number: 1,
                    created_at: now,
                    updated_at: now,
                },
                Assignment {
                    id: "assignment-2".to_string(),
                    work_unit_id: "wu-1".to_string(),
                    worker_id: "worker-a".to_string(),
                    worker_session_id: "session-1".to_string(),
                    instructions: "Second bounded pass".to_string(),
                    status: AssignmentStatus::AwaitingDecision,
                    attempt_number: 2,
                    created_at: now,
                    updated_at: now,
                },
            ],
            reports: vec![
                Report {
                    id: "report-1".to_string(),
                    work_unit_id: "wu-1".to_string(),
                    assignment_id: "assignment-1".to_string(),
                    worker_id: "worker-a".to_string(),
                    disposition: ReportDisposition::Completed,
                    summary: "Initial snapshot path landed cleanly.".to_string(),
                    findings: vec!["Snapshot summaries added.".to_string()],
                    blockers: Vec::new(),
                    questions: Vec::new(),
                    recommended_next_actions: vec!["Review event model".to_string()],
                    confidence: ReportConfidence::High,
                    raw_output: "{}".to_string(),
                    parse_result: ReportParseResult::Parsed,
                    needs_supervisor_review: false,
                    created_at: now,
                },
                Report {
                    id: "report-2".to_string(),
                    work_unit_id: "wu-1".to_string(),
                    assignment_id: "assignment-2".to_string(),
                    worker_id: "worker-a".to_string(),
                    disposition: ReportDisposition::Partial,
                    summary: "Snapshot path is implemented, review is required.".to_string(),
                    findings: vec!["Event summaries need one more pass.".to_string()],
                    blockers: Vec::new(),
                    questions: vec!["Should summaries include objective?".to_string()],
                    recommended_next_actions: vec!["Supervisor decide continue.".to_string()],
                    confidence: ReportConfidence::Medium,
                    raw_output: "noise + json".to_string(),
                    parse_result: ReportParseResult::Ambiguous,
                    needs_supervisor_review: true,
                    created_at: now,
                },
            ],
            decisions: vec![Decision {
                id: "decision-1".to_string(),
                work_unit_id: "wu-1".to_string(),
                report_id: Some("report-2".to_string()),
                decision_type: DecisionType::Continue,
                rationale: "Need one more bounded pass.".to_string(),
                created_at: now,
            }],
        },
        "wu-2" => ipc::WorkunitGetResponse {
            work_unit: WorkUnit {
                id: "wu-2".to_string(),
                workstream_id: "ws-1".to_string(),
                title: "Event wiring".to_string(),
                task_statement: "Surface collaboration events in the daemon event stream."
                    .to_string(),
                status: WorkUnitStatus::Ready,
                dependencies: vec!["wu-1".to_string()],
                latest_report_id: Some("report-3".to_string()),
                current_assignment_id: Some("assignment-3".to_string()),
                created_at: now,
                updated_at: now,
            },
            assignments: vec![Assignment {
                id: "assignment-3".to_string(),
                work_unit_id: "wu-2".to_string(),
                worker_id: "worker-a".to_string(),
                worker_session_id: "session-1".to_string(),
                instructions: "Prepare event surface".to_string(),
                status: AssignmentStatus::Created,
                attempt_number: 3,
                created_at: now,
                updated_at: now,
            }],
            reports: vec![Report {
                id: "report-3".to_string(),
                work_unit_id: "wu-2".to_string(),
                assignment_id: "assignment-3".to_string(),
                worker_id: "worker-a".to_string(),
                disposition: ReportDisposition::Completed,
                summary: "Clean report for event wiring.".to_string(),
                findings: Vec::new(),
                blockers: Vec::new(),
                questions: Vec::new(),
                recommended_next_actions: Vec::new(),
                confidence: ReportConfidence::High,
                raw_output: "{}".to_string(),
                parse_result: ReportParseResult::Parsed,
                needs_supervisor_review: false,
                created_at: now,
            }],
            decisions: Vec::new(),
        },
        _ => panic!("unknown sample work unit"),
    }
}

#[tokio::test]
async fn initial_snapshot_load_populates_state() {
    let harness = AppHarness::new(sample_snapshot()).await.unwrap();
    let connection = harness.connection_vm();
    let threads = harness.thread_list_vm();
    let workstreams = harness.workstream_list_vm();
    let work_units = harness.work_unit_list_vm();

    assert_eq!(connection.daemon_phase, DaemonConnectionPhase::Connected);
    assert_eq!(connection.upstream_status, "connected");
    assert_eq!(threads.rows.len(), 2);
    assert!(threads.rows[0].selected);
    assert_eq!(workstreams.rows.len(), 2);
    assert!(workstreams.rows[0].selected);
    assert_eq!(work_units.rows.len(), 2);
}

#[tokio::test]
async fn event_stream_updates_connection_state() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::UpstreamStatusChanged {
                upstream: ConnectionState {
                    endpoint: "ws://127.0.0.1:4500".to_string(),
                    status: "connect_failed".to_string(),
                    detail: Some("boom".to_string()),
                },
            },
        ))
        .await
        .unwrap();

    let connection = harness.connection_vm();
    assert_eq!(connection.upstream_status, "connect_failed");
    assert_eq!(connection.upstream_detail.as_deref(), Some("boom"));
}

#[tokio::test]
async fn active_turn_state_drives_prompt_in_flight_and_thread_badge() {
    let mut snapshot = sample_snapshot();
    snapshot.session.active_turns = vec![ipc::ActiveTurn {
        thread_id: "thread-1".to_string(),
        turn_id: "turn-7".to_string(),
        status: "in_progress".to_string(),
        updated_at: Utc::now(),
    }];

    let harness = AppHarness::new(snapshot).await.unwrap();
    let prompt = harness.prompt_box_vm();
    let threads = harness.thread_list_vm();

    assert!(prompt.in_flight);
    assert_eq!(threads.rows[0].status, "active");
    assert_eq!(
        threads.rows[0].turn_badge.as_deref(),
        Some("active attachable")
    );
}

#[tokio::test]
async fn thread_selection_loads_detail() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness
        .set_thread(sample_thread_view("thread-2", "later", "second output"))
        .await;
    harness
        .set_turn(ipc::TurnAttachResponse {
            turn: Some(sample_turn_state(
                "thread-2",
                "turn-1",
                ipc::TurnLifecycleState::Completed,
                "completed",
                false,
            )),
            attached: false,
            reason: Some("turn already completed; only terminal state is queryable".to_string()),
        })
        .await;
    harness.dispatch(UserAction::SelectNextThread).await;

    let threads = harness.thread_list_vm();
    let detail = harness.thread_detail_vm();
    assert!(threads.rows[1].selected);
    assert!(detail.title.contains("thread-2"));
    assert!(
        detail
            .lines
            .iter()
            .any(|line| line.contains("turn_state: completed"))
    );
    assert!(
        detail
            .lines
            .iter()
            .any(|line| line.contains("second output"))
    );
}

#[tokio::test]
async fn streamed_deltas_accumulate_in_selected_thread() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::TurnUpdated {
                thread_id: "thread-1".to_string(),
                turn: ipc::TurnView {
                    id: "turn-2".to_string(),
                    status: "in_progress".to_string(),
                    error_message: None,
                    items: Vec::new(),
                },
            },
        ))
        .await
        .unwrap();
    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::OutputDelta {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-2".to_string(),
                item_id: "item-2".to_string(),
                delta: "hello ".to_string(),
            },
        ))
        .await
        .unwrap();
    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::OutputDelta {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-2".to_string(),
                item_id: "item-2".to_string(),
                delta: "world".to_string(),
            },
        ))
        .await
        .unwrap();

    let detail = harness.thread_detail_vm();
    assert!(detail.lines.iter().any(|line| line.contains("hello world")));
}

#[tokio::test]
async fn completed_turn_clears_in_progress_marker() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness.dispatch(UserAction::EnterPromptMode).await;
    for ch in "status".chars() {
        harness.dispatch(UserAction::PromptAppend(ch)).await;
    }
    harness.dispatch(UserAction::SubmitPrompt).await;
    assert!(harness.prompt_box_vm().in_flight);

    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::TurnUpdated {
                thread_id: "thread-1".to_string(),
                turn: ipc::TurnView {
                    id: "turn-1".to_string(),
                    status: "completed".to_string(),
                    error_message: None,
                    items: Vec::new(),
                },
            },
        ))
        .await
        .unwrap();

    assert!(!harness.prompt_box_vm().in_flight);
}

#[tokio::test]
async fn prompt_submission_emits_backend_command() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness.dispatch(UserAction::EnterPromptMode).await;
    for ch in "ship it".chars() {
        harness.dispatch(UserAction::PromptAppend(ch)).await;
    }
    harness.dispatch(UserAction::SubmitPrompt).await;

    let commands = harness.recorded_commands().await;
    assert!(commands.contains(&BackendCommand::SubmitPrompt {
        thread_id: "thread-1".to_string(),
        text: "ship it".to_string(),
    }));
}

#[tokio::test]
async fn backend_failure_surfaces_in_banner_state() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness.fail_snapshot_once("cannot load snapshot").await;
    harness.dispatch(UserAction::Refresh).await;

    let banner = harness.state().banner.clone().unwrap();
    assert_eq!(banner.level, BannerLevel::Warning);
    assert!(banner.message.contains("Reconnecting"));
    assert_eq!(
        harness.state().daemon_phase,
        DaemonConnectionPhase::Reconnecting
    );
}

#[tokio::test]
async fn reconnect_recovers_with_snapshot_then_resubscribe() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    let mut recovered = sample_snapshot();
    recovered.threads = vec![sample_thread_summary("thread-2", "recovered", 300)];
    recovered.session.active_thread_id = Some("thread-2".to_string());
    recovered.active_thread = Some(sample_thread_view("thread-2", "recovered", "after restart"));
    recovered.collaboration.workstreams = vec![ipc::WorkstreamSummary {
        id: "ws-9".to_string(),
        title: "Recovered collaboration".to_string(),
        objective: "Reload collaboration snapshot.".to_string(),
        status: WorkstreamStatus::Active,
        priority: "high".to_string(),
        updated_at: Utc::now(),
    }];
    recovered.collaboration.work_units = vec![ipc::WorkUnitSummary {
        id: "wu-9".to_string(),
        workstream_id: "ws-9".to_string(),
        title: "Recovered unit".to_string(),
        status: WorkUnitStatus::Ready,
        dependency_count: 0,
        current_assignment_id: None,
        latest_report_id: None,
        updated_at: Utc::now(),
    }];
    recovered.collaboration.assignments = Vec::new();
    recovered.collaboration.reports = Vec::new();
    recovered.collaboration.decisions = Vec::new();
    harness.replace_snapshot(recovered).await;

    harness.disconnect_events().await;
    harness.process().await;

    assert_eq!(
        harness.state().daemon_phase,
        DaemonConnectionPhase::Reconnecting
    );
    assert_eq!(harness.snapshot_requests().await, 1);
    assert_eq!(harness.subscribe_requests().await, 1);

    harness.force_reconnect_now();
    harness.process().await;

    let connection = harness.connection_vm();
    let detail = harness.thread_detail_vm();
    let workstreams = harness.workstream_list_vm();
    let work_units = harness.work_unit_list_vm();
    assert_eq!(connection.daemon_phase, DaemonConnectionPhase::Connected);
    assert_eq!(harness.snapshot_requests().await, 2);
    assert_eq!(harness.subscribe_requests().await, 2);
    assert_eq!(harness.thread_list_vm().rows.len(), 1);
    assert!(detail.title.contains("thread-2"));
    assert_eq!(workstreams.rows.len(), 1);
    assert_eq!(workstreams.rows[0].title, "Recovered collaboration");
    assert_eq!(work_units.rows.len(), 1);
    assert_eq!(work_units.rows[0].title, "Recovered unit");
    assert!(
        detail
            .lines
            .iter()
            .any(|line| line.contains("after restart"))
    );
}

#[tokio::test]
async fn collaboration_snapshot_drives_rendering() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness
        .set_workunit_detail(sample_workunit_detail("wu-1"))
        .await;
    harness.dispatch(UserAction::Refresh).await;

    let workstream_detail = harness.workstream_detail_vm();
    let work_units = harness.work_unit_list_vm();
    let assignments = harness.assignment_list_vm();
    let detail = harness.collaboration_detail_vm();
    let history = harness.collaboration_history_vm();

    assert!(
        workstream_detail
            .lines
            .iter()
            .any(|line| line.contains("Harden collaboration snapshot semantics."))
    );
    assert!(
        work_units
            .rows
            .iter()
            .any(|row| row.title == "Snapshot wiring" && row.needs_supervisor_review)
    );
    assert!(
        assignments
            .rows
            .iter()
            .any(|row| row.id == "assignment-2" && row.worker_session_id == "session-1")
    );
    assert!(
        detail
            .lines
            .iter()
            .any(|line| line.contains("parse_result: ambiguous"))
    );
    assert!(
        detail
            .lines
            .iter()
            .any(|line| line.contains("decision_rationale: Need one more bounded pass."))
    );
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.contains("assignment-1 [closed]"))
    );
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.contains("report-2 [partial ambiguous review=true]"))
    );
}

#[tokio::test]
async fn collaboration_events_refresh_summaries_incrementally() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::WorkstreamLifecycle {
                action: ipc::CollaborationLifecycleAction::Created,
                workstream: ipc::WorkstreamSummary {
                    id: "ws-3".to_string(),
                    title: "Fresh stream".to_string(),
                    objective: "Add new read-only surface.".to_string(),
                    status: WorkstreamStatus::Active,
                    priority: "medium".to_string(),
                    updated_at: Utc::now(),
                },
            },
        ))
        .await
        .unwrap();
    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::WorkUnitLifecycle {
                action: ipc::CollaborationLifecycleAction::Created,
                work_unit: ipc::WorkUnitSummary {
                    id: "wu-4".to_string(),
                    workstream_id: "ws-3".to_string(),
                    title: "Render panel".to_string(),
                    status: WorkUnitStatus::Running,
                    dependency_count: 0,
                    current_assignment_id: Some("assignment-4".to_string()),
                    latest_report_id: Some("report-4".to_string()),
                    updated_at: Utc::now(),
                },
            },
        ))
        .await
        .unwrap();
    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::AssignmentLifecycle {
                action: ipc::AssignmentLifecycleAction::Started,
                assignment: ipc::AssignmentSummary {
                    id: "assignment-4".to_string(),
                    work_unit_id: "wu-4".to_string(),
                    worker_id: "worker-b".to_string(),
                    worker_session_id: "session-4".to_string(),
                    status: AssignmentStatus::Running,
                    attempt_number: 1,
                    updated_at: Utc::now(),
                },
            },
        ))
        .await
        .unwrap();
    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::ReportRecorded {
                report: ipc::ReportSummary {
                    id: "report-4".to_string(),
                    work_unit_id: "wu-4".to_string(),
                    assignment_id: "assignment-4".to_string(),
                    worker_id: "worker-b".to_string(),
                    disposition: ReportDisposition::Completed,
                    summary: "Panel rendering is visible.".to_string(),
                    confidence: ReportConfidence::High,
                    parse_result: ReportParseResult::Parsed,
                    needs_supervisor_review: false,
                    created_at: Utc::now(),
                },
            },
        ))
        .await
        .unwrap();
    harness
        .inject_event(ipc::DaemonEventEnvelope::new(
            ipc::DaemonEvent::DecisionApplied {
                decision: ipc::DecisionSummary {
                    id: "decision-4".to_string(),
                    work_unit_id: "wu-4".to_string(),
                    report_id: Some("report-4".to_string()),
                    decision_type: DecisionType::MarkComplete,
                    rationale: "Read-only visibility is good enough.".to_string(),
                    created_at: Utc::now(),
                },
            },
        ))
        .await
        .unwrap();

    harness.dispatch(UserAction::CycleFocus).await;
    for _ in 0..3 {
        if harness
            .workstream_detail_vm()
            .title
            .contains("Fresh stream")
        {
            break;
        }
        harness.dispatch(UserAction::SelectPreviousInFocus).await;
    }

    let workstreams = harness.workstream_list_vm();
    let work_units = harness.work_unit_list_vm();
    let assignments = harness.assignment_list_vm();
    let detail = harness.collaboration_detail_vm();

    assert!(
        workstreams
            .rows
            .iter()
            .any(|row| row.title == "Fresh stream")
    );
    assert!(
        harness
            .workstream_detail_vm()
            .title
            .contains("Fresh stream")
    );
    assert!(
        work_units
            .rows
            .iter()
            .any(|row| { row.title == "Render panel" && row.latest_decision == "mark_complete" })
    );
    assert!(
        assignments
            .rows
            .iter()
            .any(|row| row.id == "assignment-4" && row.worker_id == "worker-b")
    );
    assert!(
        detail
            .lines
            .iter()
            .any(|line| line.contains("decision_rationale: Read-only visibility is good enough."))
    );
}

#[tokio::test]
async fn parse_result_and_supervisor_review_display_are_distinct() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness
        .set_workunit_detail(sample_workunit_detail("wu-1"))
        .await;
    harness.dispatch(UserAction::Refresh).await;
    let work_units = harness.work_unit_list_vm();
    let detail = harness.collaboration_detail_vm();
    let history = harness.collaboration_history_vm();

    assert!(work_units.rows.iter().any(|row| {
        row.title == "Snapshot wiring"
            && row.latest_report_parse_result == "ambiguous"
            && row.needs_supervisor_review
    }));
    assert!(work_units.rows.iter().any(|row| {
        row.title == "Event wiring"
            && row.latest_report_parse_result == "parsed"
            && !row.needs_supervisor_review
    }));
    assert!(
        detail
            .lines
            .iter()
            .any(|line| line.contains("parse_result: ambiguous"))
    );
    assert!(
        detail
            .lines
            .iter()
            .any(|line| line.contains("needs_supervisor_review: true"))
    );
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.contains("report-2 [partial ambiguous review=true]"))
    );
}

#[tokio::test]
async fn reused_worker_session_does_not_imply_same_assignment_continuity() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness
        .set_workunit_detail(sample_workunit_detail("wu-1"))
        .await;
    harness.dispatch(UserAction::Refresh).await;
    let assignments = harness.assignment_list_vm();
    let detail = harness.collaboration_detail_vm();
    let history = harness.collaboration_history_vm();

    assert!(
        assignments
            .rows
            .iter()
            .any(|row| { row.id == "assignment-2" && row.worker_session_id == "session-1" })
    );
    assert!(
        assignments
            .rows
            .iter()
            .any(|row| { row.id == "assignment-3" && row.worker_session_id == "session-1" })
    );
    assert!(detail.lines.iter().any(|line| {
        line.contains(
            "assignment: assignment-2 [awaiting_decision] worker=worker-a session=session-1",
        )
    }));
    assert!(history.lines.iter().any(|line| {
        line.contains("assignment-1 [closed] attempt=1 worker=worker-a session=session-1")
    }));
    assert!(history.lines.iter().any(|line| {
        line.contains(
            "assignment-2 [awaiting_decision] attempt=2 worker=worker-a session=session-1",
        )
    }));
}

#[tokio::test]
async fn focus_switches_collaboration_navigation_without_overwriting_thread_state() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness.dispatch(UserAction::CycleFocus).await;
    harness.dispatch(UserAction::SelectNextInFocus).await;
    harness.dispatch(UserAction::CycleFocus).await;

    let status = harness.collaboration_status_vm();
    let detail = harness.workstream_detail_vm();
    let threads = harness.thread_list_vm();

    assert_eq!(status.focus, NavigationFocus::WorkUnits);
    assert!(detail.title.contains("Deferred work"));
    assert!(threads.rows[0].selected);
}

#[tokio::test]
async fn selected_work_unit_history_renders_assignment_report_and_decision_chain() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness
        .set_workunit_detail(sample_workunit_detail("wu-1"))
        .await;
    harness.dispatch(UserAction::Refresh).await;

    let history = harness.collaboration_history_vm();

    assert!(history.title.contains("Snapshot wiring"));
    assert!(history.lines.iter().any(|line| line == "Assignments"));
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.contains("assignment-1 [closed]"))
    );
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.contains("assignment-2 [awaiting_decision]"))
    );
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.contains("report-1 [completed parsed review=false]"))
    );
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.contains("decision-1 [continue]"))
    );
}

#[tokio::test]
async fn reconnect_refreshes_history_for_selected_work_unit() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    let mut recovered = sample_snapshot();
    recovered.collaboration.workstreams = vec![ipc::WorkstreamSummary {
        id: "ws-9".to_string(),
        title: "Recovered collaboration".to_string(),
        objective: "Reload collaboration snapshot.".to_string(),
        status: WorkstreamStatus::Active,
        priority: "high".to_string(),
        updated_at: Utc::now(),
    }];
    recovered.collaboration.work_units = vec![ipc::WorkUnitSummary {
        id: "wu-9".to_string(),
        workstream_id: "ws-9".to_string(),
        title: "Recovered unit".to_string(),
        status: WorkUnitStatus::AwaitingDecision,
        dependency_count: 0,
        current_assignment_id: Some("assignment-9".to_string()),
        latest_report_id: Some("report-9".to_string()),
        updated_at: Utc::now(),
    }];
    recovered.collaboration.assignments = vec![ipc::AssignmentSummary {
        id: "assignment-9".to_string(),
        work_unit_id: "wu-9".to_string(),
        worker_id: "worker-r".to_string(),
        worker_session_id: "session-9".to_string(),
        status: AssignmentStatus::AwaitingDecision,
        attempt_number: 1,
        updated_at: Utc::now(),
    }];
    recovered.collaboration.reports = vec![ipc::ReportSummary {
        id: "report-9".to_string(),
        work_unit_id: "wu-9".to_string(),
        assignment_id: "assignment-9".to_string(),
        worker_id: "worker-r".to_string(),
        disposition: ReportDisposition::Partial,
        summary: "Recovered history summary.".to_string(),
        confidence: ReportConfidence::Medium,
        parse_result: ReportParseResult::Ambiguous,
        needs_supervisor_review: true,
        created_at: Utc::now(),
    }];
    recovered.collaboration.decisions = vec![ipc::DecisionSummary {
        id: "decision-9".to_string(),
        work_unit_id: "wu-9".to_string(),
        report_id: Some("report-9".to_string()),
        decision_type: DecisionType::EscalateToHuman,
        rationale: "Recovered issue needs review.".to_string(),
        created_at: Utc::now(),
    }];
    harness.replace_snapshot(recovered).await;
    harness
        .set_workunit_detail(ipc::WorkunitGetResponse {
            work_unit: WorkUnit {
                id: "wu-9".to_string(),
                workstream_id: "ws-9".to_string(),
                title: "Recovered unit".to_string(),
                task_statement: "Recovered task.".to_string(),
                status: WorkUnitStatus::AwaitingDecision,
                dependencies: Vec::new(),
                latest_report_id: Some("report-9".to_string()),
                current_assignment_id: Some("assignment-9".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            assignments: vec![Assignment {
                id: "assignment-9".to_string(),
                work_unit_id: "wu-9".to_string(),
                worker_id: "worker-r".to_string(),
                worker_session_id: "session-9".to_string(),
                instructions: "Recovered work".to_string(),
                status: AssignmentStatus::AwaitingDecision,
                attempt_number: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }],
            reports: vec![Report {
                id: "report-9".to_string(),
                work_unit_id: "wu-9".to_string(),
                assignment_id: "assignment-9".to_string(),
                worker_id: "worker-r".to_string(),
                disposition: ReportDisposition::Partial,
                summary: "Recovered history summary.".to_string(),
                findings: Vec::new(),
                blockers: vec!["Needs operator review".to_string()],
                questions: Vec::new(),
                recommended_next_actions: Vec::new(),
                confidence: ReportConfidence::Medium,
                raw_output: "raw".to_string(),
                parse_result: ReportParseResult::Ambiguous,
                needs_supervisor_review: true,
                created_at: Utc::now(),
            }],
            decisions: vec![Decision {
                id: "decision-9".to_string(),
                work_unit_id: "wu-9".to_string(),
                report_id: Some("report-9".to_string()),
                decision_type: DecisionType::EscalateToHuman,
                rationale: "Recovered issue needs review.".to_string(),
                created_at: Utc::now(),
            }],
        })
        .await;

    harness.disconnect_events().await;
    harness.process().await;
    harness.force_reconnect_now();
    harness.process().await;

    let history = harness.collaboration_history_vm();
    assert!(history.title.contains("Recovered unit"));
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.contains("report-9 [partial ambiguous review=true]"))
    );
    assert!(
        history
            .lines
            .iter()
            .any(|line| line.contains("decision-9 [escalate_to_human]"))
    );
}

#[tokio::test]
async fn small_terminal_render_keeps_collaboration_surface_stable() {
    let mut harness = AppHarness::new(sample_snapshot()).await.unwrap();
    harness
        .set_workunit_detail(sample_workunit_detail("wu-1"))
        .await;
    harness.dispatch(UserAction::Refresh).await;

    let backend = TestBackend::new(110, 34);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render::render(frame, harness.state()))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let rendered = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Workstreams"));
    assert!(rendered.contains("History"));
    assert!(rendered.contains("Collaboration"));
    assert!(rendered.contains("Snapshot wiring"));
    assert!(rendered.contains("assignment-2"));
}
