# Orcas Collaboration

## Overview

Orcas keeps supervision state local, but the current implementation has more than one live state surface. The daemon owns the local IPC contract, the live bridge to the upstream Codex app-server, legacy collaboration state, and the authority store. The CLI is a daemon client. The TUI reads and mutates supervised state through the daemon, while also owning a local PTY-backed `codex resume` helper for interactive attachment to a selected thread.

This document describes the current implemented contract rather than an aspirational target. It focuses on:

- source of truth for each major object class
- the difference between collaboration-owned state and authority-owned state
- merged read models such as `state/get`
- current mutation visibility and event behavior
- restart and reconnect expectations
- current operator surface splits between the CLI, the TUI, and the daemon

The local-authority rationale remains documented in [Local-Authority MVP Backend Design](design/local-authority-mvp-backend.md). The tracked-thread local binding decision remains documented in [ADR 0001](adr/0001-tracked-thread-is-a-local-binding-record.md).

## Collaboration Model

Orcas currently models work across two daemon-owned stores plus one TUI-local session surface.

- Legacy collaboration state lives in daemon memory and is persisted to `state.json`.
- Authority state for authority workstreams, authority work units, and tracked threads lives in SQLite `state.db`.
- `state/get` is a merged derived snapshot that combines daemon state with projected authority summaries.
- `authority/hierarchy/get` is an authority-only hierarchy query over the SQLite store.
- The TUI also keeps local PTY-backed `codex resume` session state that is not daemon-owned.

Tracked threads are Orcas-owned local binding records, not upstream Codex thread rows. A tracked thread may reference an `upstream_thread_id`, but create, edit, and delete operations act on the local Orcas record rather than claiming ownership of upstream runtime storage.

The important rule is that ORCAS does not currently present a single uniform workflow backend. Some operator-visible objects share a name while coming from different owners and read models.

## Source-Of-Truth Matrix

### Ownership And Read Paths

| Object or state class | Authoritative owner | Durable persistence owner | Canonical mutation path | Canonical read path(s) | In `state/get` | In `authority/hierarchy/get` |
| --- | --- | --- | --- | --- | --- | --- |
| Workstream, legacy collaboration record | Daemon collaboration state | `state.json` | `workstream/create` and internal daemon updates | `state/get`, `workstream/list`, `workstream/get` | Yes | No |
| Workstream, authority record | Authority SQLite store | `state.db` | `authority/workstream/create`, `authority/workstream/edit`, `authority/workstream/delete` | `authority/hierarchy/get`, `authority/workstream/get`, projected into `state/get` | Yes, as a projected collaboration-shaped summary | Yes |
| Work unit, legacy collaboration record | Daemon collaboration state | `state.json` | `workunit/create` and internal daemon updates | `state/get`, `workunit/list`, `workunit/get` | Yes | No |
| Work unit, authority record | Authority SQLite store | `state.db` | `authority/workunit/create`, `authority/workunit/edit`, `authority/workunit/delete` | `authority/hierarchy/get`, `authority/workunit/get`, projected into `state/get`, and injected into collaboration state for assignment start compatibility | Yes, as a projected or bridged collaboration-shaped summary | Yes |
| Tracked thread, authority record | Authority SQLite store | `state.db` | `authority/tracked_thread/create`, `authority/tracked_thread/edit`, `authority/tracked_thread/delete` | `authority/hierarchy/get`, `authority/workunit/get`, `authority/tracked_thread/get` | No tracked-thread rows appear directly in `state/get` | Yes |
| Assignment | Daemon collaboration state | `state.json` | `assignment/start` plus daemon-owned lifecycle transitions | `state/get`, `assignment/get` | Yes | No |
| Proposal | Daemon collaboration state | `state.json` | `proposal/create`, `proposal/approve`, `proposal/reject`, plus daemon-owned generation and supersession | `proposal/get`, `proposal/list_for_workunit`, event stream, and nested proposal summary inside collaboration work unit summaries | No top-level proposal list in `state/get`; proposal summary can appear inside collaboration work unit summaries | No |
| Decision | Daemon collaboration state | `state.json` | `decision/apply` | `state/get`, `decision/apply` response | Yes | No |
| Report | Daemon collaboration state | `state.json` | Internal daemon recording during assignment and report handling | `state/get`, `report/get`, `report/list_for_workunit` | Yes | No |
| Worker session | Daemon collaboration state | `state.json` | Internal daemon-only selection and lifecycle updates | No dedicated public query; visible indirectly through assignment behavior and persisted collaboration state | No | No |
| Live thread state | Daemon live state mirrored from Codex | Thread mirror data in `state.json` | `thread/start`, `thread/resume`, daemon Codex event bridge, internal mirror maintenance | `state/get`, `threads/list*`, `thread/read*`, `thread/get` | Yes | No |
| Live turn state | Daemon live state mirrored from Codex | Turn mirror data in `state.json` | `turn/start`, `turn/steer`, `turn/interrupt`, daemon Codex event bridge, internal mirror maintenance | `state/get` active thread view, `turns/list_active`, `turns/recent`, `turn/get`, `turn/attach` | Yes, through session and active thread data | No |
| Local PTY-backed Codex resume session state | TUI-local `CodexSessionManager` | None | TUI `ResumeSelectedThreadInCodex` action and local PTY lifecycle | TUI-local state only | No | No |

### Projection, Visibility, And Restart Behavior

| Object or state class | Projected, derived, or synthesized notes | Current event visibility | Restart and reconnect behavior |
| --- | --- | --- | --- |
| Workstream, legacy collaboration record | Collaboration-shaped native record | `WorkstreamLifecycle` for collaboration updates | Survives daemon restart through `state.json`; clients reload via snapshot-first flow |
| Workstream, authority record | Projected into `state/get` as `ipc::WorkstreamSummary`; no authority revision, tombstone, or origin metadata in the projected row. A workstream can also be copied into legacy collaboration state when a dependent authority work unit is bridged for assignment compatibility. | `authority/workstream/create` and `authority/workstream/edit` currently emit `WorkstreamLifecycle`; delete emits no daemon event | Survives daemon restart through `state.db`; projected rows reload from authority state, and any previously bridged collaboration copy also survives through `state.json` |
| Work unit, legacy collaboration record | Collaboration-shaped native record | `WorkUnitLifecycle` for collaboration updates | Survives daemon restart through `state.json`; clients reload via snapshot-first flow |
| Work unit, authority record | Appears in `state/get` either as a projected authority summary or, after assignment compatibility bridging, as an injected collaboration record | `authority/workunit/create` and `authority/workunit/edit` currently emit `WorkUnitLifecycle`; delete emits no daemon event | Survives daemon restart through `state.db`; assignment-created collaboration compatibility state also survives through `state.json` |
| Tracked thread, authority record | Not projected into `state/get`; TUI can synthesize partial records from hierarchy summaries when detail is not loaded | No daemon event is emitted today for create, edit, or delete | Survives daemon restart through `state.db`; clients must reload authority hierarchy or detail queries |
| Assignment | Collaboration-native | `AssignmentLifecycle` | Survives daemon restart through `state.json`; clients reload via snapshot-first flow |
| Proposal | Collaboration-native; there is no top-level proposal list in `state/get`, though collaboration work unit summaries can carry nested proposal summaries | `ProposalLifecycle` | Survives daemon restart through `state.json`; clients must re-query proposal RPCs after reconnect when they need full proposal records |
| Decision | Collaboration-native | `DecisionApplied` | Survives daemon restart through `state.json`; visible again through `state/get` |
| Report | Collaboration-native | `ReportRecorded` | Survives daemon restart through `state.json`; visible again through `state/get` and report RPCs |
| Worker session | Collaboration-native internal state | No dedicated worker-session event | Survives daemon restart through `state.json`; no dedicated client reload surface exists today |
| Live thread state | Derived from Codex plus daemon mirrors; not authority state | `UpstreamStatusChanged`, `SessionChanged`, `ThreadUpdated`, `TurnUpdated`, `ItemUpdated`, `OutputDelta` as applicable | Stored mirrors reload from `state.json`, but clients still treat reconnect as snapshot-first and `turn/attach` as daemon-instance scoped |
| Live turn state | Derived from Codex plus daemon mirrors | `TurnUpdated`, `ItemUpdated`, `OutputDelta`, `SessionChanged` as applicable | Stored mirrors reload from `state.json`, but attach and stream continuity are not promised across daemon restart |
| Local PTY-backed Codex resume session state | TUI-only derived and runtime-managed; not reflected in daemon read models | No daemon event visibility | Does not survive TUI process exit or restart; daemon reconnect does not recreate it |

## IPC Contract

Orcas IPC uses a local Unix domain socket and JSON-RPC 2.0 style messages. Messages are newline-delimited JSON records. Clients issue requests for commands and queries, receive responses for results, and subscribe to notifications for incremental updates.

The daemon exposes a snapshot-first interaction pattern. Clients typically request current state first, then subscribe to live events. That keeps reconnect behavior deterministic and avoids rebuilding UI state from raw event gaps. The important caveat is that `state/get` is not the full authority hierarchy, and event coverage for authority mutations is currently incomplete.

Current request families include:

- daemon lifecycle and status:
  - `daemon/status`
  - `daemon/connect`
  - `daemon/stop`
  - `daemon/disconnect`
- snapshot and session state:
  - `state/get`
  - `session/get_active`
- models and thread views:
  - `models/list`
  - `threads/list`
  - `threads/list_scoped`
  - `threads/list_loaded`
  - `thread/start`
  - `thread/read`
  - `thread/read_history`
  - `thread/get`
  - `thread/attach`
  - `thread/detach`
  - `thread/resume`
- turn views and turn control:
  - `turns/list_active`
  - `turns/recent`
  - `turn/get`
  - `turn/attach`
  - `turn/start`
  - `turn/steer`
  - `turn/interrupt`
- workflow and authority state:
  - `workstream/create`
  - `workstream/list`
  - `workstream/get`
  - `workunit/create`
  - `workunit/list`
  - `workunit/get`
  - `authority/hierarchy/get`
  - `authority/delete/plan`
  - `authority/workstream/create`
  - `authority/workstream/edit`
  - `authority/workstream/delete`
  - `authority/workstream/list`
  - `authority/workstream/get`
  - `authority/workunit/create`
  - `authority/workunit/edit`
  - `authority/workunit/delete`
  - `authority/workunit/list`
  - `authority/workunit/get`
  - `authority/tracked_thread/create`
  - `authority/tracked_thread/edit`
  - `authority/tracked_thread/delete`
  - `authority/tracked_thread/list`
  - `authority/tracked_thread/get`
  - `assignment/start`
  - `assignment/get`
  - `report/get`
  - `report/list_for_workunit`
  - `decision/apply`
- event subscription:
  - `events/subscribe`

Notifications are delivered on `events/notification` with Orcas-owned event envelopes. The daemon keeps a recent event buffer and bounded per-client queues so one slow frontend cannot stall the broker.

## Read-Model Contract

### `state/get`

`state/get` is the daemon's merged supervision snapshot. It currently contains:

- daemon status metadata
- active session state
- thread summaries and the active thread view
- a collaboration-shaped snapshot of workstreams, work units, assignments, codex thread assignments, supervisor decisions, reports, and decisions
- recent daemon event summaries

`state/get` is not a single-store source-of-truth dump. It is assembled from daemon memory and then augmented with authority workstream and work unit projections.

`state/get` does not contain:

- tracked-thread records
- authority revisions, tombstones, or origin-node metadata
- top-level proposal records, though work unit summaries can carry nested proposal summaries for collaboration-owned work units
- worker-session records
- TUI-local PTY session state

Current limitations of the merged collaboration snapshot:

- workstream and work unit lists can contain mixed semantics
- authority workstreams appear as collaboration-shaped summaries
- authority work units projected directly from SQLite carry only limited collaboration fields
- projected authority work units currently use defaulted collaboration summary fields such as no dependency count, no current assignment id, no latest report id, and no proposal summary until a compatibility bridge has injected a collaboration record for that id
- authority deletes do not currently scrub previously bridged collaboration copies from `state.json`, so `state/get` can retain stale authority-derived workstreams or work units after the corresponding authority row has been tombstoned

### `authority/hierarchy/get`

`authority/hierarchy/get` is the daemon's authority-only hierarchy query over SQLite. It returns authority workstreams, authority work units, and tracked threads using authority-shaped records and summaries.

This read model is the current source for:

- tracked-thread hierarchy
- authority revisions
- authority tombstones when `include_deleted = true`
- authority-only metadata such as origin node identity

`authority/hierarchy/get` does not include:

- legacy collaboration-only workstreams or work units
- assignments
- proposals
- reports
- decisions
- worker sessions
- live thread or turn state

### Which Clients Rely On Which Read Models

- The CLI primarily relies on `state/get` and the legacy `workstream/*` and `workunit/*` RPC family.
- The TUI bootstraps from both `state/get` and `authority/hierarchy/get`.
- The TUI uses authority detail RPCs such as `authority/workstream/get`, `authority/workunit/get`, and `authority/tracked_thread/get` for focused editing surfaces.
- Existing subscribers should treat events as incremental hints layered on top of snapshot reloads, not as a complete replayable truth source for authority state.

### Current Client-Side Synthesis

The TUI currently synthesizes some authority-shaped records locally when detail data has not been loaded yet.

- It can derive edit-form workstream and tracked-thread records from hierarchy summaries.
- Those synthesized records do not carry the full authority detail surface.
- This is a current implementation convenience, not a guarantee that hierarchy summaries and detail records are interchangeable.

## Mutation And Event Visibility

### Authority-Owned Mutations

| Mutation | Durable write target | Read-after-write visibility | Event visibility today | What subscribers can rely on today |
| --- | --- | --- | --- | --- |
| `authority/workstream/create` | `state.db` | Appears in `authority/hierarchy/get` immediately after commit; appears in `state/get` as a projected workstream summary on the next snapshot read | Emits `WorkstreamLifecycle { action = created }` | Subscribers can observe a create hint, but the event payload is collaboration-shaped and omits authority metadata |
| `authority/workstream/edit` | `state.db` | Updated in `authority/hierarchy/get` after commit; projected changes appear in the next `state/get` snapshot | Emits `WorkstreamLifecycle { action = updated }` | Subscribers can observe an update hint, but should reload if they need authority revision or delete state |
| `authority/workstream/delete` | `state.db` tombstone | Hidden from default `authority/hierarchy/get`. In `state/get`, purely projected rows disappear on reload, but any previously bridged collaboration copy can remain because authority delete does not currently scrub legacy collaboration state. | No daemon event is emitted | Subscribers cannot rely on a live delete notification or on immediate `state/get` convergence; they must reload and treat stale bridged rows as a known limitation |
| `authority/workunit/create` | `state.db` | Appears in `authority/hierarchy/get` immediately after commit; appears in `state/get` as a projected work unit summary on the next snapshot read | Emits `WorkUnitLifecycle { action = created }` | Subscribers can observe a create hint, but the summary is collaboration-shaped and authority-specific fields are not included |
| `authority/workunit/edit` | `state.db` | Updated in `authority/hierarchy/get` after commit; projected changes appear in the next `state/get` snapshot | Emits `WorkUnitLifecycle { action = updated }` | Subscribers can observe an update hint, but should reload if they need the authority-shaped row |
| `authority/workunit/delete` | `state.db` tombstone | Hidden from default `authority/hierarchy/get`. In `state/get`, purely projected rows disappear on reload, but any previously bridged collaboration copy can remain because authority delete does not currently scrub legacy collaboration state. | No daemon event is emitted | Subscribers cannot rely on a live delete notification or on immediate `state/get` convergence; they must reload and treat stale bridged rows as a known limitation |
| `authority/tracked_thread/create` | `state.db` | Appears in `authority/hierarchy/get`, `authority/workunit/get`, and `authority/tracked_thread/get` after commit | No daemon event is emitted | Subscribers must reload authority queries; event-only tracking is insufficient |
| `authority/tracked_thread/edit` | `state.db` | Updated in authority detail and hierarchy queries after commit | No daemon event is emitted | Subscribers must reload authority queries |
| `authority/tracked_thread/delete` | `state.db` tombstone | Hidden from default authority queries after commit | No daemon event is emitted | Subscribers must reload authority queries |

### Collaboration-Owned Mutations For Contrast

Current collaboration-owned event coverage is stronger than authority event coverage, but it is still object-specific rather than universal.

- Workstream and work unit lifecycle events exist for collaboration-owned records.
- Assignments emit `AssignmentLifecycle`.
- Codex thread assignments emit `CodexAssignmentLifecycle`.
- Supervisor turn decisions emit `SupervisorDecisionLifecycle`.
- Reports emit `ReportRecorded`.
- Decisions emit `DecisionApplied`.
- Proposals emit `ProposalLifecycle`.
- Worker sessions do not have a dedicated daemon event family.

## Snapshot, Restart, And Reconnect Flow

The current client reconnect flow is:

1. Connect to the daemon socket.
2. Request `state/get`.
3. Request focused reads as needed.
4. Subscribe to `events/subscribe`.
5. After reconnect or daemon restart, rebuild from fresh reads first and only then consume incremental events.

This remains the recommended flow for both the CLI and the TUI. The TUI adds an authority reload step because its main hierarchy depends on both `state/get` and `authority/hierarchy/get`.

### Persistence Notes

- `state.json` remains live. The daemon loads collaboration state and thread/turn mirrors from it on startup and writes collaboration changes back to it.
- `state.db` remains live. The authority SQLite store persists authority workstreams, authority work units, tracked threads, revisions, tombstones, command receipts, and authority event history.
- On first authority-store initialization, SQLite can bootstrap from existing `state.json` if authority data has not already been recorded in `state.db`.

### What Survives Daemon Restart

- collaboration state in `state.json`
- thread and turn mirror state in `state.json`
- authority state in `state.db`

### What Clients Must Reload

- `state/get` after reconnect
- `authority/hierarchy/get` for TUI hierarchy views after reconnect
- authority detail queries when exact authority fields are needed
- proposal and other focused RPCs for data that is not included in `state/get`

### TUI PTY Exception

The TUI-local PTY-backed `codex resume` session manager is not part of daemon state. It does not live in `state/get` or `authority/hierarchy/get`, and it is not reconstructed by daemon reconnect.

## Upstream Codex Integration

`orcasd` owns the upstream Codex app-server connection. Clients do not use the upstream WebSocket protocol directly for supervised-state reads or writes.

The daemon connects to a configured WebSocket endpoint, with a localhost endpoint used by default in the current configuration. The upstream transport details remain an internal implementation concern. Orcas surfaces the resulting thread, turn, collaboration, and authority query state through its own IPC contract instead of mirroring the upstream wire format wholesale.

The one intentional exception is the TUI's local PTY-backed `codex resume` helper. That path is a local interactive attachment convenience rather than a daemon-owned source of supervision truth.

## Operator And Client Surfaces

- The CLI currently uses the legacy `workstream/*` and `workunit/*` workflow CRUD surface.
- The TUI currently uses the authority CRUD and hierarchy surface for workstreams, work units, and tracked threads.
- Both clients still depend on daemon snapshots and focused daemon RPCs for thread, turn, assignment, report, decision, and proposal views.
- The daemon event stream is shared, but it is not yet a complete authority mutation feed.
- The TUI's PTY-backed `codex resume` path is local to the TUI process and should be understood as an operator convenience layer rather than a daemon-managed session model.

## Known Limitations Carried Into Later Phases

- `state/get` contains mixed-semantics workstream and work unit lists.
- Authority delete visibility is currently reload-based rather than event-complete.
- Authority deletes do not currently guarantee cleanup of previously bridged collaboration copies in `state/get`.
- Tracked-thread mutations currently require authority query reloads because there is no tracked-thread daemon event.
- The CLI and TUI currently operate different CRUD surfaces for workflow hierarchy objects.
- The TUI still synthesizes some authority-shaped records locally when detail is missing.

These are current implementation truths, not guarantees that later hardening phases will preserve unchanged. Later phases can normalize the boundary, but this document intentionally describes the boundary as it exists today.
