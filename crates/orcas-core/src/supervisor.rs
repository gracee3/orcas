use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{DecisionType, ReportConfidence, ReportDisposition, ReportParseResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorProposalTriggerKind {
    ReportRecorded,
    HumanRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorProposalTrigger {
    pub kind: SupervisorProposalTriggerKind,
    pub requested_at: DateTime<Utc>,
    pub requested_by: String,
    pub source_report_id: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPolicy {
    pub supported_decisions: Vec<DecisionType>,
    pub allowed_decisions: Vec<DecisionType>,
    pub disallowed_decisions: Vec<DecisionType>,
    #[serde(default)]
    pub disallowed_reasons_by_decision: BTreeMap<String, String>,
    pub assignment_required_for: Vec<DecisionType>,
    pub assignment_forbidden_for: Vec<DecisionType>,
    pub human_review_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorPackLimits {
    pub max_related_work_units: usize,
    pub max_prior_reports: usize,
    pub max_prior_decisions: usize,
    pub max_artifacts: usize,
    pub max_raw_report_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SupervisorPackTruncation {
    pub related_work_units_truncated: bool,
    pub prior_reports_truncated: bool,
    pub prior_decisions_truncated: bool,
    pub artifacts_truncated: bool,
    pub raw_report_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorStateAnchor {
    pub workstream_id: String,
    pub primary_work_unit_id: String,
    pub source_report_id: String,
    pub source_report_created_at: DateTime<Utc>,
    pub current_assignment_id: Option<String>,
    pub primary_work_unit_updated_at: DateTime<Utc>,
    pub latest_decision_id: Option<String>,
    pub latest_decision_created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorWorkstreamContext {
    pub id: String,
    pub title: String,
    pub objective: String,
    pub status: String,
    pub priority: String,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub summary: Option<String>,
    pub open_work_unit_count: usize,
    pub blocked_work_unit_count: usize,
    pub completed_work_unit_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorWorkUnitContext {
    pub id: String,
    pub title: String,
    pub task_statement: String,
    pub status: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub current_assignment_id: Option<String>,
    pub latest_report_id: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub stop_conditions: Vec<String>,
    pub result_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorSourceReportContext {
    pub id: String,
    pub assignment_id: String,
    pub worker_id: String,
    pub worker_session_id: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub disposition: ReportDisposition,
    pub summary: String,
    #[serde(default)]
    pub findings: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub questions: Vec<String>,
    #[serde(default)]
    pub recommended_next_actions: Vec<String>,
    pub confidence: ReportConfidence,
    pub parse_result: ReportParseResult,
    pub needs_supervisor_review: bool,
    pub raw_output_excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorAssignmentContext {
    pub id: String,
    pub status: String,
    pub attempt_number: u32,
    pub worker_id: String,
    pub worker_session_id: String,
    pub instructions: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorWorkerSessionContext {
    pub id: String,
    pub worker_id: String,
    pub backend_type: String,
    pub thread_id: Option<String>,
    pub active_turn_id: Option<String>,
    pub runtime_status: String,
    pub attachability: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorDependencyContextItem {
    pub work_unit_id: String,
    pub title: String,
    pub status: String,
    pub latest_report_id: Option<String>,
    pub latest_decision_id: Option<String>,
    pub relation: String,
    pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SupervisorDependencyContext {
    #[serde(default)]
    pub upstream_dependencies: Vec<SupervisorDependencyContextItem>,
    #[serde(default)]
    pub downstream_dependents: Vec<SupervisorDependencyContextItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedWorkUnitContext {
    pub id: String,
    pub title: String,
    pub status: String,
    pub latest_report_summary: Option<String>,
    pub latest_decision_type: Option<DecisionType>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorReportContext {
    pub id: String,
    pub disposition: ReportDisposition,
    pub summary: String,
    pub parse_result: ReportParseResult,
    pub needs_supervisor_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorDecisionContext {
    pub id: String,
    pub decision_type: DecisionType,
    pub rationale: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentPrimaryHistory {
    #[serde(default)]
    pub prior_reports: Vec<PriorReportContext>,
    #[serde(default)]
    pub prior_decisions: Vec<PriorDecisionContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorArtifactRef {
    pub kind: String,
    pub locator: String,
    pub description: String,
    pub source_object_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorOperatorRequest {
    pub summary: String,
    pub focus: Option<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorContextPack {
    pub schema_version: String,
    pub generated_at: DateTime<Utc>,
    pub trigger: SupervisorProposalTrigger,
    pub pack_limits: SupervisorPackLimits,
    pub truncation: SupervisorPackTruncation,
    pub state_anchor: SupervisorStateAnchor,
    pub decision_policy: DecisionPolicy,
    pub workstream: SupervisorWorkstreamContext,
    pub primary_work_unit: SupervisorWorkUnitContext,
    pub source_report: SupervisorSourceReportContext,
    pub current_assignment: SupervisorAssignmentContext,
    pub worker_session: SupervisorWorkerSessionContext,
    pub dependency_context: SupervisorDependencyContext,
    #[serde(default)]
    pub related_work_units: Vec<RelatedWorkUnitContext>,
    pub recent_primary_history: RecentPrimaryHistory,
    #[serde(default)]
    pub relevant_artifacts: Vec<SupervisorArtifactRef>,
    pub operator_request: Option<SupervisorOperatorRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorSummary {
    pub headline: String,
    pub situation: String,
    pub recommended_action: String,
    #[serde(default)]
    pub key_evidence: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub review_focus: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedDecision {
    pub decision_type: DecisionType,
    pub target_work_unit_id: String,
    pub source_report_id: String,
    pub rationale: String,
    pub expected_work_unit_status: String,
    pub requires_assignment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftAssignment {
    pub target_work_unit_id: String,
    pub predecessor_assignment_id: String,
    pub derived_from_decision_type: DecisionType,
    pub preferred_worker_id: Option<String>,
    pub worker_kind: Option<String>,
    pub objective: String,
    #[serde(default)]
    pub instructions: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub stop_conditions: Vec<String>,
    #[serde(default)]
    pub required_context_refs: Vec<String>,
    #[serde(default)]
    pub expected_report_fields: Vec<String>,
    pub boundedness_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorProposal {
    pub schema_version: String,
    pub summary: SupervisorSummary,
    pub proposed_decision: ProposedDecision,
    pub draft_next_assignment: Option<DraftAssignment>,
    pub confidence: ReportConfidence,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SupervisorReasonerUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorProposalStatus {
    #[default]
    Open,
    Approved,
    Rejected,
    Superseded,
    Stale,
    GenerationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorProposalFailureStage {
    Backend,
    ResponseMalformed,
    ProposalMalformed,
    ProposalValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorProposalFailure {
    pub stage: SupervisorProposalFailureStage,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorProposalRecord {
    pub id: String,
    pub workstream_id: String,
    pub primary_work_unit_id: String,
    pub source_report_id: String,
    pub trigger: SupervisorProposalTrigger,
    #[serde(default)]
    pub status: SupervisorProposalStatus,
    pub created_at: DateTime<Utc>,
    pub reasoner_backend: String,
    pub reasoner_model: String,
    pub reasoner_response_id: Option<String>,
    pub reasoner_usage: Option<SupervisorReasonerUsage>,
    #[serde(default)]
    pub reasoner_output_text: Option<String>,
    pub context_pack: SupervisorContextPack,
    #[serde(default)]
    pub proposal: Option<SupervisorProposal>,
    #[serde(default)]
    pub approval_edits: Option<SupervisorProposalEdits>,
    pub approved_proposal: Option<SupervisorProposal>,
    #[serde(default)]
    pub generation_failure: Option<SupervisorProposalFailure>,
    pub validated_at: Option<DateTime<Utc>>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<String>,
    pub review_note: Option<String>,
    pub approved_decision_id: Option<String>,
    pub approved_assignment_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SupervisorProposalEdits {
    pub decision_type: Option<DecisionType>,
    pub decision_rationale: Option<String>,
    pub preferred_worker_id: Option<String>,
    pub worker_kind: Option<String>,
    pub objective: Option<String>,
    #[serde(default)]
    pub instructions: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub stop_conditions: Vec<String>,
    #[serde(default)]
    pub expected_report_fields: Vec<String>,
}

impl SupervisorProposalEdits {
    pub fn is_empty(&self) -> bool {
        self.decision_type.is_none()
            && self.decision_rationale.is_none()
            && self.preferred_worker_id.is_none()
            && self.worker_kind.is_none()
            && self.objective.is_none()
            && self.instructions.is_empty()
            && self.acceptance_criteria.is_empty()
            && self.stop_conditions.is_empty()
            && self.expected_report_fields.is_empty()
    }
}
