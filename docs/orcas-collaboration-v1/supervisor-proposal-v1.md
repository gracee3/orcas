# Orcas Supervisor Proposal v1

This document defines the first supervisor reasoning layer that sits between:

- validated worker report
- human approval
- next bounded assignment dispatch

It is intentionally narrow.

It is not:

- a scheduler
- an autonomous planner
- a persistent supervisor chat thread
- a hidden-memory coordination layer
- a worker-to-worker protocol

The supervisor model is a reasoning engine over explicit Orcas state. Orcas state remains the source of truth.

## Design Summary

The v1 loop is:

```text
Orcas state
  -> SupervisorContextPackBuilder
  -> SupervisorReasoner
  -> validated SupervisorProposal
  -> human approve / reject / edit
  -> explicit Decision
  -> next Assignment
```

Key design choices:

- pure packet mode first
- one proposal targets one primary work unit
- whole-workstream context may inform that proposal
- the model never mutates canonical state directly
- validated proposals are persisted as reviewable artifacts
- only approved decisions and assignments become authoritative Orcas state

## Current Milestone Status

This narrow proposal loop is now implemented as the current Orcas supervisor slice.

Implemented now:

- canonical proposal types and persistence
- daemon-side context-pack building
- deterministic policy and validation
- Responses-backed reasoner
- CLI proposal create, get, list, approve, and reject flows
- proposal lifecycle hardening and observability across snapshot, events, history, and read-only TUI
- opt-in auto-proposal creation on `report_recorded`
- fake-runtime confidence coverage for full completed and interrupted assignment runtime paths

Current guarantees remain unchanged from the design:

- Orcas state is authoritative
- proposals remain review artifacts, not workflow truth
- human approval is required before recording a `Decision` or creating a successor `Assignment`
- auto-proposal remains opt-in, conservative, and fail-closed

Deferred work remains intentionally out of scope for this slice. See [Milestone Closeout](./milestone-closeout.md) for the concise implemented-state and deferred-work summary.

## Scope

V1 proposal generation is only for a work unit that is already at a decision point.

That means:

- the primary work unit must be `awaiting_decision`
- a latest report must already exist for that work unit
- the trigger is either `report_recorded` or an explicit human re-synthesis request for that same decision point

V1 does not support report-free planning or open-ended workstream decomposition through this path.

## Supported Decision Universe In This Slice

The first proposal slice should use the already-implemented decision set:

- `accept`
- `continue`
- `redirect`
- `mark_complete`
- `escalate_to_human`

These are the only decision kinds the v1 reasoner should see.

Future decision kinds such as `retry`, `split`, `merge`, `interrupt`, or `abandon` should stay out of the first supervisor proposal loop. They can be added later without changing the packet-first architecture.

## New Objects

## `SupervisorProposalTrigger`

This identifies why a proposal was requested.

Recommended fields:

- `kind`: `report_recorded` or `human_requested`
- `requested_at`
- `requested_by`
- `source_report_id`
- `note`

`source_report_id` is required in v1. Human-requested synthesis is a re-synthesis of an existing decision point, not a report-free planning action.

## `DecisionPolicy`

This is a deterministic Orcas-produced envelope, not a model output.

Recommended fields:

- `supported_decisions[]`
- `allowed_decisions[]`
- `disallowed_decisions[]`
- `disallowed_reasons_by_decision`
- `assignment_required_for[]`
- `assignment_forbidden_for[]`
- `human_review_required`

This object is used twice:

- before the model call, to bound the reasoner
- after the model call, to validate the proposal

## `SupervisorContextPack`

This is the exact packet sent to the reasoner. It is built from canonical Orcas state.

At runtime it is transient.

For replayability it should be embedded immutably inside the persisted proposal record that was generated from it.

Recommended fields:

- `schema_version`
- `generated_at`
- `trigger`
- `pack_limits`
- `truncation`
- `state_anchor`
- `decision_policy`
- `workstream`
- `primary_work_unit`
- `source_report`
- `current_assignment`
- `worker_session`
- `dependency_context`
- `related_work_units[]`
- `recent_primary_history`
- `relevant_artifacts[]`
- `operator_request`

Field details are defined below in [Context Pack Schema](#context-pack-schema).

## `SupervisorSummary`

This is the human-facing concise proposal summary.

Recommended fields:

- `headline`
- `situation`
- `recommended_action`
- `key_evidence[]`
- `risks[]`
- `review_focus[]`

The summary should be short enough to read before approval without opening raw report text.

## `ProposedDecision`

This is the model's proposed decision, still non-authoritative until approval.

Recommended fields:

- `decision_type`
- `target_work_unit_id`
- `source_report_id`
- `rationale`
- `expected_work_unit_status`
- `requires_assignment`

`decision_type` must be a member of the allowed decision set from `DecisionPolicy`.

## `DraftAssignment`

This exists only when the proposed decision is `continue` or `redirect`.

The important rule is that the model does not author the canonical worker prompt as free text.

Instead it emits a structured draft assignment packet that Orcas later compiles into the existing worker prompt template.

Recommended fields:

- `target_work_unit_id`
- `predecessor_assignment_id`
- `derived_from_decision_type`
- `preferred_worker_id`
- `worker_kind`
- `objective`
- `instructions[]`
- `acceptance_criteria[]`
- `stop_conditions[]`
- `required_context_refs[]`
- `expected_report_fields[]`
- `boundedness_note`

Field details are defined below in [Draft Assignment Contract](#draft-assignment-contract).

## `SupervisorProposal`

This is the structured, validated model output.

Recommended fields:

- `schema_version`
- `summary`
- `proposed_decision`
- `draft_next_assignment`
- `confidence`
- `warnings[]`
- `open_questions[]`

This object is what the human reviews.

## `SupervisorProposalRecord`

This is the persisted review artifact.

Recommended fields:

- `id`
- `workstream_id`
- `primary_work_unit_id`
- `source_report_id`
- `trigger`
- `status`
- `created_at`
- `reasoner_backend`
- `reasoner_model`
- `reasoner_response_id`
- `context_pack`
- `proposal`
- `validated_at`
- `reviewed_at`
- `reviewed_by`
- `review_note`
- `approved_decision_id`
- `approved_assignment_id`

Recommended `status` enum:

- `open`
- `approved`
- `rejected`
- `superseded`
- `stale`

Only `SupervisorProposalRecord` is a first-class persisted object. The rest are persisted only as fields within that record or later materialized into existing authoritative objects such as `Decision` and `Assignment`.

### Proposal Record Status Semantics

`open`

- the proposal is fresh enough to review and no authoritative decision has been applied from it yet

`approved`

- the proposal was approved and Orcas recorded the authoritative `Decision`

`rejected`

- the human explicitly rejected the proposal and no authoritative decision was applied from it

`superseded`

- a newer proposal intentionally replaced this proposal for the same decision point while the same source report was still current

`stale`

- canonical Orcas state changed underneath the proposal before approval, so the proposal no longer matches the current decision point

Recommended stale versus superseded rule:

- mark an older proposal `superseded` when the operator intentionally regenerates for the same work unit and same source report, or when a newer proposal for that same unchanged decision point is approved instead
- mark an older proposal `stale` when the underlying decision point changes, for example because a newer report exists, a decision was already recorded elsewhere, the work unit left `awaiting_decision`, or the current assignment changed

An approval attempt that fails freshness checks should mark the proposal `stale`, not leave it `open`.

## Persisted Versus Transient

Recommended persistence rule:

- persist `SupervisorProposalRecord`
- do not treat it as authoritative workflow state
- do not let it replace `Decision` or `Assignment`

Concrete recommendation:

- `SupervisorContextPack`: transient while building and calling the backend, then embedded into the persisted proposal record
- `SupervisorProposal`: transient while validating, then embedded into the persisted proposal record
- `DecisionPolicy`: transient input to generation and validation, then embedded into the persisted context pack
- raw backend request and response payloads: do not store them in canonical Orcas state; if needed, write them to debug logs or optional per-proposal artifacts

Why persist proposals:

- the human needs a reviewable artifact
- approval may happen later than generation
- replayability requires the exact packet that the model saw
- a rejected or superseded proposal is still useful history

Why not treat proposals as authoritative:

- the model is not the source of truth
- the human may edit before approval
- the work unit may change between generation and approval

## Trigger Recommendation

V1 should support exactly two triggers:

## `report_recorded`

Use when:

- Orcas just recorded a worker report
- the work unit moved to `awaiting_decision`

This does not require a background scheduler. It can be invoked synchronously by the same CLI-first operator flow that just observed the new report.

## `human_requested`

Use when:

- the human wants a fresh synthesis for a work unit already at `awaiting_decision`
- an earlier proposal was rejected or edited
- the operator wants to regenerate after more human context was added to the request note

Do not add background polling, timer-based proposal generation, or autonomous multi-unit sweeps in this slice.

## Proposal Flow

The narrow lifecycle should be:

```text
1. Report is recorded, or human requests synthesis for a work unit already awaiting decision.
2. Orcas runs preflight policy checks.
3. Orcas computes the allowed and disallowed decision set.
4. Orcas builds an immutable SupervisorContextPack from canonical state.
5. Orcas calls SupervisorReasoner with that pack.
6. Orcas parses the model output into SupervisorProposal.
7. Orcas validates the proposal against the same deterministic policy.
8. Orcas persists SupervisorProposalRecord with status=open.
9. Orcas presents summary, proposed decision, and any draft next assignment.
10. Human approves, rejects, or edits before approval.
11. On approval, Orcas re-checks freshness against canonical state.
12. Orcas records an explicit Decision.
13. If the decision requires a next assignment, Orcas creates that Assignment.
14. Orcas marks the proposal record approved and links the resulting decision and assignment ids.
```

Important rule:

- no canonical workflow mutation happens before step 12

## Context Pack Schema

The pack should be inspectable JSON with deterministic ordering and explicit truncation.

Recommended top-level shape:

```json
{
  "schema_version": "supervisor_context_pack.v1",
  "generated_at": "2026-03-16T20:14:00Z",
  "trigger": {},
  "pack_limits": {},
  "truncation": {},
  "state_anchor": {},
  "decision_policy": {},
  "workstream": {},
  "primary_work_unit": {},
  "source_report": {},
  "current_assignment": {},
  "worker_session": {},
  "dependency_context": {},
  "related_work_units": [],
  "recent_primary_history": {},
  "relevant_artifacts": [],
  "operator_request": null
}
```

Recommended field list:

## `schema_version`

- fixed version string for parsing and replay

## `generated_at`

- timestamp of pack creation

## `trigger`

- `kind`
- `source_report_id`
- `requested_by`
- `requested_at`
- `note`

## `pack_limits`

Recommended fields:

- `max_related_work_units`
- `max_prior_reports`
- `max_prior_decisions`
- `max_artifacts`
- `max_raw_report_chars`

These limits should be deterministic and visible to the reviewer.

## `truncation`

Recommended fields:

- `related_work_units_truncated`
- `prior_reports_truncated`
- `prior_decisions_truncated`
- `artifacts_truncated`
- `raw_report_truncated`

If anything was dropped, the pack should say so explicitly.

## `state_anchor`

This is the freshness anchor used again at approval time.

Recommended fields:

- `workstream_id`
- `primary_work_unit_id`
- `source_report_id`
- `source_report_created_at`
- `current_assignment_id`
- `primary_work_unit_updated_at`
- `latest_decision_id`
- `latest_decision_created_at`

Approval must fail closed if these anchors no longer match canonical state.

## `decision_policy`

Embed the exact deterministic policy envelope used for this proposal:

- `supported_decisions[]`
- `allowed_decisions[]`
- `disallowed_decisions[]`
- `disallowed_reasons_by_decision`
- `assignment_required_for[]`
- `assignment_forbidden_for[]`
- `human_review_required`

## `workstream`

Recommended fields:

- `id`
- `title`
- `objective`
- `status`
- `priority`
- `success_criteria[]`
- `constraints[]`
- `summary`
- `open_work_unit_count`
- `blocked_work_unit_count`
- `completed_work_unit_count`

If `success_criteria`, `constraints`, or `summary` are not yet persisted in state, include empty arrays or `null` in the first implementation slice.

## `primary_work_unit`

Recommended fields:

- `id`
- `title`
- `task_statement`
- `status`
- `dependencies[]`
- `current_assignment_id`
- `latest_report_id`
- `acceptance_criteria[]`
- `stop_conditions[]`
- `result_summary`

This should be the fullest work-unit object in the pack.

## `source_report`

Recommended fields:

- `id`
- `assignment_id`
- `worker_id`
- `worker_session_id`
- `submitted_at`
- `disposition`
- `summary`
- `findings[]`
- `blockers[]`
- `questions[]`
- `recommended_next_actions[]`
- `confidence`
- `parse_result`
- `needs_supervisor_review`
- `raw_output_excerpt`

Important rule:

- include the report quality signals even when the report parsed cleanly

## `current_assignment`

Recommended fields:

- `id`
- `status`
- `attempt_number`
- `worker_id`
- `worker_session_id`
- `instructions`
- `created_at`
- `updated_at`

## `worker_session`

Recommended fields:

- `id`
- `worker_id`
- `backend_type`
- `thread_id`
- `active_turn_id`
- `runtime_status`
- `attachability`
- `updated_at`

This lets the reasoner distinguish clean completion from interrupted or lost runtime continuity.

## `dependency_context`

Recommended fields:

- `upstream_dependencies[]`
- `downstream_dependents[]`

Each dependency item should include:

- `work_unit_id`
- `title`
- `status`
- `latest_report_id`
- `latest_decision_id`
- `relation`
- `blocking`

V1 only needs direct edges around the primary work unit, not a whole-workstream graph dump.

## `related_work_units[]`

These are broader workstream items that are useful but not direct dependencies.

Recommended inclusion rules:

- include open sibling units first
- include blocked sibling units next
- include recently completed sibling units only if their latest report or decision is relevant to the primary unit
- sort deterministically by relevance class, then `updated_at`, then `id`

Recommended fields per related unit:

- `id`
- `title`
- `status`
- `latest_report_summary`
- `latest_decision_type`
- `updated_at`

## `recent_primary_history`

Recommended fields:

- `prior_reports[]`
- `prior_decisions[]`

Include only the primary work unit's own recent history, bounded by `pack_limits`.

Each prior report item should include:

- `id`
- `disposition`
- `summary`
- `parse_result`
- `needs_supervisor_review`

Each prior decision item should include:

- `id`
- `decision_type`
- `rationale`
- `created_at`

## `relevant_artifacts[]`

Recommended fields:

- `kind`
- `locator`
- `description`
- `source_object_id`

Only include artifacts already present in Orcas state or explicitly attached to the source report. Do not let the model invent hidden context sources.

## `operator_request`

Optional human note for manual synthesis.

Recommended fields:

- `summary`
- `focus`
- `constraints[]`

This is advisory context, not authoritative state.

## Proposal Output Contract

The reasoner should return schema-constrained structured output, not free text.

Recommended shape:

```json
{
  "schema_version": "supervisor_proposal.v1",
  "summary": {},
  "proposed_decision": {},
  "draft_next_assignment": null,
  "confidence": "medium",
  "warnings": [],
  "open_questions": []
}
```

Recommended fields:

## `summary`

Recommended `SupervisorSummary` fields:

- `headline`
- `situation`
- `recommended_action`
- `key_evidence[]`
- `risks[]`
- `review_focus[]`

Rules:

- `headline` should be one sentence
- `key_evidence` should be short and reference explicit pack facts
- `review_focus` should identify what the human should verify before approval

## `proposed_decision`

Recommended fields:

- `decision_type`
- `target_work_unit_id`
- `source_report_id`
- `rationale`
- `expected_work_unit_status`
- `requires_assignment`

Rules:

- `decision_type` must be allowed by policy
- `target_work_unit_id` must equal the primary work unit id
- `source_report_id` must equal the source report id
- `requires_assignment` must match Orcas policy for that decision type

## `draft_next_assignment`

Required when decision type is `continue` or `redirect`.

Forbidden otherwise.

### Draft Assignment Contract

Recommended fields:

- `target_work_unit_id`
- `predecessor_assignment_id`
- `derived_from_decision_type`
- `preferred_worker_id`
- `worker_kind`
- `objective`
- `instructions[]`
- `acceptance_criteria[]`
- `stop_conditions[]`
- `required_context_refs[]`
- `expected_report_fields[]`
- `boundedness_note`

Recommended bounds:

- `objective`: one sentence
- `instructions[]`: 3 to 7 items
- `acceptance_criteria[]`: 1 to 3 items
- `stop_conditions[]`: 1 to 3 items
- `required_context_refs[]`: ids only, no hidden context
- `expected_report_fields[]`: chosen from the existing report contract

`boundedness_note` should explain why this is a narrow follow-up rather than an open-ended continuation.

## `confidence`

Use the same simple enum as reports:

- `low`
- `medium`
- `high`

## `warnings[]`

Use for:

- ambiguous report parsing
- stale context risk
- unresolved blocker semantics
- missing artifact coverage

## `open_questions[]`

These are questions the human should decide before approving, not instructions for the worker.

## Policy Guardrails

The important rule is that policy lives outside the model.

The model can recommend within bounds.

Orcas decides the bounds, validates the output, and records the authoritative state transition only after approval.

## Pre-Call Guardrails

Orcas should refuse proposal generation if any of these fail:

- the work unit does not exist
- the work unit is not `awaiting_decision`
- no latest report exists for the work unit
- the requested source report is not the latest report for that work unit
- the work unit still has a provably active attachable turn
- another proposal is already `open` for the same work unit and source report, unless the caller explicitly asks to supersede it

Orcas should compute report quality before the model call:

- `clean`: `parse_result=parsed` and `needs_supervisor_review=false`
- `ambiguous`: `parse_result=ambiguous` or `needs_supervisor_review=true`
- `invalid`: `parse_result=invalid`

It should also compute runtime severity:

- `clean_terminal`
- `interrupted`
- `lost_or_unknown`

These classifications feed the allowed decision matrix.

## Post-Call Guardrails

Orcas should reject the model output if any of these are true:

- decision kind is not in the allowed decision set
- the proposal targets any work unit other than the primary work unit
- the proposal references a different or missing source report
- `continue` or `redirect` is returned without a valid `DraftAssignment`
- `accept`, `mark_complete`, or `escalate_to_human` is returned with a draft assignment
- draft assignment lacks objective, stop conditions, or expected report fields
- draft assignment references unknown worker ids or unknown context refs
- draft assignment is not bounded
- the proposal tries to create multiple assignments or multiple target work units

Approval-time freshness checks should fail closed if:

- a newer report exists
- a decision was already recorded for the same work unit after proposal generation
- the work unit status changed out of `awaiting_decision`
- the current assignment id changed

## Allowed Decision Determination

Allowed decisions are not chosen by the model.

Orcas computes them deterministically from:

- current work unit state
- source report quality
- source report disposition
- worker-session runtime continuity classification

### Supported, Allowed, Recommended, Disallowed

- supported decisions: the v1 decision universe Orcas exposes to this loop
- allowed decisions: the subset Orcas permits for this exact decision point
- recommended decision: the model's chosen `proposed_decision` from the allowed set
- disallowed decisions: supported decisions Orcas blocks for this state, with explicit reasons

### Recommended V1 Matrix

If report quality is `clean` and disposition is `completed`:

- allowed: `accept`, `continue`, `redirect`, `mark_complete`, `escalate_to_human`

If report quality is `clean` and disposition is `partial`:

- allowed: `accept`, `continue`, `redirect`, `escalate_to_human`
- disallowed: `mark_complete`
- reason: worker did not claim full completion

If disposition is `blocked`:

- allowed: `continue`, `redirect`, `escalate_to_human`
- disallowed: `accept`, `mark_complete`
- reason: worker stopped on an unresolved blocker

If disposition is `failed`:

- allowed: `continue`, `redirect`, `escalate_to_human`
- disallowed: `accept`, `mark_complete`
- reason: failure cannot be re-labeled as clean completion

If disposition is `interrupted` or runtime severity is `interrupted`:

- allowed: `continue`, `redirect`, `escalate_to_human`
- disallowed: `accept`, `mark_complete`
- reason: interrupted execution is not sufficient evidence of successful completion

If runtime severity is `lost_or_unknown`:

- allowed: `continue`, `redirect`, `escalate_to_human`
- disallowed: `accept`, `mark_complete`
- reason: continuity cannot be proven honestly

If report quality is `ambiguous` or `invalid`:

- allowed: `continue`, `redirect`, `escalate_to_human`
- disallowed: `accept`, `mark_complete`
- reason: ambiguous or invalid parsing forces review instead of completion

This matrix is intentionally conservative.

It is better to force review or bounded follow-up than to let the model promote weak evidence into a completion claim.

## Approval Transition Table

Every approved proposal first closes the current assignment for that work unit decision point.

After that, the approved decision should transition state as follows:

| Approved decision | Resulting work-unit state | Next assignment | Notes |
| --- | --- | --- | --- |
| `accept` | `accepted` | none | Accepts the report outcome as credible and retained, but does not mark the work unit complete and does not unlock downstream dependencies. If more bounded work is needed later, that must happen through a later explicit supervisor action. |
| `continue` | `ready` with `current_assignment_id=<new_assignment_id>` | create one new `created` assignment for the same work unit | Use for one more bounded pass on the same work unit with follow-up instructions. |
| `redirect` | `ready` with `current_assignment_id=<new_assignment_id>` | create one new `created` assignment for the same work unit | Use when the next bounded pass materially changes direction, scope, or worker target. |
| `mark_complete` | `completed` | none | Stronger than `accept`. This is the decision that should satisfy dependency completion. |
| `escalate_to_human` | `needs_human` | none | Human input is required before more worker dispatch on this work unit. |

Important `accept` rule:

- `accept` is not a soft synonym for `mark_complete`
- `accept` should leave the work unit closed for the current assignment but still not complete
- if the supervisor wants another worker pass immediately, it should use `continue` or `redirect`, not `accept`

## Draft Next Assignment Formation

The draft assignment should be formed as a structured follow-up packet, not a monolithic prompt blob.

Recommended rules:

## Bounded Instructions

The assignment must contain:

- one explicit objective
- a small ordered instruction list
- clear acceptance criteria
- explicit stop conditions
- the expected report fields to return

The assignment must not:

- ask for broad replanning
- delegate coordination to the worker
- ask the worker to choose the top-level goal
- reference hidden context outside `required_context_refs`

## Included Context

Only include context that already exists in Orcas state or in the context pack:

- source report id
- relevant prior report ids
- relevant decision ids
- direct dependency work-unit ids
- artifact locators already recorded by Orcas

The assignment should carry references, not a giant transcript.

## Stop Condition

At least one explicit stop condition is required.

Recommended primary stop conditions:

- acceptance criteria satisfied
- blocker requiring supervisor or human decision encountered
- required evidence cannot be obtained honestly

## Expected Worker Report Contract

Every draft assignment should declare the report fields that matter most for the next loop.

Recommended default set:

- `summary`
- `findings`
- `blockers`
- `questions`
- `recommended_next_actions`
- `confidence`

The reasoner can add emphasis such as:

- "must identify exact file path"
- "must answer question X"
- "must include evidence for or against regression Y"

## Relation To Predecessor Assignment And Decision

The draft assignment should explicitly link back to the prior execution segment:

- `predecessor_assignment_id`
- `source_report_id`
- `derived_from_decision_type`

This keeps continuation explicit. Reusing the same worker session does not imply hidden assignment continuity.

## Human Approval In V1

V1 should stay CLI-first and narrow.

Recommended actions:

## Approve

Approve the proposal as-is.

Effect:

- Orcas re-checks freshness
- Orcas records the authoritative `Decision`
- Orcas creates the next `Assignment` if required
- the proposal record becomes `approved`

## Reject

Reject the proposal without state mutation beyond the proposal record.

Effect:

- proposal record becomes `rejected`
- optional rejection reason is stored
- no `Decision` or `Assignment` is created

## Edit Before Approve

Support targeted structured overrides at approval time, not a large new editor UI.

Recommended editable fields:

- `decision_type`
- `decision_rationale`
- `preferred_worker_id`
- `objective`
- `instructions[]`
- `acceptance_criteria[]`
- `stop_conditions[]`
- `expected_report_fields[]`

Rules:

- the edited payload must still pass the same policy validator
- Orcas should persist both the original proposal and the approved edited payload
- the resulting `Decision` and `Assignment` remain the authoritative objects

This is enough for v1 without a write-heavy TUI redesign.

## Backend Abstraction

The backend boundary should be narrow and packet-first.

Recommended trait shape:

```rust
#[async_trait]
pub trait SupervisorReasoner {
    async fn propose(
        &self,
        pack: SupervisorContextPack,
    ) -> OrcasResult<SupervisorReasonerResult>;
}
```

Recommended result shape:

- `proposal: SupervisorProposal`
- `backend_kind`
- `model`
- `response_id`
- `usage`

Important design rules:

- the trait accepts one full explicit packet
- the trait returns one structured proposal
- no persistent thread id is required by the abstraction
- no hidden memory is assumed across calls

## Responses API First Recommendation

Backend B should be the first implementation:

- `ResponsesApiReasoner`

It should:

- send the full `SupervisorContextPack`
- require structured JSON output matching `SupervisorProposal`
- avoid relying on prior conversation state
- surface backend metadata for audit and debugging

A future OpenAI-compatible local TCP endpoint can implement the same trait later.

Do not let transport-specific conversation continuity leak into the core supervisor protocol.

## Suggested Code Placement

Keep the canonical state and policy inside the daemon-centered Orcas layer.

Recommended placement:

## `crates/orcas-core`

Add a new pure data module:

- `src/supervisor.rs`

Suggested contents:

- `SupervisorProposalTrigger`
- `DecisionPolicy`
- `SupervisorContextPack`
- `SupervisorSummary`
- `ProposedDecision`
- `DraftAssignment`
- `SupervisorProposal`
- `SupervisorProposalRecord`

Also extend:

- `src/collaboration.rs`
  - add `supervisor_proposals: BTreeMap<String, SupervisorProposalRecord>` to `CollaborationState`
- `src/ipc.rs`
  - add proposal request and response types

Do not put Responses API transport code here.

## `crates/orcas-daemon`

Add a focused supervisor module tree:

- `src/supervisor/mod.rs`
- `src/supervisor/context_pack.rs`
- `src/supervisor/policy.rs`
- `src/supervisor/proposals.rs`
- `src/supervisor/reasoner/mod.rs`
- `src/supervisor/reasoner/responses_api.rs`

Responsibilities:

- context-pack building from canonical state
- allowed-decision computation
- proposal validation
- proposal persistence
- freshness checks at approval time
- backend dispatch through `SupervisorReasoner`

Existing decision and assignment mutation should stay in the daemon service layer and be reused by proposal approval rather than duplicated.

## `crates/orcas-supervisor`

Extend the CLI only enough for v1:

- `supervisor proposals create`
- `supervisor proposals get`
- `supervisor proposals list-for-workunit`
- `supervisor proposals approve`
- `supervisor proposals reject`

This remains a thin daemon client.

## `crates/orcas-tui`

No write-heavy TUI work in the first slice.

At most, later add read-only proposal visibility alongside reports and decisions.

## Smallest Implementation Slice After This Design

Historical note:

The implementation slice described below is complete. It remains here as design history. For the current implemented state and deferred follow-up work, see [Milestone Closeout](./milestone-closeout.md).

The smallest convincing build slice is:

1. Add pure data types and persistence for `SupervisorProposalRecord`.
2. Add daemon-side `ContextPackBuilder` and `DecisionPolicy` for one work unit in `awaiting_decision` with a latest report.
3. Add `SupervisorReasoner` with one real `ResponsesApiReasoner` backend using structured output.
4. Add post-call validation and persist only validated proposals.
5. Add CLI commands to create, inspect, approve, and reject proposals.
6. On approval, reuse the existing `Decision` recording and next-assignment creation path.

Keep this slice narrow:

- one proposal per work-unit decision point
- only the current five decision kinds
- only CLI approval
- no autonomous background trigger loop
- no TUI write path
- no report-free planning mode

## Recommended First Demo

The first end-to-end demo after implementation should be:

1. Start an assignment.
2. Let Orcas record a report and move the work unit to `awaiting_decision`.
3. Run `supervisor proposals create --workunit <WU>`.
4. Inspect the persisted proposal record and summary.
5. Approve it as-is, or with one structured edit.
6. Confirm Orcas records the authoritative `Decision`.
7. Confirm Orcas creates the next `Assignment` only after approval.

That proves the full packet-driven supervisor loop without introducing scheduler behavior.
