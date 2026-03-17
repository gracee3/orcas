# Codex Thread Supervision v1

This slice adds Orcas-side mirror and monitoring support for Codex app-server threads.
It now also includes Orcas-native assignment metadata for binding a Codex thread to Orcas workflow objects, plus Orcas-native next-turn supervisor decisions with explicit human review.

## Canonical Boundaries

Codex app-server remains canonical for:

- thread existence
- loaded/runtime status
- turn lifecycle
- item/event delivery
- actual turn execution

Orcas remains canonical for:

- persisted mirror state used by Orcas clients
- monitor attachment intent and attachment status shown to operators
- thread-to-workflow assignment metadata
- supervisor proposal state for Codex threads
- human approval workflow for Codex next-turn sends
- stale-basis validation before Orcas sends into a Codex thread

This slice does not store Orcas workflow metadata inside Codex thread history.

## Supported v1 Behavior

- discover existing Codex threads, including externally created and headless threads
- read persisted thread history from app-server and persist the normalized Orcas mirror
- best-effort attach a thread for future live monitoring using documented `thread/resume` behavior
- detach a thread in Orcas so the operator surface reports `history only`
- assign a Codex thread to an Orcas workstream, work unit, and supervisor
- pause, resume, and release Orcas-native Codex-thread assignments
- auto-generate a single human-reviewable next-turn proposal for an active assigned idle thread
- approve and send that proposal through documented `turn/start`
- reject a proposal without sending
- mark a proposal stale when the assignment or Codex thread basis changes before send
- persist thread mirror state and turn state across Orcas restart
- persist Codex-thread assignment state across Orcas restart
- persist Codex-thread supervisor-decision state across Orcas restart
- show in the TUI:
  - loaded status
  - live attach status
  - assignment badge / assignment panel
  - pending human approval / stale / sent / rejected decision state
  - latest next-turn proposal text and rationale
  - persisted turn history
  - aggregated item text
  - turn lifecycle snapshots
  - source kind when app-server exposes it

## Monitor Semantics

Thread monitor state is explicit:

- `detached`: Orcas has history or discovery data only
- `attaching`: Orcas is attempting best-effort live attachment
- `attached`: Orcas requested live monitoring and is ingesting future app-server events
- `errored`: Orcas could not establish best-effort live monitoring

`detached` does not mean the thread is dead. It only means Orcas is not claiming an active live-monitor attachment for that thread.

## Persistence Semantics

Orcas persists:

- normalized thread summaries
- persisted turn/item history already known to Orcas
- turn lifecycle snapshots used by operator views

Orcas does not attempt to reconstruct transient deltas it never observed.

## Assignment Semantics

Codex-thread assignment is an Orcas-native object.

- It binds `codex_thread_id` to `workstream_id`, `work_unit_id`, and `supervisor_id`.
- It is persisted in Orcas collaboration state, not in Codex thread history.
- A thread can still be monitored whether assigned or unassigned.
- Creating an assignment never sends a turn.
- Creating an assignment for an active thread never interrupts, steers, or queues a send.

The daemon enforces:

- at most one active assignment per `codex_thread_id`
- paused assignments are not active
- released assignments are not active
- released assignments remain queryable for audit/history

Current assignment lifecycle operations:

- `create`
- `get`
- `list`
- `pause`
- `resume`
- `release`

## Supervisor Decision Semantics

Supervisor next-turn decisions are Orcas-native objects.

- They are bound to an active `CodexThreadAssignment`.
- They are persisted in Orcas collaboration state, not in Codex thread history.
- Orcas generates them only when the assigned thread is idle.
- Orcas does not silently send them. Human approval is required in this slice.
- Orcas keeps at most one open pending decision per assignment.

Current decision lifecycle operations:

- `list`
- `get`
- `approve_and_send`
- `reject`

Decision status meanings:

- `proposed_to_human`: pending human approval
- `approved`: reserved internal transition during the send path
- `sent`: Orcas successfully called documented `turn/start`
- `rejected`: human rejected the proposal
- `stale`: basis changed before Orcas could send
- `superseded`: defined in the model, but not yet a primary path in this slice

## Basis / Stale Validation

For Codex-thread next-turn proposals, Orcas uses a conservative basis:

- proposals are generated only when the thread is idle
- the basis is the latest known `last_seen_turn_id` at generation time
- `approve_and_send` re-checks that:
  - the assignment is still active
  - the thread is still idle
  - the latest known basis still matches the decision basis
  - the decision is still pending human review

If any of those checks fail, Orcas does not send. The decision is marked stale and remains Orcas-native audit state.

## Bootstrap Proposal Semantics

Assignments persist a `bootstrap_state`.

- new active assignments begin with bootstrap pending
- when the assigned thread is idle and no open decision exists, Orcas proposes a bootstrap next turn first
- when that bootstrap proposal is generated, assignment bootstrap state becomes `proposed`
- if bootstrap is approved and sent, bootstrap state becomes `sent`
- if bootstrap is rejected, bootstrap state becomes `not_needed`
- if bootstrap becomes stale before send, bootstrap state returns to `pending`

Bootstrap text is deterministic and template-based in this slice. Orcas does not yet rely on a separate autonomous reasoning subsystem for Codex-thread next-turn proposals.

## Non-goals In This Slice

- PTY attach or PTY replay
- exact replay of transient deltas Orcas missed before attach
- whole-thread kill semantics
- process-tree kill semantics
- steer proposal/send workflow
- interrupt proposal/send workflow
- automatic supervisor writing into Codex threads
- mutation of app-server’s persisted thread log format
- autonomous sending without human approval

## IPC Surface Added

- `threads/list_loaded`
- `thread/read_history`
- `thread/attach`
- `thread/detach`

Existing `thread/start`, `thread/read`, `thread/get`, `thread/resume`, `turn/start`, and `turn/interrupt` remain in place.

Assignment IPC added:

- `codex_assignment/create`
- `codex_assignment/get`
- `codex_assignment/list`
- `codex_assignment/pause`
- `codex_assignment/resume`
- `codex_assignment/release`

Supervisor decision IPC added:

- `supervisor_decision/list`
- `supervisor_decision/get`
- `supervisor_decision/approve_and_send`
- `supervisor_decision/reject`
