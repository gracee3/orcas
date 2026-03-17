# Collaboration Proposal Milestone Closeout

This collaboration and supervisor-proposal workstream is in a good stopping state.

The current Orcas slice is implemented, observable, and covered well enough to pivot to narrower supervisor-design work without carrying forward ambiguous milestone status.

## Implemented Now

- persistent collaboration state for workstreams, work units, assignments, worker sessions, reports, decisions, and supervisor proposals
- daemon snapshot and event surfaces for collaboration state
- read-only TUI collaboration and history inspection
- conservative report parsing with explicit `failed`, `interrupted`, and `lost` handling
- packet-first supervisor proposal loop with canonical proposal types, proposal persistence, context-pack building, deterministic policy, validation, and a Responses-backed reasoner
- CLI proposal commands for `create`, `get`, `list-for-workunit`, `approve`, and `reject`
- proposal lifecycle hardening:
  - original model proposal, human approval edits, and approved proposal are stored separately
  - `generation_failed` proposal records are persisted
  - supersede-open is fail-safe
  - approve and reject remain fail-closed
  - stale, superseded, rejected, and approved proposals remain non-authoritative
- proposal observability through bounded snapshot summary, proposal lifecycle events, work-unit detail/history getters, and read-only TUI proposal visibility
- opt-in automatic proposal creation on `report_recorded` through the real `ingest_assignment_turn_outcome` boundary
- runtime confidence coverage for the real assignment path with a fake Codex runtime on:
  - completed terminal turns
  - interrupted terminal turns

## Current Guarantees

- Orcas state remains the only source of truth.
- Codex app-server remains the worker execution substrate, not the workflow source of truth.
- Supervisor proposals remain review artifacts. They do not change authoritative workflow state by themselves.
- Human approval remains required before Orcas records an authoritative `Decision` or creates a successor `Assignment`.
- Auto-proposal is opt-in through `supervisor.proposals.auto_create_on_report_recorded` and is disabled by default.
- Auto-proposal stays narrow and conservative:
  - only on an eligible `awaiting_decision` work unit
  - only for the current authoritative report decision point
  - strict same-report suppression
  - fail-closed on backend, model, policy, or freshness errors
- Proposal visibility follows the existing Orcas discipline:
  - snapshot carries bounded summary
  - events carry narrow lifecycle updates
  - full proposal detail stays in getter and history paths
  - the TUI is a read-only consumer of daemon truth, not a second proposal state machine
- Current runtime confidence coverage proves the real assignment runtime path through report ingestion and auto-proposal creation for completed and interrupted terminal turns.

## Deferred / Future Work

The following items remain intentionally deferred:

- fake-runtime lost full-path confidence test
- fake-runtime failed terminal-turn full-path confidence test
- fake-runtime failed-start full-path confidence test
- broader automation triggers beyond the current narrow `report_recorded` path
- write-side TUI proposal controls
- scheduler, planner, or swarm behavior
- broader supervisor orchestration beyond the current packet-driven proposal loop
- any persistent supervisor thread or transcript-first supervisor memory model

## Closeout Summary

This milestone establishes a narrow, testable supervisor proposal loop above canonical Orcas state.

What exists now is enough to support:

- bounded worker execution
- explicit report-to-decision review points
- human-reviewed supervisor proposals
- conservative opt-in proposal generation at the report boundary
- bounded operator visibility across CLI, daemon state, and read-only TUI

What does not exist yet is also explicit: broader automation and broader supervisor orchestration remain future work rather than hidden behavior.
